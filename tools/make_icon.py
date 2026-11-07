"""Generate the Nexulon icon (.ico + raw RGBA) without any SVG rasteriser.

The mark is drawn as three quads with simple scanline gradients, matching
web/logo.svg closely enough to read as the same logo at 16-256 px.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw

OUT = Path(__file__).resolve().parent.parent / "assets"
SIZE = 256


def lerp(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(len(a)))


def fill_quad(draw, pts, top, bottom, x_gradient=False):
    ys = [p[1] for p in pts]
    y0, y1 = min(ys), max(ys)
    n = len(pts)
    for y in range(int(y0), int(y1) + 1):
        xs = []
        for i in range(n):
            ax, ay = pts[i]
            bx, by = pts[(i + 1) % n]
            if ay == by:
                continue
            if min(ay, by) <= y < max(ay, by):
                t = (y - ay) / (by - ay)
                xs.append(ax + t * (bx - ax))
        if len(xs) < 2:
            continue
        xs.sort()
        xa, xb = int(xs[0]), int(xs[-1])
        if xb <= xa:
            continue
        if x_gradient:
            for x in range(xa, xb):
                t = (x - xa) / max(1, (xb - xa))
                draw.point((x, y), fill=lerp(top, bottom, t))
        else:
            t = (y - y0) / max(1, (y1 - y0))
            draw.line([(xa, y), (xb, y)], fill=lerp(top, bottom, t))


def scale(pts, size):
    k = size / 100.0
    return [(x * k, y * k) for x, y in pts]


def render(size: int) -> Image.Image:
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = size * 0.055
    draw.rounded_rectangle(
        [pad, pad, size - pad, size - pad],
        radius=int(size * 0.22),
        fill=(8, 10, 14, 255),
    )

    left = [(20, 34), (36, 25.5), (36, 83), (20, 91.5)]
    right = [(64, 8.5), (80, 0), (80, 57.5), (64, 66)]
    beam = [(20, 34), (36, 25.5), (80, 66), (64, 74.5)]

    blue_top, blue_bottom = (76, 201, 255), (10, 59, 216)
    steel_left, steel_right = (255, 255, 255), (34, 71, 214)

    # shrink into the rounded tile
    inner = size * 0.78
    offset = (size - inner) / 2
    to_px = lambda pts: [(offset + x * inner / 100, offset + y * inner / 100) for x, y in pts]

    fill_quad(draw, to_px(left), blue_top, blue_bottom)
    fill_quad(draw, to_px(right), blue_top, blue_bottom)
    fill_quad(draw, to_px(beam), steel_left, steel_right, x_gradient=True)
    return img


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    master = render(SIZE)

    # Raw RGBA for the runtime window icon.
    (OUT / "icon.rgba").write_bytes(master.tobytes())

    # Multi-size .ico for the executable.
    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [render(s) for s in sizes]
    frames[-1].save(
        OUT / "nexulon.ico",
        format="ICO",
        sizes=[(s, s) for s in sizes],
        append_images=frames[:-1],
    )
    master.save(OUT / "nexulon.png")
    print(f"wrote {OUT / 'nexulon.ico'} ({ (OUT / 'nexulon.ico').stat().st_size } bytes)")


if __name__ == "__main__":
    main()
