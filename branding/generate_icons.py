#!/usr/bin/env python3
"""Regenerate every Arenna Remote icon/logo from branding/logo.svg.

Usage (from the repository root):

    python3 branding/generate_icons.py

Requires: cairosvg, Pillow. The wordmark uses the Inter font bundled in
branding/fonts (SIL Open Font License).
"""

import io
import re
from pathlib import Path

import cairosvg
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BRANDING = ROOT / "branding"
LOGO_SVG = (BRANDING / "logo.svg").read_text(encoding="utf-8")
FONT = BRANDING / "fonts" / "Inter-Bold.ttf"

PRODUCT_NAME = "Arenna Remote"
TEXT_DARK = (20, 24, 28, 255)
TEXT_LIGHT = (255, 255, 255, 255)

ICO_SIZES = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
TRAY_SIZES = [16, 20, 24, 32, 40, 48, 64]
# Android density buckets: (folder suffix, launcher px, adaptive foreground px, notification px)
ANDROID_DENSITIES = [
    ("mdpi", 48, 108, 24),
    ("hdpi", 72, 162, 36),
    ("xhdpi", 96, 216, 48),
    ("xxhdpi", 144, 324, 72),
    ("xxxhdpi", 192, 432, 96),
]


def square_svg() -> str:
    """The logo with a square viewBox so it can be used as an app icon."""
    return re.sub(r'viewBox="[^"]*"', 'viewBox="22 -3 50 50"', LOGO_SVG, count=1)


def render(svg: str, width: int, height: int) -> Image.Image:
    png = cairosvg.svg2png(bytestring=svg.encode("utf-8"), output_width=width, output_height=height)
    return Image.open(io.BytesIO(png)).convert("RGBA")


def logo_on_canvas(size: int, scale: float, background=None, round_bg=False) -> Image.Image:
    """Square canvas of `size` px with the logo centred, occupying `scale` of the side."""
    canvas = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    if background is not None:
        draw = ImageDraw.Draw(canvas)
        if round_bg:
            draw.ellipse((0, 0, size - 1, size - 1), fill=background)
        else:
            radius = round(size * 0.18)
            draw.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=background)
    inner = max(1, round(size * scale))
    logo = render(square_svg(), inner, inner)
    offset = (size - inner) // 2
    canvas.alpha_composite(logo, (offset, offset))
    return canvas


def silhouette(img: Image.Image) -> Image.Image:
    """White glyph keeping only the alpha channel (Android status bar / macOS template)."""
    alpha = img.getchannel("A")
    white = Image.new("L", img.size, 255)
    return Image.merge("LA", (white, alpha))


def save_ico(path: Path, sizes):
    frames = [logo_on_canvas(s, 1.0) for s in sizes]
    largest = frames[-1]
    largest.save(path, format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])


def wordmark(text_color) -> Image.Image:
    """Logo + product name, 600x120 (displayed at max 300x60 by the app)."""
    width, height = 600, 120
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    icon = render(square_svg(), height, height)
    canvas.alpha_composite(icon, (0, 0))
    font = ImageFont.truetype(str(FONT), 58)
    draw = ImageDraw.Draw(canvas)
    left, top, right, bottom = draw.textbbox((0, 0), PRODUCT_NAME, font=font)
    x = height + 18
    y = (height - (bottom - top)) // 2 - top
    draw.text((x, y), PRODUCT_NAME, font=font, fill=text_color)
    used = x + (right - left) + 4
    return canvas.crop((0, 0, min(width, used), height))


def main():
    res = ROOT / "res"
    flutter_assets = ROOT / "flutter" / "assets"
    android_res = ROOT / "flutter" / "android" / "app" / "src" / "main" / "res"

    # Windows / shared resources
    save_ico(res / "icon.ico", ICO_SIZES)
    save_ico(res / "tray-icon.ico", TRAY_SIZES)
    save_ico(ROOT / "flutter" / "windows" / "runner" / "resources" / "app_icon.ico", ICO_SIZES)
    logo_on_canvas(1024, 0.86).save(res / "icon.png")
    logo_on_canvas(1024, 0.70, background=(255, 255, 255, 255)).save(res / "mac-icon.png")
    for size, name in [(32, "32x32.png"), (64, "64x64.png"), (128, "128x128.png"), (256, "128x128@2x.png")]:
        logo_on_canvas(size, 0.94).save(res / name)
    silhouette(logo_on_canvas(60, 0.9)).save(res / "mac-tray-dark-x2.png")
    silhouette(logo_on_canvas(48, 0.9)).save(res / "mac-tray-light-x2.png")
    (res / "scalable.svg").write_text(square_svg(), encoding="utf-8")
    (res / "logo.svg").write_text(square_svg(), encoding="utf-8")

    # Flutter UI assets
    (flutter_assets / "icon.svg").write_text(square_svg(), encoding="utf-8")
    logo_on_canvas(256, 0.94).save(flutter_assets / "icon.png")
    wordmark(TEXT_DARK).save(flutter_assets / "logo_light.png")
    wordmark(TEXT_LIGHT).save(flutter_assets / "logo_dark.png")
    wordmark(TEXT_DARK).save(flutter_assets / "logo.png")

    # Android launcher / notification icons
    for density, launcher, foreground, notification in ANDROID_DENSITIES:
        folder = android_res / f"mipmap-{density}"
        white = (255, 255, 255, 255)
        logo_on_canvas(launcher, 0.62, background=white).save(folder / "ic_launcher.png")
        logo_on_canvas(launcher, 0.58, background=white, round_bg=True).save(folder / "ic_launcher_round.png")
        # Adaptive icons: 108dp canvas, only the central 66dp is guaranteed visible.
        logo_on_canvas(foreground, 0.46).save(folder / "ic_launcher_foreground.png")
        silhouette(logo_on_canvas(notification, 0.92)).save(folder / "ic_stat_logo.png")

    print("Icons regenerated from branding/logo.svg")


if __name__ == "__main__":
    main()
