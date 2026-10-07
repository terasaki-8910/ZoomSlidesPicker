"""Synthetic lecture-screen fixture.

render_frame(t) is a pure function of time: the 1280x720 screen a viewer sees at
t seconds. Video and webcam tiles are keyed to the exact float t, so they never
repeat on a fixed period. (A periodic CSS animation aliased against the 500 ms
sampling interval in the xcap spike; keying to t avoids that by construction.)

Video and webcam content is smooth random noise, so every sample differs.
That is a worst case for diff-based detection, which is what we want to stress.
"""

import hashlib

import numpy as np
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720
FONT_PATH = "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc"  # Hiragino Sans, macOS
INK = (17, 17, 17)
PAPER = (255, 255, 255)
PEN = (200, 30, 30)
VIDEO_BOX = (760, 300, 1200, 540)  # slide 3 playback area, 440x240
CAM_BOX = (940, 470, 1260, 650)    # slide 5 webcam tile, 320x180

# Timeline in seconds. Each entry is the time an event starts.
T = {
    "s2": 8.0,
    "s2_b1": 8.4, "s2_b2": 11.4, "s2_b3": 14.4,  # slide 2 build (3 bullets)
    "s3": 20.0,                                   # slide 3 (video slide) starts
    "video_on": 22.5, "video_off": 41.4,          # video plays 22.5-41.4, then frozen
    "s2_return": 51.9,                            # back to slide 2 (final state)
    "s4": 60.3,                                   # short slide, shown 3.3 s
    "s5": 63.6, "s5_b2": 69.0,                    # slide 5, webcam tile on from here on
    "pen_on": 76.4, "pen_off": 78.4,              # pen annotation drawn 2 s
    "end": 88.4,
}
PEN_PATH = [(120, 560), (260, 470), (330, 600), (470, 500), (560, 610), (700, 520)]

_fonts = {}


def _font(size):
    if size not in _fonts:
        _fonts[size] = ImageFont.truetype(FONT_PATH, size, index=0)
    return _fonts[size]


def _noise(key, w, h):
    """Smooth random tile keyed to an exact value. A new key gives an entirely new tile."""
    seed = int.from_bytes(hashlib.blake2b(repr(key).encode(), digest_size=8).digest(), "little")
    rng = np.random.default_rng(seed)
    low = rng.integers(0, 256, size=(h // 8, w // 8, 3), dtype=np.uint8)
    return Image.fromarray(low, "RGB").resize((w, h), Image.Resampling.BICUBIC)


def _slide(title, bullets):
    img = Image.new("RGB", (W, H), PAPER)
    d = ImageDraw.Draw(img)
    d.rectangle([40, 40, W - 40, H - 40], outline=INK, width=3)
    d.text((100, 80), title, font=_font(60), fill=INK)
    for i, text in enumerate(bullets):
        d.text((130, 220 + i * 84), "•  " + text, font=_font(40), fill=INK)
    return img, d


def _pen_line(d, t):
    frac = min(max((t - T["pen_on"]) / (T["pen_off"] - T["pen_on"]), 0.0), 1.0)
    pos = frac * (len(PEN_PATH) - 1)
    n = int(pos)
    pts = PEN_PATH[: n + 1]
    if n < len(PEN_PATH) - 1:
        local = pos - n
        (ax, ay), (bx, by) = PEN_PATH[n], PEN_PATH[n + 1]
        pts.append((ax + (bx - ax) * local, ay + (by - ay) * local))
    if len(pts) >= 2:
        d.line(pts, fill=PEN, width=6, joint="curve")


def render_frame(t: float) -> Image.Image:
    if t < T["s2"]:
        return _slide("Slide 1 導入", ["イントロダクション"])[0]

    if t < T["s3"]:
        n = sum(t >= T[k] for k in ("s2_b1", "s2_b2", "s2_b3"))
        return _slide("Slide 2 比較", ["項目 A", "項目 B", "項目 C"][:n])[0]

    if t < T["s2_return"]:
        img, d = _slide("Slide 3 デモ動画", ["再生開始"])
        x0, y0, x1, y1 = VIDEO_BOX
        if t < T["video_on"]:
            d.rectangle(VIDEO_BOX, fill=(136, 136, 136))
        else:
            img.paste(_noise(min(t, T["video_off"]), x1 - x0, y1 - y0), (x0, y0))
        return img

    if t < T["s4"]:
        return _slide("Slide 2 比較", ["項目 A", "項目 B", "項目 C"])[0]

    if t < T["s5"]:
        return _slide("Slide 4 短時間", ["一瞬だけ表示"])[0]

    bullets = ["出典 A"] + (["出典 B"] if t >= T["s5_b2"] else [])
    img, d = _slide("Slide 5 参考資料", bullets)
    x0, y0, x1, y1 = CAM_BOX
    img.paste(_noise(t, x1 - x0, y1 - y0), (x0, y0))  # webcam: always on, never repeats
    if t >= T["pen_on"]:
        _pen_line(d, t)
    return img


# ---------------------------------------------------------------------------
# Round 2 fixtures. F1 is render_frame above; the others cover cases the round 1
# fixture did not: playback at 2x, a slide that sharpens late, single-colour
# screens, pointer movement and a crossfade, and a build on a dark background.
# ---------------------------------------------------------------------------

DARK_BG = (30, 30, 30)
DARK_INK = (235, 235, 235)
GREY = (128, 128, 128)
BLACK = (0, 0, 0)

_cache = {}


def _cached(key, make):
    if key not in _cache:
        _cache[key] = make()
    return _cache[key].copy()


def _slide_dark(title, bullets):
    img = Image.new("RGB", (W, H), DARK_BG)
    d = ImageDraw.Draw(img)
    d.rectangle([40, 40, W - 40, H - 40], outline=DARK_INK, width=3)
    d.text((100, 80), title, font=_font(60), fill=DARK_INK)
    for i, text in enumerate(bullets):
        d.text((130, 220 + i * 84), "•  " + text, font=_font(40), fill=DARK_INK)
    return img


def _lowres(img):
    """What a video call shows before the shared screen sharpens: a 1/4-size frame scaled up."""
    small = img.resize((W // 4, H // 4), Image.Resampling.BOX)
    return small.resize((W, H), Image.Resampling.BILINEAR)


def _dense(name):
    """A slide with fine detail (small text and a chart), so blur is visible."""
    def make():
        img, d = _slide(f"Slide {name} 詳細", [f"{name}-1 定義と前提", f"{name}-2 計算の手順", f"{name}-3 結果の読み方"])
        x0, y0 = 760, 230
        d.line([(x0, y0), (x0, y0 + 300), (x0 + 400, y0 + 300)], fill=INK, width=2)
        for i, h in enumerate([120, 210, 160, 260, 90]):
            bx = x0 + 30 + i * 74
            d.rectangle([bx, y0 + 300 - h, bx + 44, y0 + 300], outline=INK, width=2)
            d.text((bx + 4, y0 + 308), f"{h}", font=_font(18), fill=INK)
        for k in range(6):
            d.text((130, 500 + k * 26), f"注 {k + 1}: 値は {name}{k} 群の平均（n = {20 + k}）", font=_font(18), fill=INK)
        return img
    return _cached(("dense", name), make)


# F3: each slide first appears low-resolution, then sharpens. (start, sharp_from, end, name)
F3_PHASES = [(0.0, 1.5, 10.0, "A"), (10.0, 11.0, 20.0, "B"), (20.0, 23.0, 32.0, "C"), (32.0, 32.4, 40.0, "D")]


def render_f3(t):
    for start, sharp_from, end, name in F3_PHASES:
        if start <= t < end or (name == "D" and t >= start):
            img = _dense(name)
            return img if t >= sharp_from else _lowres(img)
    return _dense("D")


def f3_sharp_reference(name):
    return _dense(name)


# F4: a slide, then black (sharing stopped or protected), grey, the same slide again, a new slide.
def render_f4(t):
    if t < 10.0:
        return _cached(("f4", "A"), lambda: _slide("Slide A 共有中", ["項目 1", "項目 2"])[0])
    if t < 25.0:
        return Image.new("RGB", (W, H), BLACK)
    if t < 37.0:
        return Image.new("RGB", (W, H), GREY)
    if t < 42.0:
        return _cached(("f4", "A"), lambda: _slide("Slide A 共有中", ["項目 1", "項目 2"])[0])
    return _cached(("f4", "B"), lambda: _slide("Slide B 再開後", ["項目 3", "項目 4"])[0])


# F5: pointer movement on one slide, then a 0.6 s crossfade to the next slide.
F5_CURSOR = [(200, 200), (900, 250), (1000, 600), (300, 620), (250, 300)]
F5_LASER = [(300, 300), (1000, 320), (800, 560), (400, 500)]
ARROW = [(0, 0), (0, 22), (6, 16), (10, 25), (14, 23), (10, 15), (17, 15)]


def _along(path, frac):
    pos = frac * (len(path) - 1)
    n = min(int(pos), len(path) - 2)
    local = pos - n
    (ax, ay), (bx, by) = path[n], path[n + 1]
    return ax + (bx - ax) * local, ay + (by - ay) * local


def _f5_a():
    return _cached(("f5", "A"), lambda: _slide("Slide A 講義の要点", ["定義", "例", "注意点", "まとめ"])[0])


def _f5_b():
    return _cached(("f5", "B"), lambda: _slide("Slide B 演習", ["問題 1", "問題 2"])[0])


def render_f5(t):
    if t >= 30.6:
        return _f5_b()
    if t >= 30.0:
        return Image.blend(_f5_a(), _f5_b(), (t - 30.0) / 0.6)
    img = _f5_a()
    d = ImageDraw.Draw(img)
    if 2.0 <= t < 12.0:
        x, y = _along(F5_CURSOR, (t - 2.0) / 10.0)
        pts = [(x + px, y + py) for px, py in ARROW]
        d.polygon(pts, fill=INK, outline=PAPER)
    elif 14.0 <= t < 24.0:
        x, y = _along(F5_LASER, (t - 14.0) / 10.0)
        d.ellipse([x - 10, y - 10, x + 10, y + 10], fill=(255, 150, 150))
        d.ellipse([x - 6, y - 6, x + 6, y + 6], fill=(255, 40, 40))
    elif 24.0 <= t < 30.0:
        x = 640 + 15 * np.cos(2 * np.pi * 3 * t)
        y = 420 + 15 * np.sin(2 * np.pi * 2 * t)
        pts = [(x + px, y + py) for px, py in ARROW]
        d.polygon(pts, fill=INK, outline=PAPER)
    return img


def f5_clean_a():
    return _f5_a()


# F6: a build on a dark background (light text added), then a different dark slide.
def render_f6(t):
    if t >= 15.0:
        return _cached(("f6", "D2"), lambda: _slide_dark("Slide D2 次の話題", ["要点 X", "要点 Y"]))
    n = sum(t >= s for s in (2.0, 4.0, 6.0))
    return _cached(("f6", n), lambda: _slide_dark("Slide D1 暗い背景", ["項目 1", "項目 2", "項目 3"][:n]))


# F7: a recorded lecture with the speaker's camera in a round tile (found in real use,
# 2026-10-07). The face changes a few cells every one to three seconds and sometimes stays
# still for a long while; it is never still or moving long enough for the motion mask.
# Slide A, slide B built in three steps, slide C with a laser pointer, back to A.
F7_FACE_C = (1150, 600)
F7_FACE_R = 80


def _f7_pose_changes():
    """(t, dx, dy, mouth) at each change of the speaker's pose, from a fixed seed."""
    rng = np.random.default_rng(7)
    gaps = [1.5, 2.0, 1.0, 2.5, 1.5, 3.0, 1.0, 2.0, 9.0, 1.5, 1.0, 2.5, 18.0, 1.0, 2.0, 1.5, 6.0,
            1.0, 2.5, 1.5, 12.0, 2.0, 1.0, 3.0, 1.5, 7.0, 1.0, 2.0, 1.5, 2.5, 16.0, 1.0, 1.5, 2.0,
            1.0, 3.0, 1.5, 2.0, 10.0, 1.5, 1.0, 2.0, 1.5, 2.5, 1.0, 20.0]
    t, out = 0.0, [(0.0, 0, 0, 2)]
    for g in gaps:
        t += g
        out.append((t, int(rng.integers(-6, 7)), int(rng.integers(-3, 4)), int(rng.integers(0, 7))))
    return out


F7_POSES = _f7_pose_changes()


def _f7_face(t):
    pose = max((p for p in F7_POSES if p[0] <= t), key=lambda p: p[0])
    _, dx, dy, mouth = pose
    size = 2 * F7_FACE_R
    tile = Image.new("RGB", (size, size), (90, 110, 140))
    d = ImageDraw.Draw(tile)
    cx, cy = F7_FACE_R + dx, F7_FACE_R + 12 + dy
    d.ellipse([cx - 70, cy + 45, cx + 70, cy + 130], fill=(200, 120, 140))      # shoulders
    d.ellipse([cx - 38, cy - 58, cx + 38, cy + 20], fill=(40, 35, 30))         # hair
    d.ellipse([cx - 33, cy - 42, cx + 33, cy + 46], fill=(225, 190, 165))      # face
    for ex in (cx - 13, cx + 13):
        d.ellipse([ex - 4, cy - 6, ex + 4, cy + 2], fill=(30, 30, 30))          # eyes
    d.ellipse([cx - 10, cy + 22, cx + 10, cy + 24 + mouth], fill=(120, 40, 50))  # mouth
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).ellipse([0, 0, size - 1, size - 1], fill=255)
    return tile, mask


def render_f7(t):
    if t < 40.0 or t >= 110.0:
        img = _cached(("f7", "A"), lambda: _slide("Slide A 授業概要", ["授業名：デジタル人文学", "担当：例 太郎", "評価：レポート 100%"])[0])
    elif t < 80.0:
        n = sum(t >= s for s in (40.0, 43.0, 46.0))
        img = _cached(("f7", "B", n), lambda: _slide("Slide B 研究の流れ", ["資料の収集", "構造化", "分析と公開"][:n])[0])
    else:
        img = _cached(("f7", "C"), lambda: _slide("Slide C 参考文献", ["文献 1", "文献 2"])[0])
        if 85.0 <= t < 95.0:
            d = ImageDraw.Draw(img)
            x, y = _along(F5_LASER, (t - 85.0) / 10.0)
            d.ellipse([x - 10, y - 10, x + 10, y + 10], fill=(255, 150, 150))
            d.ellipse([x - 6, y - 6, x + 6, y + 6], fill=(255, 40, 40))
    tile, mask = _f7_face(t)
    img.paste(tile, (F7_FACE_C[0] - F7_FACE_R, F7_FACE_C[1] - F7_FACE_R), mask)
    return img


# ---------------------------------------------------------------------------
# Registry and expectations for the chosen behaviour (DECISIONS.md, round 1):
# moving regions masked while they move and compared again once still (R1-5 b),
# builds merged into the final state (R1-6 a), returns to a saved slide skipped (R1-7 a).
# Save windows are on content time: the moment the saved state settled.
# ---------------------------------------------------------------------------

FIXTURES = {
    "F1": {
        "label": "授業（等速、88.4 秒）",
        "render": render_frame,
        "end": T["end"],
        "saves": [
            ("スライド 1", 0.5, 2.0),
            ("スライド 2（箇条書き 3 つをまとめた最終状態）", 15.0, 16.0),
            ("スライド 3（動画の前）", 20.5, 21.5),
            ("スライド 3（動画が止まった後）", 41.4, 45.0),
            ("スライド 4（3.3 秒だけ表示）", 61.0, 62.5),
            ("スライド 5（箇条書き 2 つ、Web カメラあり）", 69.5, 71.0),
            ("スライド 5（ペンの書き込み）", 79.0, 80.5),
        ],
        "revisits": 1,
        "blanks": 0,
        "quiet": [("動画の再生中", T["video_on"], T["video_off"])],
    },
    "F2": {
        "label": "授業（2 倍速、44.2 秒）",
        "render": lambda t: render_frame(2.0 * t),
        "end": T["end"] / 2.0,
        "saves": [
            ("スライド 1", 0.25, 1.5),
            ("スライド 2（最終状態）", 7.2, 8.5),
            ("スライド 3（動画の前）", 10.0, 11.25),
            ("スライド 3（動画が止まった後）", 20.7, 25.0),
            ("スライド 4（1.65 秒だけ表示）", 30.15, 31.8),
            ("スライド 5（箇条書きとペンをまとめた最終状態）", 39.2, 41.0),
        ],
        "revisits": 1,
        "blanks": 0,
        "quiet": [("動画の再生中", T["video_on"] / 2.0, T["video_off"] / 2.0)],
    },
    "F3": {
        "label": "遅れて鮮明になる（40 秒）",
        "render": render_f3,
        "end": 40.0,
        "saves": [("A", 0.5, 3.0), ("B", 10.5, 13.0), ("C", 20.5, 25.0), ("D", 32.0, 34.5)],
        "revisits": 0,
        "blanks": 0,
        "quiet": [],
        "sharp_from": [1.5, 11.0, 23.0, 32.4],
        "sharp_names": ["A", "B", "C", "D"],
    },
    "F4": {
        "label": "黒と灰色の画面（52 秒）",
        "render": render_f4,
        "end": 52.0,
        "saves": [("A", 0.5, 2.0), ("B", 42.5, 44.0)],
        # A -> black -> A resumes the slide saved last; a revisit is a return to an
        # earlier saved slide. (Pre-registered as 1, corrected after the first run.)
        "revisits": 0,
        "blanks": 1,
        "quiet": [("黒と灰色の間", 10.0, 37.0)],
    },
    "F5": {
        "label": "カーソル・レーザー・切り替えの効果（40 秒）",
        "render": render_f5,
        "end": 40.0,
        "saves": [("A", 0.5, 2.0), ("B", 30.6, 32.5)],
        "revisits": 0,
        "blanks": 0,
        "quiet": [("カーソルとレーザーの間", 2.0, 30.0)],
    },
    "F6": {
        "label": "暗い背景の段階表示（25 秒）",
        "render": render_f6,
        "end": 25.0,
        "saves": [("D1（箇条書き 3 つの最終状態）", 6.0, 8.0), ("D2", 15.0, 17.0)],
        "revisits": 0,
        "blanks": 0,
        "quiet": [],
    },
    "F7": {
        "label": "講師のカメラが丸い窓で重なる録画（140 秒）",
        "render": render_f7,
        "end": 140.0,
        "saves": [
            ("スライド A", 0.5, 8.0),
            ("スライド B（箇条書き 3 つの最終状態）", 46.0, 48.5),
            ("スライド C", 80.0, 82.5),
        ],
        "revisits": 1,
        "blanks": 0,
        "quiet": [
            ("A：カメラだけが動く間", 8.0, 40.0),
            ("B：カメラだけが動く間", 48.5, 80.0),
            ("C：カメラとレーザーの間", 82.5, 110.0),
            ("A に戻った後", 110.0, 140.0),
        ],
    },
}
