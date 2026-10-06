"""Contact sheets of the saved states for each configuration, for the decision sheet.

Reads report.json (written by run_all.py) and renders each saved state from the fixture.

Usage: .venv/bin/python samples/proto/make_montages.py
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
import fixture  # noqa: E402

OUT = HERE / "out"
THUMB_W, THUMB_H = 427, 240
COLS = 3
MAX_TILES = 12
REASON_JA = {"settled": "落ち着き", "periodic": "定期", "end": "終了時に確定"}

report = json.loads((HERE / "report.json").read_text())
font = ImageFont.truetype(fixture.FONT_PATH, 20, index=0)
font_head = ImageFont.truetype(fixture.FONT_PATH, 26, index=0)

for cfg in report["configs"]:
    events = cfg["events"][:MAX_TILES]
    rows = max(1, (len(events) + COLS - 1) // COLS)
    width = COLS * THUMB_W + (COLS + 1) * 16
    height = 64 + rows * (THUMB_H + 52)
    sheet = Image.new("RGB", (width, height), (244, 244, 244))
    d = ImageDraw.Draw(sheet)
    head = f"{cfg['id']}：保存 {cfg['count']} 件"
    if cfg["count"] > MAX_TILES:
        head += f"（先頭 {MAX_TILES} 件を表示）"
    d.text((16, 16), head, font=font_head, fill=(17, 17, 17))
    for i, e in enumerate(events):
        img = fixture.render_frame(e["t_content"]).resize((THUMB_W, THUMB_H), Image.Resampling.LANCZOS)
        x = 16 + (i % COLS) * (THUMB_W + 16)
        y = 64 + (i // COLS) * (THUMB_H + 52)
        sheet.paste(img, (x, y))
        reason = REASON_JA.get(e["reason"], e["reason"])
        label = f"内容 {e['t_content']:.1f}s ／ 保存 {e['t_save']:.1f}s ／ {reason}"
        d.text((x, y + THUMB_H + 10), label, font=font, fill=(17, 17, 17))
    sheet.save(OUT / f"montage_{cfg['id']}.png", optimize=True)
    print("wrote", OUT / f"montage_{cfg['id']}.png", len(events), "tiles")
