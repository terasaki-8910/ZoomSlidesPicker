"""Assertions for the detection prototype. Exit code 0 only if every check passes.

Usage: .venv/bin/python samples/proto/check.py
"""

import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
sys.path.insert(0, str(HERE))

import detect  # noqa: E402
import fixture  # noqa: E402
import run_all  # noqa: E402

WIN = fixture.windows()
results = {c.name: detect.run(c, run_all._cached_sampler(), run_all.END) for c in run_all.CONFIGS}


failures = []


def check(label, ok, detail=""):
    print(("PASS " if ok else "FAIL ") + label + (f"  [{detail}]" if detail else ""))
    if not ok:
        failures.append(label)


def count(config_id, window_name):
    a, b = WIN[window_name]
    return sum(1 for e in results[config_id]["events"] if a <= e["t_content"] < b)


def times(config_id, window_name):
    a, b = WIN[window_name]
    return [round(e["t_content"], 1) for e in results[config_id]["events"] if a <= e["t_content"] < b]


# 1) Default configuration: one saved state per slide, none during video, none for the revisit.
p2 = results["P2"]["events"]
expected = fixture.events_expected()
check(
    "P2 saves exactly one state per distinct slide state (6)",
    len(p2) == len(expected),
    f"got {len(p2)}: {[round(e['t_content'], 1) for e in p2]}",
)
for label, a, b in expected:
    hits = [e for e in p2 if a <= e["t_content"] <= b]
    check(f"P2 event in window for: {label}", len(hits) == 1, f"[{a}, {b}] -> {len(hits)}")
check("P2 saves nothing while the video plays", count("P2", "video playing") == 0)
check("P2 saves nothing for the frozen frame after the video", count("P2", "after video (frozen)") == 0)
check("P2 does not save again on returning to slide 2", count("P2", "slide2 revisit") == 0)
check("P2 records the slide 2 return as a revisit", len(results["P2"]["revisits"]) == 1,
      f"revisits at {[round(t, 1) for t in results['P2']['revisits']]}")

# 2) Naive polling blows up during video and webcam.
check("P0 (every 2 s) saves 8+ times during the video",
      count("P0", "video playing") >= 8, f"{count('P0', 'video playing')}")
check("P0 (every 2 s) saves 8+ times while the webcam runs",
      count("P0", "slide5 webcam") >= 8, f"{count('P0', 'slide5 webcam')}")

# 3) Settle without masking misses everything while a live region runs.
check("P1 (no mask) saves nothing on slide 5 while the webcam runs",
      count("P1", "slide5 webcam") == 0)

# 4) Build policies on slide 2 (three bullets, 3 s apart).
check("P2-each saves each stage of the slide 2 build (3)", count("P2-each", "slide2 build") >= 3,
      f"{count('P2-each', 'slide2 build')}")
check("P2-first saves only the first state of slide 2 (1)", count("P2-first", "slide2 build") == 1,
      f"{count('P2-first', 'slide2 build')}")
check("P2-first saves only the first state of slide 5 (1)", count("P2-first", "slide5 webcam") == 1,
      f"{count('P2-first', 'slide5 webcam')}")

# 5) Video policy alternatives.
check("P2-adaptive saves one frame after the video ends",
      count("P2-adaptive", "after video (frozen)") == 1, f"{times('P2-adaptive', 'after video (frozen)')}")
check("P2-periodic saves at least 8 times during the video",
      count("P2-periodic", "video playing") >= 8, f"{count('P2-periodic', 'video playing')}")

# 6) Revisit policy alternative.
check("P2-revisit-save adds exactly one save (the slide 2 return)",
      len(results["P2-revisit-save"]["events"]) == len(p2) + 1,
      f"{len(results['P2-revisit-save']['events'])} vs {len(p2) + 1}")

print()
if failures:
    print(f"{len(failures)} check(s) failed")
    sys.exit(1)
print("all checks passed")
