#!/usr/bin/env python3
"""Generates the ZoomSlidesPicker icon sources (docs/impl/shell.md > Crate and config).

Draws:
- src-tauri/icons/source.png: a 1024x1024 monochrome app icon (a slide rectangle, no
  text), in the app's accent colour (src/styles/tokens.css --color-accent). Run
  `pnpm tauri icon icons/source.png` from src-tauri/ afterwards to generate the bundle
  set (icon.icns, icon.ico, 32x32.png, 128x128.png, 128x128@2x.png, ...).
- src-tauri/icons/tray-{stopped,running,paused}-{32,64}.png: tray template images
  (SPEC.md > 画面と操作; docs/impl/shell.md). Black shapes on a transparent background --
  macOS recolours template images from their alpha channel alone, so the exact RGB value
  does not matter as long as it is opaque black.
    - stopped: the slide rectangle, outline only
    - running: the slide rectangle, filled
    - paused: the slide rectangle outline, plus two vertical bars (a pause glyph)

Every shape is drawn supersampled (8x the target size) and downsized with Lanczos
resampling, since Pillow's ImageDraw does not anti-alias on its own.
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / "src-tauri" / "icons"

ACCENT = (0x2F, 0x66, 0x90, 0xFF)  # --color-accent, src/styles/tokens.css
BLACK = (0, 0, 0, 255)
SUPERSAMPLE = 8


def slide_rect(size):
    """The slide rectangle's box (left, top, right, bottom), centered in a `size` canvas."""
    w, h = size * 0.74, size * 0.56
    x0, y0 = (size - w) / 2, (size - h) / 2
    return (x0, y0, x0 + w, y0 + h)


def render(size, draw_fn):
    """Draws at SUPERSAMPLE x `size`, then downsizes to `size` with anti-aliasing."""
    big = size * SUPERSAMPLE
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    draw_fn(ImageDraw.Draw(img), big)
    return img.resize((size, size), Image.LANCZOS)


def make_app_icon(size=1024):
    def draw(d, s):
        rect = slide_rect(s)
        radius = (rect[2] - rect[0]) * 0.11
        d.rounded_rectangle(rect, radius=radius, fill=ACCENT)

    return render(size, draw)


def make_tray(state, size):
    def draw(d, s):
        rect = slide_rect(s)
        radius = (rect[2] - rect[0]) * 0.11
        stroke = max(1, round(s * 0.045))
        if state == "running":
            d.rounded_rectangle(rect, radius=radius, fill=BLACK)
            return
        d.rounded_rectangle(rect, radius=radius, outline=BLACK, width=stroke)
        if state == "paused":
            x0, y0, x1, y1 = rect
            w, h = x1 - x0, y1 - y0
            bar_w, bar_h, gap = w * 0.14, h * 0.56, w * 0.14
            cy = (y0 + y1) / 2
            for i in (-1, 1):
                cx = (x0 + x1) / 2 + i * (bar_w / 2 + gap / 2)
                d.rounded_rectangle(
                    (cx - bar_w / 2, cy - bar_h / 2, cx + bar_w / 2, cy + bar_h / 2),
                    radius=bar_w * 0.3,
                    fill=BLACK,
                )

    return render(size, draw)


def main():
    ICONS.mkdir(parents=True, exist_ok=True)

    make_app_icon().save(ICONS / "source.png")

    for state in ("stopped", "running", "paused"):
        for size in (32, 64):
            make_tray(state, size).save(ICONS / f"tray-{state}-{size}.png")

    print(f"wrote source.png and 6 tray-*.png files to {ICONS}")


if __name__ == "__main__":
    main()
