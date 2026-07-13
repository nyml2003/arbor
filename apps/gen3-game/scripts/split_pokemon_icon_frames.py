#!/usr/bin/env python3
"""Split Generation I-V 32x64 Pokemon menu icons into two PNG frames."""

from __future__ import annotations

import argparse
from collections import Counter
import csv
from dataclasses import dataclass
from pathlib import Path
import struct

from platinum_sprite_common import read_png, write_rgba_png


APP_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = APP_ROOT / "assets" / "pokemons" / "icons"
DEFAULT_SPECIES_DATA = APP_ROOT / "assets" / "pokeapi-current-data"
FRAME_SIZE = 32
SOURCE_HEIGHT = FRAME_SIZE * 2

GENERATION_BASE_RANGES = {
    "Gen I": ((1, 151, 0),),
    "Gen II": ((152, 251, 0),),
    "Gen III": ((277, 382, -25), (383, 410, -24), (411, 411, -53)),
    "Gen IV": ((440, 546, -53),),
    "Gen V": ((578, 733, -84),),
}

FORM_SOURCES = {
    ("Gen III", "410a"): (386, "deoxys-attack"),
    ("Gen III", "410d"): (386, "deoxys-defense"),
    ("Gen III", "410s"): (386, "deoxys-speed"),
    ("Gen IV", "547"): (412, "burmy-sandy"),
    ("Gen IV", "548"): (412, "burmy-trash"),
    ("Gen IV", "549"): (413, "wormadam-sandy"),
    ("Gen IV", "550"): (413, "wormadam-trash"),
    ("Gen IV", "551"): (421, "cherrim-sunshine"),
    ("Gen IV", "552"): (422, "shellos-east"),
    ("Gen IV", "553"): (423, "gastrodon-east"),
    ("Gen IV", "554"): (479, "rotom-heat"),
    ("Gen IV", "555"): (479, "rotom-wash"),
    ("Gen IV", "556"): (479, "rotom-frost"),
    ("Gen IV", "557"): (479, "rotom-fan"),
    ("Gen IV", "558"): (479, "rotom-mow"),
    ("Gen IV", "559"): (487, "giratina-origin"),
    ("Gen IV", "560"): (492, "shaymin-sky"),
    ("Gen V", "605b"): (521, "unfezant-female"),
    ("Gen V", "676f"): (592, "frillish-female"),
    ("Gen V", "677f"): (593, "jellicent-female"),
    ("Gen V", "734"): (550, "basculin-blue-striped"),
    ("Gen V", "735"): (555, "darmanitan-zen"),
    ("Gen V", "736"): (585, "deerling-summer"),
    ("Gen V", "737"): (585, "deerling-autumn"),
    ("Gen V", "738"): (585, "deerling-winter"),
    ("Gen V", "739"): (586, "sawsbuck-summer"),
    ("Gen V", "740"): (586, "sawsbuck-autumn"),
    ("Gen V", "741"): (586, "sawsbuck-winter"),
    ("Gen V", "742"): (641, "tornadus-therian"),
    ("Gen V", "743"): (642, "thundurus-therian"),
    ("Gen V", "744"): (645, "landorus-therian"),
    ("Gen V", "745"): (646, "kyurem-white"),
    ("Gen V", "746"): (646, "kyurem-black"),
    ("Gen V", "747"): (647, "keldeo-resolute"),
    ("Gen V", "748"): (648, "meloetta-pirouette"),
}


@dataclass(frozen=True)
class SourceIcon:
    source: Path
    source_name: str
    national_dex: int | None
    canonical_name: str
    form: str
    output_stem: str
    frames: tuple[list[bytearray], list[bytearray]]


def read_species_names(data_dir: Path, language: str) -> dict[int, str]:
    languages_path = data_dir / "languages.csv"
    names_path = data_dir / "pokemon_species_names.csv"
    if not languages_path.is_file() or not names_path.is_file():
        raise ValueError(
            "species data must contain languages.csv and pokemon_species_names.csv: "
            f"{data_dir}"
        )

    with languages_path.open(encoding="utf-8-sig", newline="") as file:
        language_ids = [
            row["id"]
            for row in csv.DictReader(file)
            if row["identifier"] == language
        ]
    if len(language_ids) != 1:
        raise ValueError(f"expected one language named {language!r}")

    names: dict[int, str] = {}
    with names_path.open(encoding="utf-8-sig", newline="") as file:
        for row in csv.DictReader(file):
            if row["local_language_id"] == language_ids[0]:
                names[int(row["pokemon_species_id"])] = row["name"]
    return names


def read_indexed_bmp(path: Path) -> list[bytearray]:
    content = path.read_bytes()
    if len(content) < 54 or content[:2] != b"BM":
        raise ValueError(f"not a BMP file: {path}")
    pixel_offset = struct.unpack_from("<I", content, 10)[0]
    dib_size = struct.unpack_from("<I", content, 14)[0]
    width, signed_height = struct.unpack_from("<ii", content, 18)
    planes, bit_depth = struct.unpack_from("<HH", content, 26)
    compression = struct.unpack_from("<I", content, 30)[0]
    if dib_size < 40 or planes != 1 or bit_depth != 8 or compression != 0:
        raise ValueError(
            f"unsupported BMP format: depth={bit_depth}, compression={compression}"
        )

    height = abs(signed_height)
    color_count = struct.unpack_from("<I", content, 46)[0] or 256
    palette_start = 14 + dib_size
    palette_end = palette_start + color_count * 4
    if palette_end > pixel_offset or palette_end > len(content):
        raise ValueError(f"invalid BMP palette: {path}")
    palette = []
    for offset in range(palette_start, palette_end, 4):
        blue, green, red, _ = content[offset : offset + 4]
        palette.append((red, green, blue, 255))

    stride = ((width * bit_depth + 31) // 32) * 4
    if pixel_offset + stride * height > len(content):
        raise ValueError(f"truncated BMP pixel data: {path}")
    packed_rows = [
        content[pixel_offset + row * stride : pixel_offset + row * stride + width]
        for row in range(height)
    ]
    if signed_height > 0:
        packed_rows.reverse()

    rows: list[bytearray] = []
    for packed in packed_rows:
        row = bytearray()
        for palette_index in packed:
            if palette_index >= len(palette):
                raise ValueError(f"BMP palette index out of range: {path}")
            row.extend(palette[palette_index])
        rows.append(row)
    return rows


def make_background_transparent(rows: list[bytearray]) -> list[bytearray]:
    output = [bytearray(row) for row in rows]
    width = len(output[0]) // 4
    border = (
        [tuple(output[0][offset : offset + 4]) for offset in range(0, width * 4, 4)]
        + [tuple(output[-1][offset : offset + 4]) for offset in range(0, width * 4, 4)]
        + [tuple(row[:4]) for row in output[1:-1]]
        + [tuple(row[-4:]) for row in output[1:-1]]
    )
    background, count = Counter(border).most_common(1)[0]
    if background[3] != 0 and count / len(border) >= 0.5:
        background_rgb = background[:3]
        for row in output:
            for offset in range(0, len(row), 4):
                if tuple(row[offset : offset + 3]) == background_rgb:
                    row[offset : offset + 4] = b"\xff\xff\xff\x00"
    return output


def read_icon_rows(path: Path) -> list[bytearray]:
    if path.suffix.lower() == ".png":
        image = read_png(path)
        if (image.width, image.height) != (FRAME_SIZE, SOURCE_HEIGHT):
            raise ValueError(
                f"expected {FRAME_SIZE}x{SOURCE_HEIGHT}, got "
                f"{image.width}x{image.height}: {path}"
            )
        rows = image.rows
    elif path.suffix.lower() == ".bmp":
        rows = read_indexed_bmp(path)
        if len(rows) != SOURCE_HEIGHT or len(rows[0]) != FRAME_SIZE * 4:
            raise ValueError(f"expected {FRAME_SIZE}x{SOURCE_HEIGHT}: {path}")
    else:
        raise ValueError(f"unsupported icon extension: {path}")
    return make_background_transparent(rows)


def base_species_id(generation: str, source_number: int) -> int | None:
    for first, last, offset in GENERATION_BASE_RANGES[generation]:
        if first <= source_number <= last:
            return source_number + offset
    return None


def unown_form(source_number: int) -> tuple[int | None, str] | None:
    if source_number == 412:
        return None, "egg"
    if 413 <= source_number <= 437:
        letter = chr(ord("b") + source_number - 413)
        return 201, f"unown-{letter}"
    if source_number == 438:
        return 201, "unown-exclamation"
    if source_number == 439:
        return 201, "unown-question"
    return None


def resolve_source(
    generation: str,
    path: Path,
    species_names: dict[int, str],
) -> SourceIcon:
    key = (generation, path.stem.lower())
    form_entry = FORM_SOURCES.get(key)
    source_number = int(path.stem) if path.stem.isdigit() else None
    if form_entry is None and generation == "Gen II" and source_number is not None:
        form_entry = unown_form(source_number)

    if form_entry is not None:
        national_dex, form = form_entry
        output_stem = form if national_dex is None else f"{national_dex:03d}_{form}"
    elif source_number is not None:
        national_dex = base_species_id(generation, source_number)
        if national_dex is None:
            raise ValueError(f"unmapped source icon: {generation}/{path.name}")
        form = ""
        output_stem = f"{national_dex:03d}"
    else:
        raise ValueError(f"unmapped source icon: {generation}/{path.name}")

    if national_dex is None:
        canonical_name = "蛋"
    else:
        try:
            canonical_name = species_names[national_dex]
        except KeyError as error:
            raise ValueError(f"missing species {national_dex} in PokeAPI names") from error

    rows = read_icon_rows(path)
    frames = (
        [bytearray(row) for row in rows[:FRAME_SIZE]],
        [bytearray(row) for row in rows[FRAME_SIZE:]],
    )
    return SourceIcon(
        source=path,
        source_name=f"{generation}/{path.name}",
        national_dex=national_dex,
        canonical_name=canonical_name,
        form=form,
        output_stem=output_stem,
        frames=frames,
    )


def collect_icons(source: Path, species_names: dict[int, str]) -> list[SourceIcon]:
    icons: list[SourceIcon] = []
    for generation in GENERATION_BASE_RANGES:
        generation_dir = source / generation
        if not generation_dir.is_dir():
            raise ValueError(f"missing generation directory: {generation_dir}")
        paths = sorted(
            (
                path
                for path in generation_dir.iterdir()
                if path.is_file() and path.suffix.lower() in {".png", ".bmp"}
            ),
            key=lambda path: path.name.lower(),
        )
        for path in paths:
            if path.stem.lower().startswith("gen"):
                continue
            icons.append(resolve_source(generation, path, species_names))

    base_ids = {icon.national_dex for icon in icons if not icon.form}
    expected_ids = set(range(1, 650))
    if base_ids != expected_ids:
        raise ValueError(
            f"base species mismatch; missing={sorted(expected_ids - base_ids)}, "
            f"extra={sorted(base_ids - expected_ids)}"
        )
    output_stems = [icon.output_stem for icon in icons]
    duplicates = [stem for stem, count in Counter(output_stems).items() if count > 1]
    if duplicates:
        raise ValueError(f"duplicate output stems: {duplicates}")
    return sorted(
        icons,
        key=lambda icon: (
            icon.national_dex is None,
            icon.national_dex or 0,
            icon.form,
        ),
    )


def write_index(path: Path, icons: list[SourceIcon]) -> None:
    with path.open("w", encoding="utf-8-sig", newline="") as file:
        writer = csv.DictWriter(
            file,
            fieldnames=(
                "national_dex",
                "canonical_name",
                "form",
                "source",
                "frame",
                "file",
            ),
        )
        writer.writeheader()
        for icon in icons:
            for frame in range(2):
                writer.writerow(
                    {
                        "national_dex": icon.national_dex,
                        "canonical_name": icon.canonical_name,
                        "form": icon.form,
                        "source": icon.source_name,
                        "frame": frame,
                        "file": f"{icon.output_stem}_{frame}.png",
                    }
                )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--species-data",
        type=Path,
        default=DEFAULT_SPECIES_DATA,
        help="directory containing PokeAPI languages and species names CSV files",
    )
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()

    source = args.source.resolve()
    output = args.output.resolve()
    try:
        species_names = read_species_names(args.species_data.resolve(), "zh-hans")
        icons = collect_icons(source, species_names)
    except (OSError, ValueError) as error:
        parser.error(str(error))

    output_paths = [
        output / f"{icon.output_stem}_{frame}.png"
        for icon in icons
        for frame in range(2)
    ]
    existing = [path for path in output_paths if path.exists()]
    if existing and not args.overwrite:
        parser.error(
            f"{len(existing)} output files already exist; pass --overwrite to replace them"
        )

    output.mkdir(parents=True, exist_ok=True)
    for icon in icons:
        for frame, rows in enumerate(icon.frames):
            write_rgba_png(output / f"{icon.output_stem}_{frame}.png", rows)
    write_index(output / "index-gen1-5.csv", icons)
    base_count = sum(not icon.form for icon in icons)
    form_count = sum(bool(icon.form) for icon in icons)
    print(
        f"Converted {len(icons) * 2} frames from {len(icons)} icons into {output}; "
        f"base={base_count}, forms={form_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
