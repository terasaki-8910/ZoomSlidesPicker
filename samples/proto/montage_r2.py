"""Round 2 sheet samples: a contact sheet of the saved images per fixture, and a 1:1 crop
comparing the blurred first frame of an F3 slide with the frame that was saved.

Reads report_r2.json (run_r2.py). Output: samples/proto/out_r2/montage_F*.png, f3_crop.png

Usage: .venv/bin/python samples/proto/montage_r2.py
"""

import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "fixture"))
import fixture  # noqa: E402

OUT = HERE / "out_r2"
THUMB_W, THUMB_H = 427, 240
COLS = 3
REASON_JA = {"settled": "落ち着き", "end": "終了時に確定"}

font = ImageFont.truetype(fixture.FONT_PATH, 20, index=0)
font_head = ImageFont.truetype(fixture.FONT_PATH, 26, index=0)
report = json.loads((HERE / "report_r2.json").read_text())

for fx in report["fixtures"]:
    spec = fixture.FIXTURES[fx["id"]]
    saves = fx["saves"]
    rows = max(1, (len(saves) + COLS - 1) // COLS)
    width = COLS * THUMB_W + (COLS + 1) * 16
    height = 64 + rows * (THUMB_H + 52)
    sheet = Image.new("RGB", (width, height), (244, 244, 244))
    d = ImageDraw.Draw(sheet)
    d.text((16, 16), f"{fx['id']} {fx['label']}：保存 {len(saves)} 件", font=font_head, fill=(17, 17, 17))
    for i, s in enumerate(saves):
        img = spec["render"](s["held_t"]).resize((THUMB_W, THUMB_H), Image.Resampling.LANCZOS)
        x = 16 + (i % COLS) * (THUMB_W + 16)
        y = 64 + (i // COLS) * (THUMB_H + 52)
        sheet.paste(img, (x, y))
        label = f"内容 {s['t_content']:.1f}s ／ 保存 {s['t_save']:.1f}s ／ {REASON_JA.get(s['reason'], s['reason'])}"
        d.text((x, y + THUMB_H + 10), label, font=font, fill=(17, 17, 17))
    sheet.save(OUT / f"montage_{fx['id']}.png", optimize=True)
    print("wrote", OUT / f"montage_{fx['id']}.png")

# F3 slide C: blurred for 3 s (20-23 s), sharp from 23 s. Crop the chart and notes at 1:1.
f3 = next(fx for fx in report["fixtures"] if fx["id"] == "F3")
saved_c = f3["saves"][2]
box = (700, 210, 1200, 690)
blurred = fixture.render_f3(21.0).crop(box)
saved = fixture.render_f3(saved_c["held_t"]).crop(box)
pad = 16
w, h = blurred.size
crop_sheet = Image.new("RGB", (w * 2 + pad * 3, h + 90), (244, 244, 244))
d = ImageDraw.Draw(crop_sheet)
crop_sheet.paste(blurred, (pad, 60))
crop_sheet.paste(saved, (w + pad * 2, 60))
d.text((pad, 18), "21.0 s：最初に届いた画面（ぼやけている）", font=font, fill=(17, 17, 17))
d.text((w + pad * 2, 18), f"保存された画像（{saved_c['held_t']:.1f} s のフレーム）", font=font, fill=(17, 17, 17))
crop_sheet.save(OUT / "f3_crop.png", optimize=True)
print("wrote", OUT / "f3_crop.png")
