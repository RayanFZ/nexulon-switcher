"""Build the Nexulon app icon from ``assets/icon-source.svg``.

The vector is rasterised with the Edge/Chromium build that ships with Windows,
so the repo needs no SVG rasteriser. The resulting PNG, multi-size ICO and raw
RGBA feed the window icon, the executable icon and the in-app header.

    python tools/make_icon.py
"""

from __future__ import annotations

import io
import struct
import subprocess
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
SOURCE = ASSETS / "icon-source.svg"
SOURCE_SMALL = ASSETS / "icon-small.svg"
SIZE = 512
SCALE = 2

EDGE_CANDIDATES = [
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
]


def find_edge() -> str:
    for candidate in EDGE_CANDIDATES:
        if Path(candidate).exists():
            return candidate
    raise SystemExit("Microsoft Edge not found; it is needed to rasterise the SVG.")


def render(source: Path) -> Image.Image:
    edge = find_edge()
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "icon.png"
        profile = Path(tmp) / "profile"
        result = subprocess.run(
            [
                edge,
                "--headless=new",
                "--disable-gpu",
                "--hide-scrollbars",
                f"--user-data-dir={profile}",
                f"--force-device-scale-factor={SCALE}",
                f"--window-size={SIZE},{SIZE}",
                "--default-background-color=00000000",
                f"--screenshot={out}",
                source.as_uri(),
            ],
            capture_output=True,
        )
        if not out.exists():
            raise SystemExit(
                "Edge did not produce a screenshot:\n"
                + result.stderr.decode("utf-8", "replace")
            )
        return Image.open(out).convert("RGBA")


def write_ico(path: Path, frames: list[Image.Image]) -> None:
    """Write a multi-size ICO with one PNG frame per size.

    Pillow's own ICO writer regenerates every size from a single image, which
    would throw away the heavier compact frames used at 16-32 px.
    """
    blobs = []
    for frame in frames:
        buffer = io.BytesIO()
        frame.save(buffer, format="PNG", optimize=True)
        blobs.append(buffer.getvalue())

    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = 6 + 16 * len(frames)
    entries = b""
    for frame, blob in zip(frames, blobs):
        side = frame.width
        entries += struct.pack(
            "<BBBBHHII",
            0 if side >= 256 else side,
            0 if side >= 256 else side,
            0,
            0,
            1,
            32,
            len(blob),
            offset,
        )
        offset += len(blob)
    path.write_bytes(header + entries + b"".join(blobs))


def main() -> None:
    master = render(SOURCE).resize((SIZE * SCALE, SIZE * SCALE), Image.LANCZOS)
    # 16-32 px get a heavier stroke and no glow so the mark stays legible.
    compact = render(SOURCE_SMALL).resize((SIZE * SCALE, SIZE * SCALE), Image.LANCZOS)

    master.resize((SIZE, SIZE), Image.LANCZOS).save(ASSETS / "nexulon.png")
    (ASSETS / "icon.rgba").write_bytes(
        master.resize((256, 256), Image.LANCZOS).tobytes()
    )

    sizes = [16, 24, 32, 48, 64, 128, 256]
    frames = [
        (compact if size <= 32 else master).resize((size, size), Image.LANCZOS)
        for size in sizes
    ]
    write_ico(ASSETS / "nexulon.ico", frames)

    for name in ("nexulon.png", "nexulon.ico", "icon.rgba"):
        path = ASSETS / name
        print(f"wrote {path} ({path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
