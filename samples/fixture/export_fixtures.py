"""Export the fixtures in the form the Rust tests read (committed under fixtures/).

For each fixture F1-F7 (samples every 0.5 s, the default comparison period):
  fixtures/<id>/grid.png     8-bit grayscale strip: one analysis grid per sample, stacked
  fixtures/<id>/detail.png   16-bit grayscale strip: detail x 64, same layout
  fixtures/<id>/times.json   {"interval_s", "grid_w", "grid_h", "times": [...]}
fixtures/expected.json      expectations per fixture (from fixture.FIXTURES)
fixtures/frames/<name>.png  full-size frames for the analysis equivalence test (ACCEPTANCE B1),
                            with the reference implementation's grid and detail next to them
fixtures/f1_2fps.mp4        F1 as H.264 at 2 fps (ACCEPTANCE B8)

Usage: .venv/bin/python samples/fixture/export_fixtures.py [F7 ...]
(with ids, only those strips and their expectations are rewritten)
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "proto"))

import detect  # noqa: E402
import fixture  # noqa: E402

OUT = ROOT / "fixtures"
INTERVAL = 0.5
DETAIL_SCALE = 64


def save_u8(arr, path):
    Image.fromarray(np.asarray(arr, dtype=np.uint8), "L").save(path, format="PNG", optimize=True)


def save_u16(arr, path):
    a = np.clip(np.round(np.asarray(arr, dtype=np.float64) * DETAIL_SCALE), 0, 65535).astype(np.uint16)
    Image.fromarray(a).save(path, format="PNG")


def export_strips(only=None):
    """All fixtures, or only the ids in `only` (the others' files and expectations are kept)."""
    path = OUT / "expected.json"
    expected = json.loads(path.read_text()) if only and path.exists() else {}
    for fid, spec in fixture.FIXTURES.items():
        if only and fid not in only:
            continue
        times = [round(float(t), 3) for t in np.arange(0.0, spec["end"] + 1e-9, INTERVAL)]
        grids, details = [], []
        for t in times:
            g, d = detect.sample_of(spec["render"](t))
            grids.append(g)
            details.append(d)
        gh, gw = grids[0].shape
        out = OUT / fid
        out.mkdir(parents=True, exist_ok=True)
        save_u8(np.vstack(grids), out / "grid.png")
        save_u16(np.vstack(details), out / "detail.png")
        (out / "times.json").write_text(json.dumps(
            {"interval_s": INTERVAL, "grid_w": gw, "grid_h": gh, "detail_scale": DETAIL_SCALE, "times": times},
            indent=1,
        ))
        expected[fid] = {
            "label": spec["label"],
            "end": spec["end"],
            "saves": [{"label": lb, "from": a, "to": b} for lb, a, b in spec["saves"]],
            "revisits": spec["revisits"],
            "blanks": spec["blanks"],
            "quiet": [{"label": lb, "from": a, "to": b} for lb, a, b in spec["quiet"]],
        }
        if spec.get("replaces"):
            expected[fid]["replaces"] = spec["replaces"]
        if "sharp_from" in spec:
            expected[fid]["sharp_from"] = spec["sharp_from"]
        print(f"{fid}: {len(times)} samples, grid {gw}x{gh}")
    expected = {fid: expected[fid] for fid in fixture.FIXTURES if fid in expected}
    (OUT / "expected.json").write_text(json.dumps(expected, ensure_ascii=False, indent=1))


def export_frames():
    F = fixture.FIXTURES
    frames = {
        "f1_slide1": F["F1"]["render"](1.0),
        "f1_video": F["F1"]["render"](30.0),
        "f1_webcam": F["F1"]["render"](70.0),
        "f3_lowres": F["F3"]["render"](21.0),
        "f5_cursor": F["F5"]["render"](5.0),
        "f6_dark": F["F6"]["render"](7.0),
        # Non-16:9 crops: grid height follows the aspect ratio, cells are not whole pixels.
        "crop_4x3": F["F1"]["render"](15.5).crop((100, 0, 1060, 720)),
        "crop_odd": F["F3"]["render"](25.0).crop((7, 11, 1340 - 63, 788 - 79)),
    }
    out = OUT / "frames"
    out.mkdir(parents=True, exist_ok=True)
    index = {}
    for name, img in frames.items():
        img = img.convert("RGB")
        img.save(out / f"{name}.png", format="PNG")
        g, d = detect.sample_of(img)
        save_u8(g, out / f"{name}.grid.png")
        save_u16(d, out / f"{name}.detail.png")
        index[name] = {"width": img.width, "height": img.height, "grid_w": g.shape[1], "grid_h": g.shape[0]}
        print(f"frame {name}: {img.width}x{img.height} -> grid {g.shape[1]}x{g.shape[0]}")
    (out / "index.json").write_text(json.dumps(
        {"detail_scale": DETAIL_SCALE, "frames": index}, indent=1))


def export_mp4():
    fps = 2
    n = int(fixture.T["end"] * fps)
    with tempfile.TemporaryDirectory() as tmp:
        for i in range(n):
            fixture.render_frame(i / fps).save(Path(tmp) / f"f{i:05d}.png")
        subprocess.run(
            ["ffmpeg", "-y", "-loglevel", "error", "-framerate", str(fps),
             "-i", str(Path(tmp) / "f%05d.png"), "-c:v", "libx264", "-pix_fmt", "yuv420p",
             "-crf", "18", str(OUT / "f1_2fps.mp4")],
            check=True,
        )
    print(f"mp4: {n} frames at {fps} fps")


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    if len(sys.argv) > 1:
        export_strips(only=set(sys.argv[1:]))  # e.g. `export_fixtures.py F7`: just those strips
    else:
        export_strips()
        export_frames()
        export_mp4()
