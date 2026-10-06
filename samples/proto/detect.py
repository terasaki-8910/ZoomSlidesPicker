"""Change detector, round 2: the behaviour chosen in DECISIONS.md, round 1.

Pure logic over samples. Each sample is (t, grid, detail):
  grid    analysis image: the capture region scaled to 160 px wide, grayscale, uint8
  detail  per-cell edge strength (mean |Laplacian| of the full-size frame), float32,
          used only to pick the sharpest frame of a state
The capture layer turns frames into samples; this module never sees full-size images.
It returns which sample's full-size frame to save, so the caller keeps at most the
frame of the pending state (one full-size frame).

Behaviour (DECISIONS.md):
  R1-5 b  moving regions are excluded while they move and compared again once still
  R1-6 a  additions to the same slide merge into the final state
  R1-7 a  a state equal to an already-saved one is recorded as a revisit, not saved
Round 2 additions:
  - addition vs removal is judged against the slide's background level, so builds on
    dark backgrounds merge the same way as on light ones
  - single-colour frames (black, grey, white) are never saved; a stretch of 10 s or
    more is reported once
  - while a state is pending, the sharpest frame of that state is kept for saving
    (a shared screen often arrives blurred and sharpens a moment later)
"""

import time
from collections import deque
from dataclasses import dataclass

import numpy as np
from PIL import Image

GRID_W = 160


def grid_shape(width, height):
    gh = int(round(GRID_W * height / width))
    return max(30, min(240, gh)), GRID_W


def to_grid(img):
    gh, gw = grid_shape(*img.size)
    small = img.convert("L").resize((gw, gh), Image.Resampling.BILINEAR)
    return np.asarray(small, dtype=np.uint8)


def to_detail(img):
    """Mean |Laplacian| per grid cell, from the full-size frame."""
    g = np.asarray(img.convert("L"), dtype=np.float32)
    lap = np.zeros_like(g)
    lap[1:-1, 1:-1] = np.abs(
        4 * g[1:-1, 1:-1] - g[:-2, 1:-1] - g[2:, 1:-1] - g[1:-1, :-2] - g[1:-1, 2:]
    )
    gh, gw = grid_shape(img.width, img.height)
    ys = np.linspace(0, g.shape[0], gh + 1).astype(int)
    xs = np.linspace(0, g.shape[1], gw + 1).astype(int)
    # Block means via cumulative sums (cells may differ in size by one pixel).
    c = np.pad(lap.cumsum(0).cumsum(1), ((1, 0), (1, 0)))
    s = c[ys[1:]][:, xs[1:]] - c[ys[:-1]][:, xs[1:]] - c[ys[1:]][:, xs[:-1]] + c[ys[:-1]][:, xs[:-1]]
    area = np.outer(np.diff(ys), np.diff(xs))
    return (s / area).astype(np.float32)


def sample_of(img):
    return to_grid(img), to_detail(img)


@dataclass(frozen=True)
class Config:
    interval_s: float = 0.5        # comparison period (N); settable 0.5-5 s
    settle_s: float = 1.0          # quiet time before a state counts as settled
    final_quiet_s: float = 5.0     # additions closer together than this merge into one save
    cell_thresh: float = 12.0      # per-cell |diff| (0-255) that counts as a changed cell
    change_thresh: float = 0.002   # share of compared cells changed => a real change (0.2%)
    removed_thresh: float = 0.001  # share of cells that went back toward the background => new slide
    motion_window: int = 6         # samples
    motion_ratio: float = 0.5      # a cell is moving when active in more than this share of the window
    mask_pad: int = 1              # grow moving regions by this many cells
    uniform_std: float = 2.0       # grid std below this => single-colour frame
    blank_report_s: float = 10.0   # report a single-colour stretch once it lasts this long
    sharper_ratio: float = 1.10    # replace the held frame only when this much sharper
    refine_ratio: float = 1.20     # a change counts as the same slide sharpening only when this much sharper


def changed_frac(a, b, keep, cell_thresh):
    if not keep.any():
        return 0.0
    d = np.abs(a[keep].astype(np.int16) - b[keep].astype(np.int16))
    return float((d > cell_thresh).mean())


def removed_frac(old, new, keep, cell_thresh):
    """Share of compared cells that changed and moved toward the old state's background level."""
    if not keep.any():
        return 0.0
    o = old[keep].astype(np.int16)
    n = new[keep].astype(np.int16)
    bg = int(np.median(o))
    changed = np.abs(n - o) > cell_thresh
    toward_bg = np.abs(n - bg) < np.abs(o - bg)
    return float((changed & toward_bg).mean())


def coarse_removed_frac(old, new, keep, cell_thresh, f=2):
    """removed_frac on f x f blocks of cells. Blur halos average out at this scale; removed text does not."""
    h, w = old.shape
    h2, w2 = h // f, w // f
    o = old[:h2 * f, :w2 * f].astype(np.float32).reshape(h2, f, w2, f).mean(axis=(1, 3))
    n = new[:h2 * f, :w2 * f].astype(np.float32).reshape(h2, f, w2, f).mean(axis=(1, 3))
    k = keep[:h2 * f, :w2 * f].reshape(h2, f, w2, f).all(axis=(1, 3))
    if not k.any():
        return 0.0
    bg = float(np.median(o[k]))
    changed = np.abs(n[k] - o[k]) > cell_thresh
    toward_bg = np.abs(n[k] - bg) < np.abs(o[k] - bg)
    return float((changed & toward_bg).mean())


def dilate(mask, r):
    if r <= 0:
        return mask
    p = np.pad(mask, r, constant_values=False)
    h, w = mask.shape
    out = np.zeros_like(mask)
    for dy in range(2 * r + 1):
        for dx in range(2 * r + 1):
            out |= p[dy:dy + h, dx:dx + w]
    return out


class Detector:
    def __init__(self, cfg: Config = Config()):
        self.cfg = cfg
        self.prev = None
        self.recent = deque(maxlen=cfg.motion_window)
        self.last_change_t = 0.0
        self.last_saved = None
        self.history = []          # (t_content, grid) of saved states
        self.pending = None        # dict: t (content), grid, held_t, held_sharp
        self.uniform_since = None
        self.blank_reported = False
        self.saves = []            # dicts: t_content, t_save, held_t, reason
        self.revisits = []         # content times of states equal to an earlier save
        self.blanks = []           # times a single-colour stretch was reported
        self.detect_s = 0.0

    def _commit(self, t_save, keep, reason):
        p = self.pending
        self.pending = None
        cfg = self.cfg
        for _, hg in reversed(self.history):
            if changed_frac(p["grid"], hg, keep, cfg.cell_thresh) <= cfg.change_thresh:
                self.revisits.append(p["t"])
                self.last_saved = hg
                return
        self.saves.append({"t_content": p["t"], "t_save": t_save, "held_t": p["held_t"], "reason": reason})
        self.last_saved = p["grid"]
        self.history.append((p["t"], p["grid"]))

    def _start_pending(self, t, g, sharp):
        self.pending = {"t": t, "grid": g, "held_t": t, "held_sharp": sharp}

    def step(self, t, g, detail):
        cfg = self.cfg
        t0 = time.perf_counter()
        prev = self.prev if self.prev is not None else g

        self.recent.append(np.abs(g.astype(np.int16) - prev.astype(np.int16)) > cfg.cell_thresh)
        moving = dilate(np.mean(np.stack(self.recent), axis=0) > cfg.motion_ratio, cfg.mask_pad)
        keep = ~moving  # R1-5 b: masked only while moving

        if changed_frac(g, prev, keep, cfg.cell_thresh) > cfg.change_thresh:
            self.last_change_t = t
        quiet_for = t - self.last_change_t

        uniform = float(g.std()) < cfg.uniform_std
        if uniform:
            if self.uniform_since is None:
                self.uniform_since, self.blank_reported = t, False
            elif not self.blank_reported and t - self.uniform_since >= cfg.blank_report_s:
                self.blanks.append(t)
                self.blank_reported = True
        else:
            self.uniform_since = None

        sharp = float(detail[keep].mean()) if keep.any() else 0.0

        if not uniform and quiet_for >= cfg.settle_s:
            p = self.pending
            ref = p["grid"] if p is not None else self.last_saved
            differs = ref is None or changed_frac(g, ref, keep, cfg.cell_thresh) > cfg.change_thresh
            if p is None:
                if differs:
                    self._start_pending(t, g, sharp)
            elif differs:
                sharpened = (
                    sharp >= p["held_sharp"] * cfg.refine_ratio
                    and coarse_removed_frac(p["grid"], g, keep, cfg.cell_thresh) < cfg.removed_thresh
                )
                if sharpened:
                    # Same slide arriving sharper: replace it but keep the time it first settled.
                    p["grid"], p["held_t"], p["held_sharp"] = g, t, sharp
                elif removed_frac(p["grid"], g, keep, cfg.cell_thresh) >= cfg.removed_thresh:
                    self._commit(t, keep, "settled")   # content went away: a different slide
                    if self.last_saved is None or \
                            changed_frac(g, self.last_saved, keep, cfg.cell_thresh) > cfg.change_thresh:
                        self._start_pending(t, g, sharp)
                else:
                    self._start_pending(t, g, sharp)   # addition on the same slide: keep the latest
            elif sharp > p["held_sharp"] * cfg.sharper_ratio:
                p["held_t"], p["held_sharp"] = t, sharp  # same state, clearly sharper frame

        if self.pending is not None and quiet_for >= cfg.final_quiet_s:
            self._commit(t, keep, "settled")

        self.prev = g
        self.detect_s += time.perf_counter() - t0

    def finish(self, t):
        if self.pending is not None:
            self._commit(t, np.ones_like(self.pending["grid"], dtype=bool), "end")


def run(sample, t_end, cfg: Config = Config()):
    """Replay sample(t) -> (grid, detail) from 0 to t_end every cfg.interval_s."""
    det = Detector(cfg)
    times = np.arange(0.0, t_end + 1e-9, cfg.interval_s)
    for t in times:
        g, detail = sample(float(t))
        det.step(float(t), g, detail)
    det.finish(float(times[-1]))
    return det
