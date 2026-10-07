"""Converts a record_grids.py recording into the fixture layout, so the detector can replay
it (`cargo run --release -p zsp-detect --example trace_fixture -- <out dir>`).

The recording keeps no full frames, so detail (edge strength, used only to pick the
sharpest frame of a state) is approximated by the Laplacian of the grid itself. Write the
output to a scratch directory; it is lecture content, never commit it.

    .venv/bin/python samples/verify/grids_to_fixture.py <grids.npz> <out dir>
"""

import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

DETAIL_SCALE = 64


def main():
    src, out = sys.argv[1], Path(sys.argv[2])
    data = np.load(src)
    times, grids = data["times"], data["grids"]
    n, gh, gw = grids.shape
    g = grids.astype(np.float32)
    lap = np.zeros_like(g)
    lap[:, 1:-1, 1:-1] = np.abs(4 * g[:, 1:-1, 1:-1] - g[:, :-2, 1:-1] - g[:, 2:, 1:-1]
                                - g[:, 1:-1, :-2] - g[:, 1:-1, 2:])
    detail = np.clip(np.round(lap * DETAIL_SCALE), 0, 65535).astype(np.uint16)
    out.mkdir(parents=True, exist_ok=True)
    Image.fromarray(grids.reshape(n * gh, gw)).save(out / "grid.png")
    Image.fromarray(detail.reshape(n * gh, gw)).save(out / "detail.png")
    t0 = float(times[0])
    (out / "times.json").write_text(json.dumps({
        "interval_s": float(np.median(np.diff(times))), "grid_w": gw, "grid_h": gh,
        "detail_scale": DETAIL_SCALE, "t0_epoch": t0,
        "times": [round(float(t) - t0, 3) for t in times]}))
    print(f"{n} samples {gw}x{gh} -> {out}")


if __name__ == "__main__":
    main()
