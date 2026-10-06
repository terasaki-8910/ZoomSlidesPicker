"""Replay F1 from its mp4 (2 fps, libx264) through the detector and compare with the expectations.

This is the path of the mp4 mode (R1-9): frames come from ffmpeg instead of the screen.
Requires ffmpeg on PATH and samples/fixture/out/fixture_2fps.mp4 (export_mp4.py).

Usage: .venv/bin/python samples/proto/mp4_check.py
"""

import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
sys.path.insert(0, str(HERE))

import detect  # noqa: E402
import fixture  # noqa: E402

MP4 = HERE.parent / "fixture" / "out" / "fixture_2fps.mp4"
FPS = 2


def main():
    if not MP4.exists():
        print(f"missing {MP4}; run export_mp4.py first")
        sys.exit(2)
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run(
            ["ffmpeg", "-loglevel", "error", "-i", str(MP4), "-vsync", "0", str(Path(tmp) / "f%05d.png")],
            check=True,
        )
        frames = sorted(Path(tmp).glob("f*.png"))
        cache = {}

        def sample(t):
            i = min(int(round(t * FPS)), len(frames) - 1)
            if i not in cache:
                cache[i] = detect.sample_of(Image.open(frames[i]).convert("RGB"))
            return cache[i]

        spec = fixture.FIXTURES["F1"]
        det = detect.run(sample, spec["end"])

    times = [round(s["t_content"], 2) for s in det.saves]
    print(f"decoded {len(frames)} frames; saves {times}; revisits {det.revisits}; blanks {det.blanks}")
    ok = len(det.saves) == len(spec["saves"]) and all(
        sum(1 for s in det.saves if a <= s["t_content"] <= b) == 1 for _, a, b in spec["saves"]
    ) and len(det.revisits) == spec["revisits"]
    print("PASS mp4 replay matches F1 expectations" if ok else "FAIL mp4 replay differs from F1 expectations")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
