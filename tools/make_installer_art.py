"""Generate the WiX banner/dialog bitmaps for the Nexulon Switcher installer.

Outputs (relative to the installer folder):
  banner.bmp  493x58   top strip used by every WixUI dialog
  dialog.bmp  493x312  dialog background (dark brand panel + light content area)
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).resolve().parent.parent / "installer"

BLUE_TOP = (99, 210, 255)
BLUE_MID = (29, 125, 255)
BLUE_DEEP = (10, 59, 216)
STEEL = (223, 231, 243)
INK = (4, 6, 10)
PANEL = (10, 16, 26)


def font(size: int, bold: bool = True) -> ImageFont.FreeTypeFont:
    name = "segoeuib.ttf" if bold else "segoeui.ttf"
    for root in (r"C:\Windows\Fonts",):
        path = Path(root) / name
        if path.is_file():
            return ImageFont.truetype(str(path), size)
    return ImageFont.load_default()


def vertical_gradient(size: tuple[int, int], top: tuple[int, int, int], bottom: tuple[int, int, int]) -> Image.Image:
    width, height = size
    image = Image.new("RGB", size)
    draw = ImageDraw.Draw(image)
    for y in range(height):
        t = y / max(1, height - 1)
        color = tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3))
        draw.line([(0, y), (width, y)], fill=color)
    return image


def draw_mark(image: Image.Image, x: float, y: float, size: float) -> None:
    """Draw the Nexulon N mark at (x, y) with the given box size."""
    scale = size / 100.0
    draw = ImageDraw.Draw(image)

    def pts(raw: list[tuple[float, float]]) -> list[tuple[float, float]]:
        return [(x + px * scale, y + py * scale) for px, py in raw]

    left = [(20, 34), (36, 25.5), (36, 83), (20, 91.5)]
    right = [(64, 8.5), (80, 0), (80, 57.5), (64, 66)]
    bar = [(20, 34), (36, 25.5), (80, 66), (64, 74.5)]

    draw.polygon(pts(left), fill=BLUE_MID)
    draw.polygon(pts(right), fill=BLUE_MID)
    draw.polygon(pts(bar), fill=STEEL)


def spaced_text(draw: ImageDraw.ImageDraw, xy: tuple[int, int], text: str, fnt, fill, spacing: int) -> int:
    x, y = xy
    for char in text:
        draw.text((x, y), char, font=fnt, fill=fill)
        x += draw.textlength(char, font=fnt) + spacing
    return x


def make_banner() -> None:
    width, height = 493, 58
    image = vertical_gradient((width, height), (10, 16, 27), (5, 8, 13))
    draw = ImageDraw.Draw(image)

    # Electric underline.
    for x in range(width):
        t = x / (width - 1)
        alpha = max(0.0, 1.0 - abs(t - 0.22) * 2.4)
        color = tuple(round(INK[i] + (BLUE_MID[i] - INK[i]) * alpha * 0.8) for i in range(3))
        draw.line([(x, height - 2), (x, height - 1)], fill=color)

    draw_mark(image, 18, 12, 34)
    spaced_text(draw, (62, 13), "NEXULON", font(15, True), (236, 243, 252), 3)
    spaced_text(draw, (62, 32), "SWITCHER", font(9, False), (120, 138, 162), 4)

    image.save(HERE / "banner.bmp")


def make_dialog() -> None:
    width, height = 493, 312
    image = Image.new("RGB", (width, height), (242, 246, 252))
    draw = ImageDraw.Draw(image)

    # Dark brand panel on the left.
    panel_width = 186
    gradient = vertical_gradient((panel_width, height), (8, 13, 22), (4, 6, 10))
    image.paste(gradient, (0, 0))

    draw_mark(image, 43, 66, 100)

    spaced_text(draw, (30, 186), "NEXULON", font(18, True), (240, 246, 255), 3)
    spaced_text(draw, (32, 212), "SWITCHER", font(10, False), (120, 142, 172), 4)

    # Divider between the panel and the light content area.
    draw.line([(panel_width, 0), (panel_width, height)], fill=(20, 40, 72))
    draw.line([(panel_width + 1, 0), (panel_width + 1, height)], fill=(215, 226, 241))

    image.save(HERE / "dialog.bmp")


def main() -> None:
    HERE.mkdir(parents=True, exist_ok=True)
    make_banner()
    make_dialog()
    print("wrote banner.bmp and dialog.bmp")


if __name__ == "__main__":
    main()
