"""Run the detection matrix on the fixture and write report.json plus sample PNGs.

Usage: .venv/bin/python samples/proto/run_all.py
"""

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
sys.path.insert(0, str(HERE))

import detect  # noqa: E402
import fixture  # noqa: E402
from detect import Config  # noqa: E402

END = fixture.T["end"]
OUT = HERE / "out"
MAX_SAMPLE_PNGS = 12

CONFIGS = [
    # P0 is the idea as first stated: check every 2 s, save when different from the last save.
    Config("P0", interval_s=2.0, settle_s=0.0, mask="none", build="each"),
    # P1 adds settle-before-save but no mask: a playing or live region blocks every save.
    Config("P1", mask="none"),
    # P2 is the recommended default: moving regions masked, sticky video, final build, skip revisits.
    Config("P2"),
    Config("P2-adaptive", video="adaptive"),
    Config("P2-periodic", video="periodic", periodic_s=2.0),
    Config("P2-each", build="each"),
    Config("P2-first", build="first"),
    Config("P2-revisit-save", on_return="save"),
]


def _cached_sampler():
    cache = {}

    def sample(t):
        key = round(t, 6)
        if key not in cache:
            cache[key] = detect.to_grid(fixture.render_frame(t))
        return cache[key]

    return sample


def run_matrix(write_samples=False):
    sample = _cached_sampler()
    results = []
    for cfg in CONFIGS:
        res = detect.run(cfg, sample, END)
        windows = {}
        for name, (a, b) in fixture.windows().items():
            windows[name] = sum(1 for e in res["events"] if a <= e["t_content"] < b)
        events = []
        png_total = 0
        out_dir = OUT / cfg.name
        if write_samples:
            out_dir.mkdir(parents=True, exist_ok=True)
        for i, e in enumerate(res["events"]):
            t = e["t_content"]
            img = fixture.render_frame(t)  # the state that was saved, as a full-size frame
            size = detect.png_bytes(img)
            png_total += size
            rec = {
                "t_content": round(t, 2),
                "t_save": round(e["t_save"], 2),
                "reason": e["reason"],
                "png_bytes": size,
            }
            if write_samples and i < MAX_SAMPLE_PNGS:
                name = f"{i:02d}_t{t:06.2f}.png"
                img.save(out_dir / name, format="PNG")
                rec["file"] = f"out/{cfg.name}/{name}"
            events.append(rec)
        results.append(
            {
                "id": cfg.name,
                "config": {
                    "interval_s": cfg.interval_s,
                    "settle_s": cfg.settle_s,
                    "mask": cfg.mask,
                    "video": cfg.video,
                    "build": cfg.build,
                    "on_return": cfg.on_return,
                },
                "count": len(res["events"]),
                "windows": windows,
                "events": events,
                "revisits": [round(t, 2) for t in res["revisits"]],
                "total_png_kb": round(png_total / 1024, 1),
                "avg_png_kb": round(png_total / max(len(events), 1) / 1024, 1),
                "detect_ms_total": round(res["detect_ms_total"], 1),
                "samples": res["samples"],
                "live_buffer_kb": round(res["live_buffer_bytes"] / 1024, 1),
                "history_kb": round(res["history_bytes"] / 1024, 1),
            }
        )
    return results


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    results = run_matrix(write_samples=True)
    report = {
        "fixture_end_s": END,
        "timeline": fixture.T,
        "expected_default": [
            {"label": label, "from": a, "to": b} for label, a, b in fixture.events_expected()
        ],
        "windows": {k: list(v) for k, v in fixture.windows().items()},
        "configs": results,
    }
    (HERE / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print(f"{'id':<17}{'count':>6}  reason-mix  " + "  ".join(fixture.windows().keys()))
    for r in results:
        reasons = {}
        for e in r["events"]:
            reasons[e["reason"]] = reasons.get(e["reason"], 0) + 1
        w = "  ".join(f"{r['windows'][k]}" for k in fixture.windows())
        print(f"{r['id']:<17}{r['count']:>6}  {reasons}  {w}")
    print("revisits (P2):", next(r["revisits"] for r in results if r["id"] == "P2"))


if __name__ == "__main__":
    main()
