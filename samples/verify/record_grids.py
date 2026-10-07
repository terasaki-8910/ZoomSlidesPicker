"""Records what a running session's window shows, as small analysis grids, for checking
the saves afterwards (verify_session.py). Not committed data: run it locally only.

Every `--every` seconds: screencapture of the window (by CGWindowID), cropped to the
session's region (read from its session.json), reduced to a 160-wide grayscale grid like
the detector's, kept with the wall-clock time. The full frame is deleted right away, so
only grids are stored (about 14 KB a sample). Stops when the session's session.json has
`ended_at`, or after `--max` seconds.

    .venv/bin/python samples/verify/record_grids.py <window id> <session dir> <out.npz>
"""

import argparse
import json
import os
import subprocess
import tempfile
import time
from datetime import datetime

import numpy as np
from PIL import Image

GRID_W = 160


def grid_of(img: Image.Image, region) -> np.ndarray:
    w, h = img.size
    x, y, rw, rh = region
    box = (round(x * w), round(y * h), round((x + rw) * w), round((y + rh) * h))
    crop = img.crop(box).convert("L")
    gh = max(30, min(240, round(GRID_W * crop.height / crop.width)))
    return np.asarray(crop.resize((GRID_W, gh), Image.BILINEAR), dtype=np.uint8)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("window_id")
    ap.add_argument("session_dir")
    ap.add_argument("out")
    ap.add_argument("--every", type=float, default=1.0)
    ap.add_argument("--max", type=float, default=4 * 3600)
    a = ap.parse_args()

    session_json = os.path.join(a.session_dir, "session.json")
    region = json.load(open(session_json))["region"]
    region = (region["x"], region["y"], region["w"], region["h"])

    times, grids = [], []
    start = time.monotonic()
    tmp = tempfile.mkdtemp(prefix="zsp-rec-")
    frame = os.path.join(tmp, "f.png")
    i = 0
    try:
        while time.monotonic() - start < a.max:
            due = start + i * a.every
            if due > time.monotonic():
                time.sleep(due - time.monotonic())
            i += 1
            wall = time.time()
            r = subprocess.run(["screencapture", "-l", a.window_id, "-o", "-x", "-t", "png", frame])
            if r.returncode != 0 or not os.path.exists(frame):
                continue
            try:
                g = grid_of(Image.open(frame), region)
            finally:
                os.remove(frame)
            times.append(wall)
            grids.append(g)
            if i % 30 == 0:
                np.savez_compressed(a.out, times=np.array(times), grids=np.stack(grids))
                if json.load(open(session_json)).get("ended_at"):
                    break
    finally:
        if grids:
            np.savez_compressed(a.out, times=np.array(times), grids=np.stack(grids))
        os.rmdir(tmp)
    print(f"{len(grids)} samples, {datetime.fromtimestamp(times[0]):%H:%M:%S}"
          f"-{datetime.fromtimestamp(times[-1]):%H:%M:%S}" if grids else "no samples")


if __name__ == "__main__":
    main()
