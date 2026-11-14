"""Generate the app icon set from one description, with no image library.

Tauri needs a Windows `.ico` at build time and a `.png` for the other bundles. Both are
derived here from the same 32x32 pixel function, so the two files cannot drift. The
palette is the theme's accent on the theme's surface, which keeps the icon legible on a
light and a dark taskbar.

Run `python tools/make_icons.py` after changing the palette; the output is committed
because a build must not depend on Python.
"""

from __future__ import annotations

import struct
from pathlib import Path

SIZE = 32
ACCENT = (0x2F, 0x6F, 0xED)
SURFACE = (0x11, 0x14, 0x1A)
GLYPH = (0xF5, 0xF7, 0xFA)


def pixel(x: int, y: int) -> tuple[int, int, int, int]:
    """Return the RGBA colour of one pixel.

    The mark is a rounded square in the accent colour with two white bars across the
    middle: strokes that read as a clapper board at 16 px. A pixel is inside the mark
    when it is within the inset square and within `radius` of the nearest corner centre,
    which rounds the four corners without a per-corner special case.
    """
    inset = 2
    radius = 6
    low = inset
    high = SIZE - inset - 1
    if not (low <= x <= high and low <= y <= high):
        return (0, 0, 0, 0)
    centre_x = min(max(x, low + radius), high - radius)
    centre_y = min(max(y, low + radius), high - radius)
    if (x - centre_x) ** 2 + (y - centre_y) ** 2 > radius * radius:
        return (0, 0, 0, 0)
    if 11 <= y <= 14 or 18 <= y <= 21:
        return (*GLYPH, 255)
    return (*ACCENT, 255)


def rows() -> list[list[tuple[int, int, int, int]]]:
    """The image as rows from the top, which is the order a PNG wants."""
    return [[pixel(x, y) for x in range(SIZE)] for y in range(SIZE)]


def write_ico(path: Path) -> None:
    """Write a single-image 32-bit ICO."""
    image = rows()
    header = struct.pack("<IiiHHIIiiII", 40, SIZE, SIZE * 2, 1, 32, 0, 0, 0, 0, 0, 0)
    body = bytearray()
    for y in range(SIZE - 1, -1, -1):
        for x in range(SIZE):
            r, g, b, a = image[y][x]
            body += bytes((b, g, r, a))
    mask_row = ((SIZE + 31) // 32) * 4
    mask = bytes(mask_row * SIZE)
    payload = header + bytes(body) + mask
    directory = struct.pack("<BBBBHHII", SIZE, SIZE, 0, 0, 1, 32, len(payload), 22)
    path.write_bytes(struct.pack("<HHH", 0, 1, 1) + directory + payload)


def write_png(path: Path) -> None:
    """Write an RGBA PNG with zlib, without a third-party encoder."""
    import zlib

    raw = bytearray()
    for row in rows():
        raw.append(0)
        for r, g, b, a in row:
            raw += bytes((r, g, b, a))

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def main() -> int:
    """Write both icons next to the Tauri manifest."""
    icons = Path(__file__).resolve().parent.parent / "apps" / "desktop" / "src-tauri" / "icons"
    icons.mkdir(parents=True, exist_ok=True)
    write_ico(icons / "icon.ico")
    write_png(icons / "icon.png")
    print(f"wrote {icons / 'icon.ico'} and {icons / 'icon.png'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
