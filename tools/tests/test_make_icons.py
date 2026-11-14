"""Tests for the icon generator.

The generator exists so the `.ico` and the `.png` cannot drift, so the tests check the
properties a build depends on: the sizes, the alpha at the corners, and that both files
decode to the same pixels.
"""

from __future__ import annotations

import struct
import zlib
from pathlib import Path

import pytest

from tools import make_icons


def test_pixel_is_transparent_outside_the_mark() -> None:
    assert make_icons.pixel(0, 0)[3] == 0
    assert make_icons.pixel(make_icons.SIZE - 1, make_icons.SIZE - 1)[3] == 0


def test_pixel_is_opaque_inside_the_mark() -> None:
    assert make_icons.pixel(16, 16)[3] == 255


def test_pixel_uses_the_glyph_colour_on_the_bars() -> None:
    assert make_icons.pixel(16, 12) == (*make_icons.GLYPH, 255)
    assert make_icons.pixel(16, 19) == (*make_icons.GLYPH, 255)


def test_pixel_uses_the_accent_colour_between_the_bars() -> None:
    assert make_icons.pixel(16, 16) == (*make_icons.ACCENT, 255)


def test_pixel_is_deterministic() -> None:
    assert make_icons.pixel(7, 9) == make_icons.pixel(7, 9)


def test_rows_has_one_entry_per_pixel() -> None:
    rows = make_icons.rows()

    assert len(rows) == make_icons.SIZE
    assert all(len(row) == make_icons.SIZE for row in rows)


def test_rows_starts_at_the_top_of_the_image() -> None:
    rows = make_icons.rows()

    assert rows[0][0][3] == 0
    assert rows[make_icons.SIZE // 2][make_icons.SIZE // 2][3] == 255


def test_write_ico_produces_a_single_image_icon(tmp_path: Path) -> None:
    path = tmp_path / "icon.ico"

    make_icons.write_ico(path)

    data = path.read_bytes()
    reserved, kind, count = struct.unpack("<HHH", data[:6])
    assert (reserved, kind, count) == (0, 1, 1)
    width, height, colours, _, planes, bits, length, offset = struct.unpack(
        "<BBBBHHII", data[6:22]
    )
    assert (width, height, colours) == (make_icons.SIZE, make_icons.SIZE, 0)
    assert (planes, bits) == (1, 32)
    assert offset + length == len(data)


def test_write_ico_stores_rows_bottom_up(tmp_path: Path) -> None:
    path = tmp_path / "icon.ico"

    make_icons.write_ico(path)

    data = path.read_bytes()
    offset = struct.unpack("<I", data[18:22])[0]
    header = struct.unpack("<IiiHHIIiiII", data[offset : offset + 40])
    assert header[0] == 40
    assert header[1] == make_icons.SIZE
    assert header[2] == make_icons.SIZE * 2
    first_pixel = data[offset + 40 : offset + 44]
    assert first_pixel[3] == 0, "the first stored row is the bottom row, which is transparent"


def test_write_png_produces_a_decodable_image(tmp_path: Path) -> None:
    path = tmp_path / "icon.png"

    make_icons.write_png(path)

    data = path.read_bytes()
    assert data.startswith(b"\x89PNG\r\n\x1a\n")
    width, height, depth, colour = struct.unpack(">IIBB", data[16:26])
    assert (width, height, depth, colour) == (make_icons.SIZE, make_icons.SIZE, 8, 6)


def test_write_png_round_trips_the_pixels(tmp_path: Path) -> None:
    path = tmp_path / "icon.png"

    make_icons.write_png(path)

    data = path.read_bytes()
    idat_start = data.index(b"IDAT") - 4
    idat_length = struct.unpack(">I", data[idat_start : idat_start + 4])[0]
    raw = zlib.decompress(data[idat_start + 8 : idat_start + 8 + idat_length])
    stride = 1 + make_icons.SIZE * 4
    assert len(raw) == stride * make_icons.SIZE
    assert all(raw[row * stride] == 0 for row in range(make_icons.SIZE))
    first = tuple(raw[1:5])
    assert first == make_icons.pixel(0, 0)


def test_write_png_and_write_ico_agree_on_the_middle_pixel(tmp_path: Path) -> None:
    png = tmp_path / "icon.png"
    ico = tmp_path / "icon.ico"

    make_icons.write_png(png)
    make_icons.write_ico(ico)

    data = png.read_bytes()
    idat_start = data.index(b"IDAT") - 4
    idat_length = struct.unpack(">I", data[idat_start : idat_start + 4])[0]
    raw = zlib.decompress(data[idat_start + 8 : idat_start + 8 + idat_length])
    stride = 1 + make_icons.SIZE * 4
    middle = make_icons.SIZE // 2
    offset = middle * stride + 1 + middle * 4
    assert tuple(raw[offset : offset + 4]) == make_icons.pixel(middle, middle)

    ico_data = ico.read_bytes()
    ico_offset = struct.unpack("<I", ico_data[18:22])[0]
    row_from_bottom = make_icons.SIZE - 1 - middle
    pixel_offset = ico_offset + 40 + (row_from_bottom * make_icons.SIZE + middle) * 4
    blue, green, red, alpha = ico_data[pixel_offset : pixel_offset + 4]
    assert (red, green, blue, alpha) == make_icons.pixel(middle, middle)


def test_main_writes_both_files(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(make_icons, "__file__", str(tmp_path / "tools" / "make_icons.py"))

    assert make_icons.main() == 0

    icons = tmp_path / "apps" / "desktop" / "src-tauri" / "icons"
    assert (icons / "icon.ico").is_file()
    assert (icons / "icon.png").is_file()


def test_main_is_idempotent(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(make_icons, "__file__", str(tmp_path / "tools" / "make_icons.py"))

    make_icons.main()
    icons = tmp_path / "apps" / "desktop" / "src-tauri" / "icons"
    first = (icons / "icon.ico").read_bytes()
    make_icons.main()

    assert (icons / "icon.ico").read_bytes() == first
