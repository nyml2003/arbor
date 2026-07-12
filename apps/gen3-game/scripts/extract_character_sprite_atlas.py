#!/usr/bin/env python3
"""Extract individual centered actions from a character sprite atlas."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import shutil

from platinum_sprite_common import read_png, write_rgba_png


APP_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = APP_ROOT / "assets" / "characters" / "red" / "actions"
KNOWN_LABELS = {
    (0, 0): "up_stand",
    (0, 8): "up_walk_1",
    (0, 9): "up_walk_2",
    (0, 7): "up_run_1",
    (0, 10): "up_run_2",
    (0, 20): "up_run_3",
    (0, 1): "left_stand",
    (0, 2): "left_walk_1",
    (0, 3): "left_walk_2",
    (0, 14): "left_run_1",
    (0, 15): "left_run_2",
    (0, 16): "left_run_3",
    (0, 4): "right_stand",
    (0, 5): "right_walk_1",
    (0, 6): "right_walk_2",
    (0, 17): "right_run_1",
    (0, 18): "right_run_2",
    (0, 19): "right_run_3",
    (0, 11): "down_stand",
    (0, 12): "down_walk_1",
    (0, 13): "down_walk_2",
    (0, 21): "down_run_1",
    (0, 22): "down_run_2",
    (0, 23): "down_run_3",
}


@dataclass
class DetectedSprite:
    row: int
    left: int
    right: int
    top: int
    bottom: int
    center_x: float
    group: int = -1


def occupied_intervals(
    rows: list[bytearray],
    background_rgb: tuple[int, int, int],
    top: int,
    height: int,
    merge_gap: int,
) -> list[tuple[int, int]]:
    width = len(rows[0]) // 4
    occupied = []
    for x in range(width):
        if any(
            tuple(rows[y][x * 4 : x * 4 + 3]) != background_rgb
            for y in range(top, top + height)
        ):
            occupied.append(x)
    if not occupied:
        return []

    intervals: list[tuple[int, int]] = []
    start = previous = occupied[0]
    for x in occupied[1:]:
        if x - previous - 1 > merge_gap:
            intervals.append((start, previous))
            start = x
        previous = x
    intervals.append((start, previous))
    return intervals


def detect_sprites(
    rows: list[bytearray],
    background_rgb: tuple[int, int, int],
    cell_height: int,
    merge_gap: int,
) -> list[DetectedSprite]:
    height = len(rows)
    if height % cell_height:
        raise ValueError(f"atlas height {height} is not divisible by {cell_height}")

    sprites: list[DetectedSprite] = []
    for row_index, top in enumerate(range(0, height, cell_height)):
        for left, right in occupied_intervals(
            rows, background_rgb, top, cell_height, merge_gap
        ):
            points = [
                (x, y)
                for y in range(top, top + cell_height)
                for x in range(left, right + 1)
                if tuple(rows[y][x * 4 : x * 4 + 3]) != background_rgb
            ]
            if not points:
                continue
            xs = [point[0] for point in points]
            ys = [point[1] for point in points]
            sprites.append(
                DetectedSprite(
                    row=row_index,
                    left=min(xs),
                    right=max(xs),
                    top=min(ys),
                    bottom=max(ys),
                    center_x=(min(xs) + max(xs)) / 2,
                )
            )
    return sprites


def assign_groups(sprites: list[DetectedSprite], tolerance: float = 18) -> list[float]:
    clusters: list[list[float]] = []
    for center in sorted(sprite.center_x for sprite in sprites):
        nearest = min(
            range(len(clusters)),
            key=lambda index: abs(center - sum(clusters[index]) / len(clusters[index])),
            default=None,
        )
        if nearest is None or abs(center - sum(clusters[nearest]) / len(clusters[nearest])) > tolerance:
            clusters.append([center])
        else:
            clusters[nearest].append(center)
    centers = sorted(sum(cluster) / len(cluster) for cluster in clusters)
    for sprite in sprites:
        sprite.group = min(
            range(len(centers)), key=lambda index: abs(sprite.center_x - centers[index])
        )
    return centers


def centered_frame(
    rows: list[bytearray],
    sprite: DetectedSprite,
    background_rgb: tuple[int, int, int],
    size: int,
) -> tuple[list[bytearray], tuple[int, int]]:
    content_width = sprite.right - sprite.left + 1
    content_height = sprite.bottom - sprite.top + 1
    if content_width > size or content_height > size:
        raise ValueError(
            f"sprite at row {sprite.row}, x={sprite.left} is "
            f"{content_width}x{content_height}, larger than {size}x{size}"
        )
    offset_x = (size - content_width) // 2
    offset_y = (size - content_height) // 2
    frame = [bytearray(size * 4) for _ in range(size)]
    for source_y in range(sprite.top, sprite.bottom + 1):
        for source_x in range(sprite.left, sprite.right + 1):
            pixel = rows[source_y][source_x * 4 : source_x * 4 + 4]
            if tuple(pixel[:3]) == background_rgb:
                continue
            target_x = offset_x + source_x - sprite.left
            target_y = offset_y + source_y - sprite.top
            frame[target_y][target_x * 4 : target_x * 4 + 4] = pixel
    return frame, (offset_x, offset_y)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--cell-height", type=int, default=32)
    parser.add_argument("--size", type=int, default=32)
    parser.add_argument("--merge-gap", type=int, default=3)
    parser.add_argument("--clean", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    source = args.source.resolve()
    output = args.output.resolve()
    image = read_png(source)
    if image.background is None:
        raise RuntimeError("could not detect a dominant atlas background color")
    background_rgb = image.background[:3]
    sprites = detect_sprites(
        image.rows, background_rgb, args.cell_height, args.merge_gap
    )
    group_centers = assign_groups(sprites)

    print(
        f"Detected {len(sprites)} sprites in {image.width}x{image.height}; "
        f"rows={image.height // args.cell_height}, groups={len(group_centers)}, "
        f"background=#{bytes(background_rgb).hex().upper()}"
    )
    if args.dry_run:
        return 0

    if output.exists():
        if not args.clean:
            parser.error(f"output already exists; pass --clean to replace it: {output}")
        shutil.rmtree(output)

    manifest = {
        "source": str(source),
        "source_size": [image.width, image.height],
        "background": "#{:02X}{:02X}{:02X}".format(*background_rgb),
        "cell_height": args.cell_height,
        "output_size": [args.size, args.size],
        "group_centers_x": group_centers,
        "sprites": [],
    }
    for index, sprite in enumerate(sprites):
        frame, destination_offset = centered_frame(
            image.rows, sprite, background_rgb, args.size
        )
        label = KNOWN_LABELS.get((sprite.group, sprite.row))
        name = (
            f"{label}.png"
            if label is not None
            else f"group-{sprite.group:02}_row-{sprite.row:02}.png"
        )
        relative_path = Path(f"group-{sprite.group:02}") / name
        write_rgba_png(output / relative_path, frame)
        manifest["sprites"].append(
            {
                "index": index,
                "group": sprite.group,
                "row": sprite.row,
                "file": relative_path.as_posix(),
                "source_bbox": [
                    sprite.left,
                    sprite.top,
                    sprite.right - sprite.left + 1,
                    sprite.bottom - sprite.top + 1,
                ],
                "destination_offset": list(destination_offset),
                "label": label,
            }
        )
    output.mkdir(parents=True, exist_ok=True)
    (output / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(f"Wrote {len(sprites)} centered {args.size}x{args.size} sprites to {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
