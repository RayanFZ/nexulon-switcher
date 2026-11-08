"""Generate the Nexulon Corp cyber-circuit icon (.ico + raw RGBA).

The mark is a neon "N" on a deep-navy tile with faint circuit traces, so it
reads at 16 px and matches the Circuit theme used by the interface.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).resolve().parent.parent / "assets"
SIZE = 256
SUPERSAMPLE = 4


def render(size: int) -> Image.Image:
    s = size
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Rounded navy tile.
    pad = s * 0.035
    radius = int(s * 0.20)
    draw.rounded_rectangle(
        [pad, pad, s - pad, s - pad],
        radius=radius,
        fill=(2, 11, 24, 255),
    )
    draw.rounded_rectangle(
        [pad, pad, s - pad, s - pad],
        radius=radius,
        outline=(0, 240, 255, 46),
        width=max(1, int(s * 0.006)),
    )

    # Faint circuit traces and nodes around the mark.
    trace = (0, 240, 255, 42)
    tw = max(1, int(s * 0.007))
    traces = [
        ((0.09, 0.30), (0.19, 0.30), (0.19, 0.22), (0.25, 0.22)),
        ((0.09, 0.70), (0.20, 0.70), (0.20, 0.80), (0.25, 0.80)),
        ((0.91, 0.26), (0.81, 0.26), (0.81, 0.20), (0.75, 0.20)),
        ((0.91, 0.74), (0.82, 0.74), (0.82, 0.80), (0.75, 0.80)),
        ((0.09, 0.50), (0.16, 0.50)),
        ((0.91, 0.50), (0.84, 0.50)),
    ]
    for points in traces:
        draw.line([(x * s, y * s) for x, y in points], fill=trace, width=tw)
    for x, y in [(0.09, 0.30), (0.09, 0.70), (0.91, 0.26), (0.91, 0.74), (0.09, 0.50), (0.91, 0.50)]:
        r = max(2, int(s * 0.011))
        draw.ellipse([x * s - r, y * s - r, x * s + r, y * s + r], fill=(0, 240, 255, 110))

    # Neon N: left stem, diagonal, right stem.
    left = [(0.26, 0.20), (0.38, 0.20), (0.38, 0.80), (0.26, 0.80)]
    diag = [(0.38, 0.20), (0.50, 0.20), (0.74, 0.80), (0.62, 0.80)]
    right = [(0.62, 0.20), (0.74, 0.20), (0.74, 0.80), (0.62, 0.80)]
    shapes = (left, diag, right)

    # Outer glow.
    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for shape in shapes:
        gd.polygon([(x * s, y * s) for x, y in shape], fill=(0, 240, 255, 190))
    glow = glow.filter(ImageFilter.GaussianBlur(radius=s * 0.028))
    img = Image.alpha_composite(img, glow)
    draw = ImageDraw.Draw(img)

    # Cyan-to-blue vertical gradient clipped to the N.
    mask = Image.new("L", (s, s), 0)
    md = ImageDraw.Draw(mask)
    for shape in shapes:
        md.polygon([(x * s, y * s) for x, y in shape], fill=255)
    gradient = Image.new("RGBA", (s, s))
    gdraw = ImageDraw.Draw(gradient)
    for y in range(s):
        t = y / max(1, s - 1)
        r = round(125 + (0 - 125) * t)
        g = round(252 + (168 - 252) * t)
        b = round(255 + (255 - 255) * t)
        gdraw.line([(0, y), (s, y)], fill=(r, g, b, 255))
    img.paste(gradient, (0, 0), mask)

    # Crisp neon edge.
    for shape in shapes:
        draw.polygon(
            [(x * s, y * s) for x, y in shape],
            outline=(180, 255, 255, 230),
            width=max(1, int(s * 0.004)),
        )
    return img


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    master = render(SIZE * SUPERSAMPLE).resize((SIZE, SIZE), Image.LANCZOS)

    # Raw RGBA for the runtime window icon.
    (OUT / "icon.rgba").write_bytes(master.tobytes())

    # Multi-size .ico for the executable.
    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [master.resize((s, s), Image.LANCZOS) for s in sizes]
    frames[-1].save(
        OUT / "nexulon.ico",
        format="ICO",
        sizes=[(s, s) for s in sizes],
        append_images=frames[:-1],
    )
    master.save(OUT / "nexulon.png")
    print(f"wrote {OUT / 'nexulon.ico'} ({(OUT / 'nexulon.ico').stat().st_size} bytes)")


if __name__ == "__main__":
    main()
