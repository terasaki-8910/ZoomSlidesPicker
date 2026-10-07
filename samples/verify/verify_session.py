"""Checks a session's saves against what the window showed (record_grids.py output).

Ground truth from the recording:
- Live cells: cells that change in at least 4 of some 10 consecutive samples (a playing
  video, a camera tile, a player's clock); slide text changes far less often. They are
  left out of every comparison below, as the detector does.
- States: stretches with no change outside live cells, merged when their content is the
  same. A state shown for at least `--min-state` seconds in one stretch should be saved,
  unless the next state only adds to it within 5 s (a build, merged by design).

Each saved image is matched to the state it shows (the 160-wide grid with a looser cell
threshold, so a 1 px difference in cropping does not count).
Reported: states with no save (misses), states with more than one save (duplicates), and
saves matching no state (transitional or overlay frames).

    .venv/bin/python samples/verify/verify_session.py <session dir> <grids.npz>
"""

import argparse
import json
import os
from datetime import datetime

import numpy as np
from PIL import Image

CELL = 12          # detector cell threshold
CHANGE = 0.002     # detector change threshold
MATCH_T = 30       # matching a saved image to a state: cell threshold
MATCH_SHARE = 0.003
LIVE_WINDOW = 10   # samples
LIVE_HITS = 4


def dilate(m: np.ndarray, r: int = 1) -> np.ndarray:
    out = m.copy()
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            out |= np.roll(np.roll(m, dy, 0), dx, 1)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("session_dir")
    ap.add_argument("grids")
    ap.add_argument("--min-state", type=float, default=6.0)
    a = ap.parse_args()

    data = np.load(a.grids)
    times, grids = data["times"], data["grids"].astype(np.int16)
    n = len(times)
    diffs = np.abs(np.diff(grids, axis=0)) > CELL            # (n-1, h, w)
    share = diffs.reshape(n - 1, -1).mean(axis=1)

    # Live cells: changed in at least LIVE_HITS of some LIVE_WINDOW consecutive samples.
    c = np.cumsum(np.concatenate([np.zeros((1,) + diffs.shape[1:], int), diffs.astype(int)]), axis=0)
    w = min(LIVE_WINDOW, n - 1)
    rolling = c[w:] - c[:-w]
    live = dilate(rolling.max(axis=0) >= LIVE_HITS, 1)
    keep = ~live

    # States: break where the content outside live cells changes.
    changed = (diffs & keep).reshape(n - 1, -1).sum(axis=1) / keep.sum() > CHANGE
    states = []  # [start_idx, end_idx]
    s = 0
    for i, c in enumerate(changed):
        if c:
            states.append([s, i])
            s = i + 1
    states.append([s, n - 1])
    reps = [np.median(grids[a_:b + 1], axis=0).astype(np.int16) for a_, b in states]

    def same(x, y):
        d = (np.abs(x - y) > MATCH_T) & keep
        return d.sum() / keep.sum() <= MATCH_SHARE

    # Merge stretches with the same content into states (a revisit is the same state).
    state_of = []
    distinct = []  # representative grids
    for r in reps:
        for k, d in enumerate(distinct):
            if same(r, d):
                state_of.append(k)
                break
        else:
            distinct.append(r)
            state_of.append(len(distinct) - 1)

    session = json.load(open(os.path.join(a.session_dir, "session.json")))
    caps = session["captures"]
    cap_state = []
    for c in caps:
        img = Image.open(os.path.join(a.session_dir, c["file"])).convert("L")
        gh = max(30, min(240, round(160 * img.height / img.width)))
        g = np.asarray(img.resize((160, gh), Image.BILINEAR), dtype=np.int16)
        if g.shape != grids.shape[1:]:
            g = np.asarray(img.resize(grids.shape[1:][::-1], Image.BILINEAR), dtype=np.int16)
        match = next((k for k, d in enumerate(distinct) if same(g, d)), None)
        cap_state.append(match)

    def span(k):
        return [(times[a_], times[b]) for (a_, b), sk in zip(states, state_of) if sk == k]

    def clock(t):
        return datetime.fromtimestamp(t).strftime("%H:%M:%S")

    longest = {k: max(e - b for b, e in span(k)) for k in range(len(distinct))}
    expected = [k for k in range(len(distinct)) if longest[k] >= a.min_state]
    saved = {}
    for i, k in enumerate(cap_state):
        if k is not None:
            saved.setdefault(k, []).append(caps[i]["file"])

    misses = [k for k in expected if k not in saved]
    dups = {k: v for k, v in saved.items() if len(v) > 1}
    stray = [caps[i]["file"] for i, k in enumerate(cap_state) if k is None]

    print(f"recording: {n} samples {clock(times[0])}-{clock(times[-1])}, live cells {live.sum()}")
    print(f"states: {len(distinct)} distinct, {len(expected)} shown >= {a.min_state:.0f} s; saves: {len(caps)}")
    print(f"matched states: {len([k for k in expected if k in saved])}/{len(expected)}")
    for k in misses:
        print(f"MISS state {k}: " + ", ".join(f"{clock(b)}-{clock(e)}" for b, e in span(k)))
    for k, v in dups.items():
        print(f"DUP  state {k}: {', '.join(v)}")
    for f in stray:
        print(f"STRAY save {f} (matches no stable state)")


if __name__ == "__main__":
    main()
