"""Render one saved slide state three ways for decision R1-13 (moving regions in saved images).

  as-is: the screen as it was, webcam tile included
  grey:  the moving region filled with mid grey
  crop:  the largest rectangle that avoids the moving region (left column)

The state is slide 5 at 70.0 s: bullets A-B on screen, webcam running.
Output: samples/proto/out/r1-13-{as-is,grey,crop}.png, each 540 px tall.

Usage: .venv/bin/python samples/proto/make_r13_samples.py
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
import fixture  # noqa: E402

T = 70.0
OUT = HERE / "out"
HEIGHT = 540


def scaled(img):
    return img.resize((round(img.width * HEIGHT / img.height), HEIGHT), Image.Resampling.LANCZOS)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    full = fixture.render_frame(T)
    x0, y0, x1, y1 = fixture.CAM_BOX

    grey = full.copy()
    ImageDraw.Draw(grey).rectangle(fixture.CAM_BOX, fill=(136, 136, 136))

    crop = full.crop((0, 0, x0, fixture.H))  # left column, clear of the tile

    for name, img in [("as-is", full), ("grey", grey), ("crop", crop)]:
        path = OUT / f"r1-13-{name}.png"
        scaled(img).save(path, format="PNG")
        print("wrote", path)


if __name__ == "__main__":
    main()
