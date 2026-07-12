#!/usr/bin/env python3
"""Replace an exact sprite background color with transparency."""

from __future__ import annotations

import argparse
import binascii
from collections import Counter
import re
import struct
import sys
import zlib
from pathlib import Path


APP_ROOT = Path(__file__).resolve().parent.parent
ASSETS_ROOT = APP_ROOT / "assets"
NUMBERED_PNG = re.compile(r"^(\d{1,3}).*\.png$", re.IGNORECASE)
BACK_MARKER = "\ufffd" * 4
SOURCE_BUCKET = re.compile(r"^\d{3}-\d{3}$")
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def parse_color(value: str) -> tuple[int, int, int]:
    color = value.removeprefix("#")
    if not re.fullmatch(r"[0-9a-fA-F]{6}", color):
        raise argparse.ArgumentTypeError("color must use RRGGBB or #RRGGBB format")
    return tuple(int(color[index : index + 2], 16) for index in (0, 2, 4))


def find_default_source() -> Path:
    matches = sorted(
        path
        for path in ASSETS_ROOT.iterdir()
        if path.is_dir() and "493全PM64×64单帧素材" in path.name
    )
    if len(matches) != 1:
        raise RuntimeError(
            "could not uniquely locate the sprite source; pass it with --source"
        )
    return matches[0]


def collect_inputs(
    source: Path,
    side: str,
    start: int,
    count: int | None,
) -> list[tuple[int | None, Path]]:
    candidates: list[tuple[int | None, Path]] = []
    for bucket in source.iterdir():
        if not bucket.is_dir() or not SOURCE_BUCKET.fullmatch(bucket.name):
            continue
        for path in bucket.iterdir():
            if not path.is_file():
                continue
            if path.suffix.lower() != ".png":
                continue
            inferred_side = "back" if BACK_MARKER in path.stem else "front"
            if inferred_side != side:
                continue
            match = NUMBERED_PNG.fullmatch(path.name)
            if match is not None:
                candidates.append((int(match.group(1)), path))
            elif count is None:
                candidates.append((None, path))

    candidates.sort(
        key=lambda item: (
            item[0] is None,
            item[0] if item[0] is not None else 0,
            item[1].name,
        )
    )
    if count is None:
        return candidates

    expected = set(range(start, start + count))
    found: dict[int, Path] = {}
    for number, path in candidates:
        if number is None:
            continue
        if number not in expected:
            continue
        if number in found:
            raise RuntimeError(f"duplicate {side} sprite {number:03}: {path}")
        found[number] = path

    missing = sorted(expected - found.keys())
    if missing:
        names = ", ".join(f"{number:03}.png" for number in missing)
        raise RuntimeError(f"missing {side} sprites: {names}")
    return [(number, found[number]) for number in sorted(expected)]


def output_names(inputs: list[tuple[int | None, Path]]) -> list[str]:
    totals: dict[int | None, int] = {}
    seen: dict[int | None, int] = {}
    for number, _ in inputs:
        totals[number] = totals.get(number, 0) + 1

    names: list[str] = []
    for number, _ in inputs:
        seen[number] = seen.get(number, 0) + 1
        ordinal = seen[number]
        if number is None:
            names.append(f"unnumbered-{ordinal:02}.png")
        elif totals[number] == 1 or ordinal == 1:
            names.append(f"{number:03}.png")
        else:
            names.append(f"{number:03}-{ordinal:02}.png")
    return names


def pack_chunk(chunk_type: bytes, data: bytes) -> bytes:
    checksum = binascii.crc32(chunk_type + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + chunk_type + data + struct.pack(">I", checksum)


def paeth(left: int, up: int, upper_left: int) -> int:
    estimate = left + up - upper_left
    left_distance = abs(estimate - left)
    up_distance = abs(estimate - up)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= up_distance and left_distance <= upper_left_distance:
        return left
    if up_distance <= upper_left_distance:
        return up
    return upper_left


def decode_rows(
    compressed: bytes, width: int, height: int, bytes_per_pixel: int
) -> list[bytearray]:
    raw = zlib.decompress(compressed)
    stride = width * bytes_per_pixel
    expected_size = height * (stride + 1)
    if len(raw) != expected_size:
        raise RuntimeError(
            f"unexpected decompressed image size: expected {expected_size}, got {len(raw)}"
        )

    rows: list[bytearray] = []
    previous = bytearray(stride)
    offset = 0
    for _ in range(height):
        filter_type = raw[offset]
        offset += 1
        row = bytearray(raw[offset : offset + stride])
        offset += stride
        for column in range(stride):
            left = row[column - bytes_per_pixel] if column >= bytes_per_pixel else 0
            up = previous[column]
            upper_left = (
                previous[column - bytes_per_pixel]
                if column >= bytes_per_pixel
                else 0
            )
            if filter_type == 1:
                row[column] = (row[column] + left) & 0xFF
            elif filter_type == 2:
                row[column] = (row[column] + up) & 0xFF
            elif filter_type == 3:
                row[column] = (row[column] + ((left + up) // 2)) & 0xFF
            elif filter_type == 4:
                row[column] = (row[column] + paeth(left, up, upper_left)) & 0xFF
            elif filter_type != 0:
                raise RuntimeError(f"unsupported PNG filter type: {filter_type}")
        rows.append(row)
        previous = row
    return rows


def make_background_transparent(
    source: Path,
    destination: Path,
    background: tuple[int, int, int],
) -> tuple[int, int]:
    content = source.read_bytes()
    if not content.startswith(PNG_SIGNATURE):
        raise RuntimeError(f"not a PNG file: {source}")

    chunks: list[tuple[bytes, bytes]] = []
    offset = len(PNG_SIGNATURE)
    while offset < len(content):
        if offset + 12 > len(content):
            raise RuntimeError(f"truncated PNG chunk: {source}")
        length = struct.unpack(">I", content[offset : offset + 4])[0]
        chunk_type = content[offset + 4 : offset + 8]
        data_start = offset + 8
        data_end = data_start + length
        if data_end + 4 > len(content):
            raise RuntimeError(f"truncated PNG chunk data: {source}")
        data = content[data_start:data_end]
        expected_crc = struct.unpack(">I", content[data_end : data_end + 4])[0]
        actual_crc = binascii.crc32(chunk_type + data) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise RuntimeError(f"invalid PNG checksum in {chunk_type!r}: {source}")
        chunks.append((chunk_type, data))
        offset = data_end + 4
        if chunk_type == b"IEND":
            break

    chunk_map = {chunk_type: data for chunk_type, data in chunks}
    try:
        width, height, bit_depth, color_type, _, _, interlace = struct.unpack(
            ">IIBBBBB", chunk_map[b"IHDR"]
        )
    except (KeyError, struct.error) as error:
        raise RuntimeError(f"missing or invalid PNG metadata: {source}") from error

    if bit_depth != 8 or color_type not in {3, 6} or interlace != 0:
        raise RuntimeError(
            "expected non-interlaced 8-bit indexed or RGBA PNG, got "
            f"bit_depth={bit_depth}, color_type={color_type}, interlace={interlace}: "
            f"{source}"
        )

    compressed = b"".join(data for chunk_type, data in chunks if chunk_type == b"IDAT")
    rgba_rows: list[bytes] = []
    replaced_pixels = 0
    if color_type == 3:
        try:
            palette = chunk_map[b"PLTE"]
        except KeyError as error:
            raise RuntimeError(f"indexed PNG has no palette: {source}") from error
        colors = [
            tuple(palette[index : index + 3])
            for index in range(0, len(palette), 3)
        ]
        indexed_rows = decode_rows(compressed, width, height, 1)
        transparent_indexes = {
            index for index, color in enumerate(colors) if color == background
        }
        border_indexes = (
            list(indexed_rows[0])
            + list(indexed_rows[-1])
            + [row[0] for row in indexed_rows[1:-1]]
            + [row[-1] for row in indexed_rows[1:-1]]
        )
        border_index, frequency = Counter(border_indexes).most_common(1)[0]
        if frequency * 2 >= len(border_indexes):
            transparent_indexes.add(border_index)
        alpha = bytearray([255] * len(colors))
        existing_alpha = chunk_map.get(b"tRNS", b"")
        alpha[: len(existing_alpha)] = existing_alpha
        for row in indexed_rows:
            rgba = bytearray()
            for palette_index in row:
                red, green, blue = colors[palette_index]
                if palette_index in transparent_indexes:
                    rgba.extend((0, 0, 0, 0))
                    replaced_pixels += 1
                else:
                    rgba.extend((red, green, blue, alpha[palette_index]))
            rgba_rows.append(b"\x00" + bytes(rgba))
    else:
        decoded_rgba_rows = decode_rows(compressed, width, height, 4)
        transparent_colors = {background}
        border_colors = []
        for row_index, row in enumerate(decoded_rgba_rows):
            colors_in_row = [
                tuple(row[offset : offset + 3])
                for offset in range(0, len(row), 4)
            ]
            if row_index in {0, len(decoded_rgba_rows) - 1}:
                border_colors.extend(colors_in_row)
            else:
                border_colors.extend((colors_in_row[0], colors_in_row[-1]))
        border_color, frequency = Counter(border_colors).most_common(1)[0]
        if frequency * 2 >= len(border_colors):
            transparent_colors.add(border_color)
        for row in decoded_rgba_rows:
            rgba = bytearray(row)
            for offset in range(0, len(rgba), 4):
                if tuple(rgba[offset : offset + 3]) in transparent_colors:
                    rgba[offset : offset + 4] = b"\x00\x00\x00\x00"
                    replaced_pixels += 1
            rgba_rows.append(b"\x00" + bytes(rgba))

    if width == 65:
        if any(row[-4:] != b"\x00\x00\x00\x00" for row in rgba_rows):
            raise RuntimeError(f"cannot crop non-transparent column from 65px image: {source}")
        rgba_rows = [row[:-4] for row in rgba_rows]
        width = 64
    if width != 64 or height % 64 != 0:
        raise RuntimeError(
            f"cannot normalize {width}x{height} image into 64x64 frames: {source}"
        )

    destination.parent.mkdir(parents=True, exist_ok=True)
    frame_count = height // 64
    rgba_header = struct.pack(">IIBBBBB", 64, 64, 8, 6, 0, 0, 0)
    for frame_index in range(frame_count):
        frame_rows = rgba_rows[frame_index * 64 : (frame_index + 1) * 64]
        rgba_image = zlib.compress(b"".join(frame_rows), level=9)
        output_chunks = (
            (b"IHDR", rgba_header),
            (b"IDAT", rgba_image),
            (b"IEND", b""),
        )
        if frame_count == 1:
            frame_path = destination
        else:
            frame_path = destination.with_name(
                f"{destination.stem}-frame-{frame_index + 1:02}.png"
            )
        frame_path.write_bytes(
            PNG_SIGNATURE
            + b"".join(
                pack_chunk(chunk_type, data) for chunk_type, data in output_chunks
            )
        )
    return replaced_pixels, frame_count


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, help="source sprite directory")
    parser.add_argument(
        "--output",
        type=Path,
        help="output root (default: assets/pokemons)",
    )
    parser.add_argument("--background", type=parse_color, default=parse_color("#F8B8F8"))
    parser.add_argument("--start", type=int, default=1)
    parser.add_argument("--count", type=int, default=30)
    parser.add_argument(
        "--all",
        action="store_true",
        help="process every PNG in the numbered source directories",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.start < 0 or args.count < 1:
        raise RuntimeError("--start must be non-negative and --count must be positive")

    source = args.source.resolve() if args.source else find_default_source()
    if not source.is_dir():
        raise RuntimeError(f"source directory does not exist: {source}")
    output = args.output.resolve() if args.output else ASSETS_ROOT / "pokemons"

    total_replaced = 0
    processed_sources = 0
    written_images = 0
    for side in ("front", "back"):
        inputs = collect_inputs(
            source, side, args.start, None if args.all else args.count
        )
        for (_, input_path), output_name in zip(inputs, output_names(inputs)):
            output_path = output / side / output_name
            replaced, frame_count = make_background_transparent(
                input_path, output_path, args.background
            )
            total_replaced += replaced
            processed_sources += 1
            written_images += frame_count
            print(f"{side}/{input_path.name} -> {side}/{output_path.name}")

    print(
        f"Processed {processed_sources} sources into {written_images} sprites at {output} "
        f"({total_replaced} pixels made transparent)."
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
