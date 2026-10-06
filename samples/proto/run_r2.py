"""Round 2: run the chosen detector on every fixture; write report_r2.json and the saved images.

Usage: .venv/bin/python samples/proto/run_r2.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
sys.path.insert(0, str(HERE))

import detect  # noqa: E402
import fixture  # noqa: E402
from detect import Config, sample_of  # noqa: E402

OUT = HERE / "out_r2"


def sampler(render):
    cache = {}

    def sample(t):
        key = round(t, 6)
        if key not in cache:
            cache[key] = sample_of(render(t))
        return cache[key]

    return sample


def run_fixture(fid, spec, cfg=Config(), write=False):
    det = detect.run(sampler(spec["render"]), spec["end"], cfg)
    out_dir = OUT / fid
    if write:
        out_dir.mkdir(parents=True, exist_ok=True)
    saves = []
    for i, s in enumerate(det.saves):
        img = spec["render"](s["held_t"])  # the frame the app would write
        rec = {
            "t_content": round(s["t_content"], 2),
            "t_save": round(s["t_save"], 2),
            "held_t": round(s["held_t"], 2),
            "reason": s["reason"],
            "png_kb": round(detect_png_kb(img), 1),
        }
        if write:
            name = f"{i + 1:03d}_t{s['t_content']:06.2f}.png"
            img.save(out_dir / name, format="PNG")
            rec["file"] = f"out_r2/{fid}/{name}"
        saves.append(rec)
    samples = int(spec["end"] / cfg.interval_s) + 1
    return {
        "id": fid,
        "label": spec["label"],
        "end": spec["end"],
        "saves": saves,
        "revisits": [round(t, 2) for t in det.revisits],
        "blanks": [round(t, 2) for t in det.blanks],
        "detect_ms_per_sample": round(det.detect_s * 1000.0 / samples, 3),
        "expected": {
            "saves": [{"label": lb, "from": a, "to": b} for lb, a, b in spec["saves"]],
            "revisits": spec["revisits"],
            "blanks": spec["blanks"],
            "quiet": [{"label": lb, "from": a, "to": b} for lb, a, b in spec["quiet"]],
        },
    }


def detect_png_kb(img):
    import io

    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getbuffer().nbytes / 1024


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    results = [run_fixture(fid, spec, write=True) for fid, spec in fixture.FIXTURES.items()]
    (HERE / "report_r2.json").write_text(json.dumps({"fixtures": results}, ensure_ascii=False, indent=2))
    for r in results:
        print(f"{r['id']} {r['label']}: saves={len(r['saves'])} (expected {len(r['expected']['saves'])}) "
              f"revisits={r['revisits']} blanks={r['blanks']} {r['detect_ms_per_sample']} ms/sample")
        for s in r["saves"]:
            print(f"    content {s['t_content']:6.2f}  save {s['t_save']:6.2f}  held {s['held_t']:6.2f}  {s['reason']}")


if __name__ == "__main__":
    main()
