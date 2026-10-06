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


def events_expected():
    """Ground truth for the default configuration (P2: sticky video, final build, skip revisits).

    One saved state per slide. Each entry is (label, from, to) for the content time, which is
    the moment the state settled. Save time is later (the final build waits for a quiet period).
    """
    return [
        ("slide1 導入", 0.5, 2.0),
        ("slide2 bullets A-C merged into one state", 15.0, 16.0),
        ("slide3 before video (video region excluded)", 20.5, 21.5),
        ("slide4 short slide (shown 3.3 s)", 61.0, 62.5),
        ("slide5 bullets A-B (webcam on, bullet 1 merged)", 69.5, 71.0),
        ("slide5 pen annotation (saved after a 5 s pause)", 79.0, 80.5),
    ]


def windows():
    """Named time windows used by the sheet and by check.py."""
    return {
        "slide1": (0.0, T["s2"]),
        "slide2 build": (T["s2"], T["s3"]),
        "slide3 before video": (T["s3"], T["video_on"]),
        "video playing": (T["video_on"], T["video_off"]),
        "after video (frozen)": (T["video_off"], T["s2_return"]),
        "slide2 revisit": (T["s2_return"], T["s4"]),
        "slide4 short": (T["s4"], T["s5"]),
        "slide5 webcam": (T["s5"], T["end"]),
    }
