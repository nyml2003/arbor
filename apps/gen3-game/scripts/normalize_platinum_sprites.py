#!/usr/bin/env python3
"""Normalize Platinum sprite sheets into transparent 80x80 RGBA PNG frames."""

from __future__ import annotations

import argparse
from pathlib import Path
import shutil

from platinum_sprite_common import normalize_frames, read_png, write_rgba_png


APP_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = APP_ROOT / "assets" / "pokemons"
FOLDER_LAYOUT = {
    "普通正面": Path("normal/front"),
    "普通背面": Path("normal/back"),
    "闪光正面": Path("shiny/front"),
    "闪光背面": Path("shiny/back"),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--clean",
        action="store_true",
        help="remove the existing output directory before normalization",
    )
    args = parser.parse_args()

    source = args.source.resolve()
    output = args.output.resolve()
    if not source.is_dir():
        parser.error(f"source directory does not exist: {source}")
    if output == source or source in output.parents:
        parser.error("output must not be the source directory or one of its children")
    if output.exists():
        if not args.clean:
            parser.error(f"output already exists; pass --clean to replace it: {output}")
        shutil.rmtree(output)

    source_count = 0
    frame_count = 0
    transparent_pixels = 0
    for source_folder, relative_output in FOLDER_LAYOUT.items():
        input_dir = source / source_folder
        if not input_dir.is_dir():
            raise RuntimeError(f"missing source folder: {input_dir}")
        for input_path in sorted(input_dir.glob("*.png")):
            image = read_png(input_path)
            if image.background is None:
                raise RuntimeError(f"no dominant opaque border color: {input_path}")
            frames = normalize_frames(image)
            for frame_index, rows in enumerate(frames):
                output_name = f"{input_path.stem}__frame_{frame_index}.png"
                write_rgba_png(output / relative_output / output_name, rows)
                transparent_pixels += sum(
                    row[offset + 3] == 0
                    for row in rows
                    for offset in range(0, len(row), 4)
                )
                frame_count += 1
            source_count += 1

    print(
        f"Normalized {source_count} source files into {frame_count} frames at {output} "
        f"({transparent_pixels} transparent pixels)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
