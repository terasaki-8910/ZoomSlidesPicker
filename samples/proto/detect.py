"""Change detector: pure logic over (time, analysis grid) samples.

run() takes a sample(t) callable. The same logic can drive a live capture or a
fixture replay; only the sample source differs.

Choices that map to decision-sheet questions:
  mask / video   R1-4  how a playing video region is handled
  build          R1-5  how builds on one slide are saved (final / each / first)
  on_return      R1-6  whether returning to an earlier slide saves again

Change is a fraction of compared cells, not a mean difference: one bullet added
to a sparse slide moves the mean by about 0.3 of 255 but changes roughly 0.5% of
cells. Fractions match how a viewer describes a change.
"""

import io
import time
from collections import deque
from dataclasses import dataclass

import numpy as np
from PIL import Image

GRID_SHAPE = (90, 160)  # analysis grid (H, W): 1280x720 downscaled 8x, grayscale, uint8
GRID_BYTES = GRID_SHAPE[0] * GRID_SHAPE[1]
NO_MASK = np.zeros(GRID_SHAPE, bool)


def to_grid(img):
    small = img.convert("L").resize((GRID_SHAPE[1], GRID_SHAPE[0]), Image.Resampling.BILINEAR)
    return np.asarray(small, dtype=np.uint8)


@dataclass(frozen=True)
class Config:
    name: str
    interval_s: float = 0.5       # sampling period
    settle_s: float = 1.0         # quiet time before a change counts as a settled state
    mask: str = "motion"          # "none" | "motion": exclude moving regions from comparison
    video: str = "sticky"         # "sticky" | "adaptive" | "periodic"; only with mask="motion"
    periodic_s: float = 2.0       # video="periodic": forced save interval while something moves
    build: str = "final"          # "final" | "each" | "first"
    final_quiet_s: float = 5.0    # build="final": additions this close together merge into one save
    on_return: str = "skip"       # "skip" | "save": a frame equal to an earlier saved state
    cell_thresh: float = 12.0     # per-cell |diff| (0-255) that counts as a changed cell
    change_thresh: float = 0.002  # fraction of compared cells changed => a real change (0.2%)
    removed_thresh: float = 0.001  # fraction of cells that lost content => a different slide
    motion_window: int = 6        # samples
    motion_ratio: float = 0.5     # a cell is "moving" when active in more than this share of the window
    mask_pad: int = 1             # grow moving regions by this many cells (antialiased edges)


def _changed_frac(a, b, keep, cell_thresh):
    if not keep.any():
        return 0.0
    d = np.abs(a[keep].astype(np.int16) - b[keep].astype(np.int16))
    return float((d > cell_thresh).mean())


def _removed_frac(old, new, keep, cell_thresh):
    """Fraction of compared cells that got lighter (ink removed). Added ink does not count."""
    if not keep.any():
        return 0.0
    d = new[keep].astype(np.int16) - old[keep].astype(np.int16)
    return float((d > cell_thresh).mean())


def _dilate(mask, r):
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
    def __init__(self, cfg: Config):
        self.cfg = cfg
        self.prev = None
        self.last_saved = None
        self.last_save_t = -1e9
        self.last_change_t = 0.0
        self.history = []          # (t_content, grid) of saved states, for revisit detection
        self.recent = deque(maxlen=cfg.motion_window)
        self.sticky = np.zeros(GRID_SHAPE, bool)
        self.pending = None        # (t_content, grid): build="final" candidate not yet saved
        self.events = []           # dicts: t_content, t_save, reason
        self.revisits = []
        self.detect_s = 0.0

    def _mask_for(self, moving):
        cfg = self.cfg
        if cfg.mask != "motion":
            return NO_MASK
        if cfg.video == "sticky":
            self.sticky |= moving
            return self.sticky
        if cfg.video == "adaptive":
            return moving
        return NO_MASK  # periodic: nothing masked, motion only forces periodic saves

    def _commit(self, t_save, t_content, grid, reason, keep):
        """Save a state, unless it equals an earlier saved state (then it is a revisit)."""
        cfg = self.cfg
        if cfg.on_return == "skip":
            for _, hg in reversed(self.history):
                if _changed_frac(grid, hg, keep, cfg.cell_thresh) <= cfg.change_thresh:
                    self.revisits.append(t_save)
                    self.last_saved = hg
                    self.sticky[:] = False
                    return
        self.events.append({"t_content": t_content, "t_save": t_save, "reason": reason})
        self.last_saved = grid
        self.last_save_t = t_save
        self.history.append((t_content, grid))
        self.sticky[:] = False

    def step(self, t, g):
        cfg = self.cfg
        t0 = time.perf_counter()
        prev = self.prev if self.prev is not None else g

        active = np.abs(g.astype(np.int16) - prev.astype(np.int16)) > cfg.cell_thresh
        self.recent.append(active)
        moving = None
        if cfg.mask == "motion":
            moving = _dilate(np.mean(np.stack(self.recent), axis=0) > cfg.motion_ratio, cfg.mask_pad)
        else:
            moving = NO_MASK
        keep = ~self._mask_for(moving)

        if _changed_frac(g, prev, keep, cfg.cell_thresh) > cfg.change_thresh:
            self.last_change_t = t
        quiet_for = t - self.last_change_t
        changed = self.last_saved is None or (
            _changed_frac(g, self.last_saved, keep, cfg.cell_thresh) > cfg.change_thresh
        )

        if cfg.mask == "motion" and cfg.video == "periodic" and moving.any() and changed \
                and t - self.last_save_t >= cfg.periodic_s:
            self._commit(t, t, g, "periodic", keep)
        elif changed and quiet_for >= cfg.settle_s:
            if cfg.build == "final":
                if self.pending is None:
                    self.pending = (t, g)
                elif _removed_frac(self.pending[1], g, keep, cfg.cell_thresh) >= cfg.removed_thresh:
                    self._commit(t, self.pending[0], self.pending[1], "settled", keep)  # other slide
                    self.pending = (t, g)
                elif _changed_frac(self.pending[1], g, keep, cfg.cell_thresh) > cfg.change_thresh:
                    self.pending = (t, g)  # addition on the same slide: keep the latest state
                # else: only masked content moved (video, webcam): keep the candidate
            elif cfg.build == "each":
                self._commit(t, t, g, "settled", keep)
            else:  # "first"
                if self.last_saved is None or \
                        _removed_frac(self.last_saved, g, keep, cfg.cell_thresh) >= cfg.removed_thresh:
                    self._commit(t, t, g, "settled", keep)
                # else: an addition on a slide already captured
        if cfg.build == "final" and self.pending is not None and quiet_for >= cfg.final_quiet_s:
            self._commit(t, self.pending[0], self.pending[1], "settled", keep)
            self.pending = None

        self.prev = g
        self.detect_s += time.perf_counter() - t0

    def finish(self, t):
        """Flush a pending final-build state when the replay ends."""
        if self.pending is not None:
            self._commit(t, self.pending[0], self.pending[1], "end", ~NO_MASK)
            self.pending = None


def run(cfg: Config, sample, t_end: float) -> dict:
    """Replay sample(t) -> grid from 0 to t_end at cfg.interval_s."""
    det = Detector(cfg)
    times = np.arange(0.0, t_end + 1e-9, cfg.interval_s)
    for t in times:
        det.step(float(t), sample(float(t)))
    det.finish(float(times[-1]))
    live_buffers = 3 + (cfg.motion_window + 1 if cfg.mask == "motion" else 0)
    return {
        "events": det.events,
        "revisits": det.revisits,
        "detect_ms_total": det.detect_s * 1000.0,
        "samples": len(times),
        "live_buffer_bytes": live_buffers * GRID_BYTES,
        "history_bytes": len(det.history) * GRID_BYTES,
    }


def png_bytes(img: Image.Image) -> int:
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getbuffer().nbytes
