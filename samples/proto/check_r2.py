"""Round 2 checks: the chosen behaviour on fixtures F1-F6. Exit code 0 only if every check passes.

These are the detection criteria proposed for ACCEPTANCE.md, run against the Python prototype.

Usage: .venv/bin/python samples/proto/check_r2.py
"""

import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
sys.path.insert(0, str(HERE))

import detect  # noqa: E402
import fixture  # noqa: E402
from run_r2 import sampler  # noqa: E402

failures = []


def check(label, ok, detail=""):
    print(("PASS " if ok else "FAIL ") + label + (f"  [{detail}]" if detail else ""))
    if not ok:
        failures.append(label)


def mean_abs(a, b):
    return float(np.abs(np.asarray(a, np.int16) - np.asarray(b, np.int16)).mean())


results = {}
for fid, spec in fixture.FIXTURES.items():
    det = detect.run(sampler(spec["render"]), spec["end"])
    results[fid] = det
    saves = det.saves
    times = [round(s["t_content"], 2) for s in saves]
    check(f"{fid}: saves {len(spec['saves'])}", len(saves) == len(spec["saves"]), f"got {len(saves)}: {times}")
    for label, a, b in spec["saves"]:
        hits = [s for s in saves if a <= s["t_content"] <= b]
        check(f"{fid}: one save for {label}", len(hits) == 1, f"[{a}, {b}] -> {len(hits)}")
    for label, a, b in spec["quiet"]:
        inside = [round(s["t_content"], 2) for s in saves if a <= s["t_content"] < b]
        check(f"{fid}: no save during {label}", not inside, f"{inside}")
    check(f"{fid}: revisits {spec['revisits']}", len(det.revisits) == spec["revisits"],
          f"got {[round(t, 2) for t in det.revisits]}")
    check(f"{fid}: single-colour reports {spec['blanks']}", len(det.blanks) == spec["blanks"],
          f"got {[round(t, 2) for t in det.blanks]}")
    uniform_saved = [s["held_t"] for s in saves if float(detect.to_grid(spec["render"](s["held_t"])).std()) < 2.0]
    check(f"{fid}: no single-colour image saved", not uniform_saved, f"{uniform_saved}")

# Sharpening: each F3 save must be the sharp version of its slide.
f3 = fixture.FIXTURES["F3"]
for s, sharp_from, name in zip(results["F3"].saves, f3["sharp_from"], f3["sharp_names"]):
    saved = f3["render"](s["held_t"])
    diff = mean_abs(saved, fixture.f3_sharp_reference(name))
    check(f"F3: slide {name} saved from its sharp frames (held {s['held_t']:.1f} s >= {sharp_from} s)",
          s["held_t"] >= sharp_from and diff <= 1.0, f"mean |diff| to sharp = {diff:.2f}")

# Pointer: the saved slide A in F5 must not contain the cursor or laser.
f5_a = results["F5"].saves[0] if results["F5"].saves else None
if f5_a is not None:
    diff = mean_abs(fixture.FIXTURES["F5"]["render"](f5_a["held_t"]), fixture.f5_clean_a())
    check("F5: saved slide A has no pointer in it", diff <= 0.5, f"mean |diff| to clean A = {diff:.3f}")

# Determinism: the same input gives the same saves.
again = detect.run(sampler(fixture.FIXTURES["F1"]["render"]), fixture.FIXTURES["F1"]["end"])
check("F1: same input, same saves",
      [(s["t_content"], s["t_save"], s["held_t"]) for s in again.saves]
      == [(s["t_content"], s["t_save"], s["held_t"]) for s in results["F1"].saves])

print()
if failures:
    print(f"{len(failures)} check(s) failed")
    sys.exit(1)
print("all checks passed")
