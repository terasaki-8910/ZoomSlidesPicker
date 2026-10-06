"""Write the fixture as an mp4 (libx264) for manual playback and the mp4 extraction test.

Frames are rendered from fixture.render_frame at 2 fps, then encoded with ffmpeg.
Output: samples/fixture/out/fixture_2fps.mp4 (gitignored).

Usage: .venv/bin/python samples/fixture/export_mp4.py
"""

import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import fixture  # noqa: E402

FPS = 2
OUT = HERE / "out" / "fixture_2fps.mp4"


def main():
    OUT.parent.mkdir(parents=True, exist_ok=True)
    n = int(fixture.T["end"] * FPS)
    with tempfile.TemporaryDirectory() as tmp:
        for i in range(n):
            fixture.render_frame(i / FPS).save(Path(tmp) / f"f{i:05d}.png")
        subprocess.run(
            [
                "ffmpeg", "-y", "-loglevel", "error",
                "-framerate", str(FPS), "-i", str(Path(tmp) / "f%05d.png"),
                "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18",
                str(OUT),
            ],
            check=True,
        )
    print("wrote", OUT, f"{n} frames at {FPS} fps")


if __name__ == "__main__":
    main()
