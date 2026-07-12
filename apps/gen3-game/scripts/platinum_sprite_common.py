"""PNG decoding and normalization helpers for Platinum sprite assets."""

from __future__ import annotations

import binascii
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import struct
import zlib


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


@dataclass
class DecodedPng:
    width: int
    height: int
    bit_depth: int
    color_type: int
    rows: list[bytearray]
    background: tuple[int, int, int, int] | None
    background_ratio: float
    transparent_pixels: int


def _read_chunks(path: Path) -> list[tuple[bytes, bytes]]:
    content = path.read_bytes()
    if not content.startswith(PNG_SIGNATURE):
        raise ValueError("not a PNG file")

    chunks: list[tuple[bytes, bytes]] = []
    offset = len(PNG_SIGNATURE)
    while offset < len(content):
        if offset + 12 > len(content):
            raise ValueError("truncated PNG chunk")
        length = struct.unpack(">I", content[offset : offset + 4])[0]
        chunk_type = content[offset + 4 : offset + 8]
        data_start = offset + 8
        data_end = data_start + length
        if data_end + 4 > len(content):
            raise ValueError("truncated PNG chunk data")
        data = content[data_start:data_end]
        expected_crc = struct.unpack(">I", content[data_end : data_end + 4])[0]
        actual_crc = binascii.crc32(chunk_type + data) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise ValueError(f"invalid checksum in {chunk_type.decode('ascii', 'replace')}")
        chunks.append((chunk_type, data))
        offset = data_end + 4
        if chunk_type == b"IEND":
            return chunks
    raise ValueError("missing IEND chunk")


def _paeth(left: int, up: int, upper_left: int) -> int:
    estimate = left + up - upper_left
    distances = (
        abs(estimate - left),
        abs(estimate - up),
        abs(estimate - upper_left),
    )
    if distances[0] <= distances[1] and distances[0] <= distances[2]:
        return left
    return up if distances[1] <= distances[2] else upper_left


def _decode_scanlines(
    compressed: bytes,
    stride: int,
    height: int,
    filter_bytes_per_pixel: int,
) -> list[bytearray]:
    raw = zlib.decompress(compressed)
    expected = height * (stride + 1)
    if len(raw) != expected:
        raise ValueError(f"expected {expected} decompressed bytes, got {len(raw)}")

    rows: list[bytearray] = []
    previous = bytearray(stride)
    offset = 0
    for _ in range(height):
        filter_type = raw[offset]
        offset += 1
        row = bytearray(raw[offset : offset + stride])
        offset += stride
        for column in range(stride):
            left = (
                row[column - filter_bytes_per_pixel]
                if column >= filter_bytes_per_pixel
                else 0
            )
            up = previous[column]
            upper_left = (
                previous[column - filter_bytes_per_pixel]
                if column >= filter_bytes_per_pixel
                else 0
            )
            if filter_type == 1:
                row[column] = (row[column] + left) & 0xFF
            elif filter_type == 2:
                row[column] = (row[column] + up) & 0xFF
            elif filter_type == 3:
                row[column] = (row[column] + ((left + up) // 2)) & 0xFF
            elif filter_type == 4:
                row[column] = (row[column] + _paeth(left, up, upper_left)) & 0xFF
            elif filter_type != 0:
                raise ValueError(f"unsupported PNG filter type {filter_type}")
        rows.append(row)
        previous = row
    return rows


def _indexed_rows(
    chunks: dict[bytes, bytes],
    compressed: bytes,
    width: int,
    height: int,
    bit_depth: int,
) -> list[bytearray]:
    if bit_depth not in {4, 8}:
        raise ValueError(f"unsupported indexed bit depth {bit_depth}")
    palette = chunks.get(b"PLTE")
    if palette is None or len(palette) % 3:
        raise ValueError("missing or invalid palette")
    colors = [tuple(palette[i : i + 3]) for i in range(0, len(palette), 3)]
    alpha = bytearray([255] * len(colors))
    transparency = chunks.get(b"tRNS", b"")
    alpha[: len(transparency)] = transparency

    stride = (width * bit_depth + 7) // 8
    packed_rows = _decode_scanlines(compressed, stride, height, 1)
    rgba_rows: list[bytearray] = []
    for packed in packed_rows:
        if bit_depth == 8:
            indexes = list(packed[:width])
        else:
            indexes = [value for byte in packed for value in (byte >> 4, byte & 15)][:width]
        rgba = bytearray()
        for index in indexes:
            if index >= len(colors):
                raise ValueError(f"palette index {index} is out of range")
            rgba.extend((*colors[index], alpha[index]))
        rgba_rows.append(rgba)
    return rgba_rows


def read_png(path: Path) -> DecodedPng:
    chunk_list = _read_chunks(path)
    chunks = dict(chunk_list)
    try:
        width, height, bit_depth, color_type, compression, filtering, interlace = (
            struct.unpack(">IIBBBBB", chunks[b"IHDR"])
        )
    except (KeyError, struct.error) as error:
        raise ValueError("missing or invalid IHDR chunk") from error
    if (compression, filtering, interlace) != (0, 0, 0):
        raise ValueError("only non-interlaced PNG files are supported")

    compressed = b"".join(data for kind, data in chunk_list if kind == b"IDAT")
    if color_type == 3:
        rows = _indexed_rows(chunks, compressed, width, height, bit_depth)
    elif color_type == 2 and bit_depth == 8:
        rgb_rows = _decode_scanlines(compressed, width * 3, height, 3)
        rows = []
        for rgb_row in rgb_rows:
            rgba = bytearray()
            for offset in range(0, len(rgb_row), 3):
                rgba.extend((*rgb_row[offset : offset + 3], 255))
            rows.append(rgba)
    elif color_type == 6 and bit_depth == 8:
        rows = _decode_scanlines(compressed, width * 4, height, 4)
    else:
        raise ValueError(f"unsupported PNG format: depth={bit_depth}, type={color_type}")

    pixels = [tuple(row[i : i + 4]) for row in rows for i in range(0, len(row), 4)]
    transparent_pixels = sum(pixel[3] == 0 for pixel in pixels)
    border = (
        [tuple(rows[0][i : i + 4]) for i in range(0, len(rows[0]), 4)]
        + [tuple(rows[-1][i : i + 4]) for i in range(0, len(rows[-1]), 4)]
        + [tuple(row[:4]) for row in rows[1:-1]]
        + [tuple(row[-4:]) for row in rows[1:-1]]
    )
    background, count = Counter(border).most_common(1)[0]
    ratio = count / len(border)
    if background[3] == 0 or ratio < 0.5:
        background = None

    return DecodedPng(
        width=width,
        height=height,
        bit_depth=bit_depth,
        color_type=color_type,
        rows=rows,
        background=background,
        background_ratio=ratio,
        transparent_pixels=transparent_pixels,
    )


def normalize_frames(image: DecodedPng) -> list[list[bytearray]]:
    rows = [bytearray(row) for row in image.rows]
    if image.background is not None:
        background_rgb = image.background[:3]
        for row in rows:
            for offset in range(0, len(row), 4):
                if tuple(row[offset : offset + 3]) == background_rgb:
                    row[offset : offset + 4] = b"\x00\x00\x00\x00"

    if (image.width, image.height) == (160, 80):
        return [
            [bytearray(row[start * 4 : (start + 80) * 4]) for row in rows]
            for start in (0, 80)
        ]
    if (image.width, image.height) == (80, 80):
        return [rows]
    if (image.width, image.height) == (64, 64):
        canvas = [bytearray(80 * 4) for _ in range(80)]
        for y, row in enumerate(rows, start=8):
            canvas[y][8 * 4 : 72 * 4] = row
        return [canvas]
    raise ValueError(f"cannot normalize {image.width}x{image.height} into 80x80 frames")


def _pack_chunk(chunk_type: bytes, data: bytes) -> bytes:
    crc = binascii.crc32(chunk_type + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + chunk_type + data + struct.pack(">I", crc)


def write_rgba_png(path: Path, rows: list[bytearray]) -> None:
    if not rows or not rows[0] or len(rows[0]) % 4:
        raise ValueError("output frame must contain RGBA pixels")
    width = len(rows[0]) // 4
    height = len(rows)
    if any(len(row) != width * 4 for row in rows):
        raise ValueError("output frame rows must have a consistent width")
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    image_data = zlib.compress(b"".join(b"\x00" + bytes(row) for row in rows), level=9)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        PNG_SIGNATURE
        + _pack_chunk(b"IHDR", header)
        + _pack_chunk(b"IDAT", image_data)
        + _pack_chunk(b"IEND", b"")
    )
