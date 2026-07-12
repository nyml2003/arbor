#!/usr/bin/env python3
"""Extract deduplicated 16x16 tiles from an irregular map asset sheet."""

from __future__ import annotations

import argparse
from collections import deque
import hashlib
import json
import math
from pathlib import Path
import shutil

from platinum_sprite_common import read_png, write_rgba_png


APP_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = APP_ROOT / "assets" / "maps" / "25_47179"
DEFAULT_EXCLUSIONS = ["470,166,83,17"]


def parse_rectangle(value: str) -> tuple[int, int, int, int]:
    try:
        rectangle = tuple(int(part) for part in value.split(","))
    except ValueError as error:
        raise argparse.ArgumentTypeError("rectangle must be x,y,width,height") from error
    if len(rectangle) != 4 or any(part < 0 for part in rectangle):
        raise argparse.ArgumentTypeError("rectangle must contain four non-negative integers")
    if rectangle[2] == 0 or rectangle[3] == 0:
        raise argparse.ArgumentTypeError("rectangle width and height must be positive")
    return rectangle


def remove_external_background(
    rows: list[bytearray], preserve_enclosed: bool
) -> tuple[int, int, int]:
    width = len(rows[0]) // 4
    height = len(rows)
    background = tuple(rows[0][:3])
    outside: set[tuple[int, int]] = set()
    queue: deque[tuple[int, int]] = deque()

    def enqueue(x: int, y: int) -> None:
        if (x, y) in outside:
            return
        if tuple(rows[y][x * 4 : x * 4 + 3]) != background:
            return
        outside.add((x, y))
        queue.append((x, y))

    for x in range(width):
        enqueue(x, 0)
        enqueue(x, height - 1)
    for y in range(height):
        enqueue(0, y)
        enqueue(width - 1, y)
    while queue:
        x, y = queue.popleft()
        for neighbor_x, neighbor_y in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
            if 0 <= neighbor_x < width and 0 <= neighbor_y < height:
                enqueue(neighbor_x, neighbor_y)
    for x, y in outside:
        rows[y][x * 4 : x * 4 + 4] = b"\x00\x00\x00\x00"
    if not preserve_enclosed:
        for row in rows:
            for offset in range(0, len(row), 4):
                if tuple(row[offset : offset + 3]) == background:
                    row[offset : offset + 4] = b"\x00\x00\x00\x00"
    return background


def clear_rectangles(
    rows: list[bytearray], rectangles: list[tuple[int, int, int, int]]
) -> None:
    width = len(rows[0]) // 4
    height = len(rows)
    for left, top, rectangle_width, rectangle_height in rectangles:
        for y in range(top, min(top + rectangle_height, height)):
            for x in range(left, min(left + rectangle_width, width)):
                rows[y][x * 4 : x * 4 + 4] = b"\x00\x00\x00\x00"


def connected_components(
    rows: list[bytearray], min_pixels: int
) -> list[list[tuple[int, int]]]:
    width = len(rows[0]) // 4
    height = len(rows)
    foreground = {
        (x, y)
        for y in range(height)
        for x in range(width)
        if rows[y][x * 4 + 3] != 0
    }
    visited: set[tuple[int, int]] = set()
    components: list[list[tuple[int, int]]] = []
    for start in foreground:
        if start in visited:
            continue
        stack = [start]
        visited.add(start)
        points: list[tuple[int, int]] = []
        while stack:
            x, y = stack.pop()
            points.append((x, y))
            for neighbor_y in range(max(0, y - 1), min(height, y + 2)):
                for neighbor_x in range(max(0, x - 1), min(width, x + 2)):
                    neighbor = (neighbor_x, neighbor_y)
                    if neighbor in foreground and neighbor not in visited:
                        visited.add(neighbor)
                        stack.append(neighbor)
        if len(points) >= min_pixels:
            components.append(points)
    return sorted(
        components,
        key=lambda points: (min(y for _, y in points), min(x for x, _ in points)),
    )


def object_canvas(
    rows: list[bytearray],
    points: list[tuple[int, int]],
    tile_size: int,
) -> tuple[list[bytearray], list[int], list[int]]:
    left = min(x for x, _ in points)
    right = max(x for x, _ in points)
    top = min(y for _, y in points)
    bottom = max(y for _, y in points)
    content_width = right - left + 1
    content_height = bottom - top + 1
    canvas_width = math.ceil(content_width / tile_size) * tile_size
    canvas_height = math.ceil(content_height / tile_size) * tile_size
    offset_x = (canvas_width - content_width) // 2
    offset_y = canvas_height - content_height
    canvas = [bytearray(canvas_width * 4) for _ in range(canvas_height)]
    point_set = set(points)
    for source_x, source_y in point_set:
        target_x = offset_x + source_x - left
        target_y = offset_y + source_y - top
        canvas[target_y][target_x * 4 : target_x * 4 + 4] = rows[source_y][
            source_x * 4 : source_x * 4 + 4
        ]
    return (
        canvas,
        [left, top, content_width, content_height],
        [offset_x, offset_y],
    )


def split_tiles(canvas: list[bytearray], tile_size: int) -> list[list[list[bytearray] | None]]:
    width = len(canvas[0]) // 4
    tile_rows: list[list[list[bytearray] | None]] = []
    for top in range(0, len(canvas), tile_size):
        output_row: list[list[bytearray] | None] = []
        for left in range(0, width, tile_size):
            tile = [
                bytearray(row[left * 4 : (left + tile_size) * 4])
                for row in canvas[top : top + tile_size]
            ]
            output_row.append(
                tile if any(pixel for row in tile for pixel in row[3::4]) else None
            )
        tile_rows.append(output_row)
    return tile_rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--tile-size", type=int, default=16)
    parser.add_argument("--min-component-pixels", type=int, default=16)
    parser.add_argument(
        "--exclude",
        action="append",
        type=parse_rectangle,
        default=[parse_rectangle(value) for value in DEFAULT_EXCLUSIONS],
        help="exclude x,y,width,height; may be repeated",
    )
    parser.add_argument("--clean", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--preserve-enclosed-background",
        action="store_true",
        help="only remove background pixels connected to the sheet boundary",
    )
    args = parser.parse_args()

    if args.tile_size < 1:
        parser.error("--tile-size must be positive")
    source = args.source.resolve()
    output = args.output.resolve()
    image = read_png(source)
    rows = [bytearray(row) for row in image.rows]
    background = remove_external_background(rows, args.preserve_enclosed_background)
    clear_rectangles(rows, args.exclude)
    components = connected_components(rows, args.min_component_pixels)

    canvases = [object_canvas(rows, points, args.tile_size) for points in components]
    tile_grids = [split_tiles(canvas, args.tile_size) for canvas, _, _ in canvases]
    unique_hashes = {
        hashlib.sha256(b"".join(bytes(row) for row in tile)).hexdigest()
        for grid in tile_grids
        for tile_row in grid
        for tile in tile_row
        if tile is not None
    }
    print(
        f"Detected {len(components)} objects and {len(unique_hashes)} unique "
        f"{args.tile_size}x{args.tile_size} tiles; background="
        f"#{bytes(background).hex().upper()}"
    )
    if args.dry_run:
        return 0

    if output.exists():
        if not args.clean:
            parser.error(f"output already exists; pass --clean to replace it: {output}")
        shutil.rmtree(output)
    tiles_dir = output / "tiles"
    tile_ids: dict[str, str] = {}
    manifest_objects = []
    for object_index, ((canvas, source_bbox, offset), grid) in enumerate(
        zip(canvases, tile_grids)
    ):
        manifest_grid: list[list[str | None]] = []
        for tile_row in grid:
            manifest_row: list[str | None] = []
            for tile in tile_row:
                if tile is None:
                    manifest_row.append(None)
                    continue
                digest = hashlib.sha256(b"".join(bytes(row) for row in tile)).hexdigest()
                tile_id = tile_ids.get(digest)
                if tile_id is None:
                    tile_id = f"tile-{len(tile_ids):04}"
                    tile_ids[digest] = tile_id
                    write_rgba_png(tiles_dir / f"{tile_id}.png", tile)
                manifest_row.append(tile_id)
            manifest_grid.append(manifest_row)
        manifest_objects.append(
            {
                "id": f"object-{object_index:03}",
                "source_bbox": source_bbox,
                "canvas_size": [len(canvas[0]) // 4, len(canvas)],
                "content_offset": offset,
                "tiles": manifest_grid,
            }
        )

    output.mkdir(parents=True, exist_ok=True)
    manifest = {
        "source": str(source),
        "source_size": [image.width, image.height],
        "background": "#{:02X}{:02X}{:02X}".format(*background),
        "preserve_enclosed_background": args.preserve_enclosed_background,
        "tile_size": args.tile_size,
        "excluded_rectangles": [list(rectangle) for rectangle in args.exclude],
        "object_count": len(manifest_objects),
        "unique_tile_count": len(tile_ids),
        "objects": manifest_objects,
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote {len(tile_ids)} unique tiles and manifest to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
