#!/usr/bin/env python3
"""Convert tiled 16-color IEE icon data to transparent PNG files."""

from __future__ import annotations

import argparse
import csv
from dataclasses import dataclass
from pathlib import Path

from platinum_sprite_common import write_rgba_png


APP_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = APP_ROOT / "tmp" / "iee-icons"
DEFAULT_NUMBERED_OUTPUT = APP_ROOT / "assets" / "pokemons" / "icons"
DEFAULT_SPECIES_DATA = APP_ROOT / "assets" / "pokeapi-current-data"
ICON_SIZE = 32
TILE_SIZE = 8
PIXEL_COUNT = ICON_SIZE * ICON_SIZE

LEGACY_NAME_ALIASES = {
    "草栗鼠": "哈力栗",
    "防猬栗": "胖胖哈力",
    "护猬栗": "布里卡隆",
    "火耳狐": "火狐狸",
    "柴尾狐": "长尾火狐",
    "魔妖狐": "妖火红狐",
    "泡沫蛙": "呱呱泡蛙",
    "精英蛙": "呱头蛙",
    "忍者蛙": "甲贺忍蛙",
    "挖掘兔": "掘掘兔",
    "铲地兔": "掘地兔",
    "矢更鸟": "小箭雀",
    "火矢鸟": "火箭雀",
    "炎矢鹰": "烈箭鹰",
    "吹粉虫": "粉蝶虫",
    "吹粉茧": "粉蝶蛹",
    "闪翅蝶": "彩粉蝶",
    "小火狮": "小狮狮",
    "焰吼狮": "火炎狮",
    "芙拉蓓蓓": "花蓓蓓",
    "芙拉惠特": "花叶蒂",
    "芙拉洁丝": "花洁夫人",
    "咩咩羊": "坐骑小羊",
    "前进羊": "坐骑山羊",
    "淘气熊猫": "顽皮熊猫",
    "恶棍熊猫": "霸道熊猫",
    "贵宾犬": "多丽米亚",
    "念爪猫": "妙喵",
    "超能喵": "超能妙喵",
    "孤灵剑": "独剑鞘",
    "双弑剑": "双剑鞘",
    "神盾剑": "坚盾剑怪",
    "香香鸟": "粉香香",
    "飘香鸟": "芳香精",
    "棉糖舔": "绵绵泡芙",
    "奶糖舔": "胖甜妮",
    "魔魔鱿": "好啦鱿",
    "魔乌贼": "乌贼王",
    "双头藤壶": "龟脚脚",
    "野蛮藤壶": "龟足巨铠",
    "藻海马": "垃垃藻",
    "类藻海龙": "毒藻龙",
    "钳炮虾": "铁臂枪虾",
    "枪炮虾": "钢炮臂虾",
    "电伞蜥": "伞电蜥",
    "光电蜥": "光电伞蜥",
    "稚暴龙": "宝宝暴龙",
    "暴君龙": "怪颚龙",
    "冰光龙": "冰雪龙",
    "极光龙": "冰雪巨龙",
    "仙伊布": "仙子伊布",
    "摔跤鹰": "摔角鹰人",
    "电电鼠": "咚咚鼠",
    "钻石精灵": "小碎钻",
    "黏宝龙": "黏黏宝",
    "黏蜗龙": "黏美儿",
    "黏王龙": "黏美龙",
    "妖匙链": "钥圈儿",
    "幽木灵": "小木灵",
    "枯树灵": "朽木妖",
    "幽灵草": "南瓜精",
    "幽灵南瓜": "南瓜怪人",
    "冰块石": "冰宝",
    "冰隙山": "冰岩怪",
    "音爆蝙蝠": "嗡蝠",
    "音爆飞龙": "音波龙",
    "泽尼亚斯": "哲尔尼亚斯",
    "伊维塔尔": "伊裴尔塔尔",
    "桀伽亚德": "基格尔德",
}

PALETTE_1_SPECIES = {
    650, 651, 652, 669, 670, 671, 672, 673, 674, 675, 685, 701, 705, 708,
    709, 718,
}
PALETTE_2_SPECIES = {
    656, 657, 659, 660, 667, 668, 677, 687, 690, 691, 696, 698, 699, 704,
    714, 715,
}

# Pokemon Emerald uses three shared palettes for 32x32 menu icons. IEE stores
# only the palette index, so callers must select the palette used by the icon.
# Palette index 0 is the transparent background in the exported PNG.
ICON_PALETTES = {
    "0": (
        (255, 255, 255, 0),
        (131, 131, 115, 255),
        (189, 189, 189, 255),
        (255, 255, 255, 255),
        (189, 164, 65, 255),
        (246, 246, 41, 255),
        (213, 98, 65, 255),
        (246, 148, 41, 255),
        (139, 123, 255, 255),
        (98, 74, 205, 255),
        (238, 115, 156, 255),
        (255, 180, 164, 255),
        (164, 197, 255, 255),
        (106, 172, 156, 255),
        (98, 98, 90, 255),
        (65, 65, 65, 255),
    ),
    "1": (
        (255, 255, 255, 0),
        (115, 115, 115, 255),
        (189, 189, 189, 255),
        (255, 255, 255, 255),
        (123, 156, 74, 255),
        (156, 205, 74, 255),
        (148, 246, 74, 255),
        (238, 115, 156, 255),
        (246, 148, 246, 255),
        (189, 164, 90, 255),
        (246, 230, 41, 255),
        (246, 246, 172, 255),
        (213, 213, 106, 255),
        (230, 74, 41, 255),
        (98, 98, 90, 255),
        (65, 65, 65, 255),
    ),
    "2": (
        (255, 255, 255, 0),
        (123, 123, 123, 255),
        (189, 189, 180, 255),
        (255, 255, 255, 255),
        (115, 115, 205, 255),
        (164, 172, 246, 255),
        (180, 131, 90, 255),
        (238, 197, 139, 255),
        (197, 172, 41, 255),
        (246, 246, 41, 255),
        (246, 98, 82, 255),
        (148, 123, 205, 255),
        (197, 164, 205, 255),
        (189, 41, 156, 255),
        (98, 98, 90, 255),
        (65, 65, 65, 255),
    ),
}


@dataclass(frozen=True)
class Conversion:
    source: Path
    output: Path
    pixels: list[int]
    missing_count: int
    palette_id: str
    national_dex: int | None
    canonical_name: str | None
    legacy_name: str
    frame: int | None


def read_iee(path: Path) -> tuple[list[int], int]:
    lines = path.read_text(encoding="ascii").splitlines()
    if not lines or lines[0].strip() != "[Icon]":
        raise ValueError(f"missing [Icon] header: {path}")

    values: dict[int, int] = {}
    for line_number, raw_line in enumerate(lines[1:], start=2):
        line = raw_line.strip()
        if not line:
            continue
        try:
            raw_index, raw_value = line.split("=", maxsplit=1)
            index = int(raw_index)
            value = int(raw_value, 16)
        except ValueError as error:
            raise ValueError(
                f"invalid entry at {path}:{line_number}: {raw_line!r}"
            ) from error
        if not 0 <= index < PIXEL_COUNT:
            raise ValueError(f"pixel index outside 0..{PIXEL_COUNT - 1}: {index}")
        if not 0 <= value < 16:
            raise ValueError(f"palette index outside 0..F: {raw_value!r}")
        if index in values:
            raise ValueError(f"duplicate pixel index {index}: {path}")
        values[index] = value

    if not values:
        raise ValueError(f"no pixel data: {path}")
    expected_indices = set(range(max(values) + 1))
    missing_inside_data = sorted(expected_indices - values.keys())
    if missing_inside_data:
        raise ValueError(f"missing pixel index {missing_inside_data[0]} before end of data")

    missing_count = PIXEL_COUNT - len(values)
    pixels = [values.get(index, 0) for index in range(PIXEL_COUNT)]
    return pixels, missing_count


def untile(pixels: list[int]) -> list[list[int]]:
    tiles_per_row = ICON_SIZE // TILE_SIZE
    rows = [[0] * ICON_SIZE for _ in range(ICON_SIZE)]
    pixels_per_tile = TILE_SIZE * TILE_SIZE
    for source_index, palette_index in enumerate(pixels):
        tile_index, within_tile = divmod(source_index, pixels_per_tile)
        tile_x = tile_index % tiles_per_row
        tile_y = tile_index // tiles_per_row
        pixel_x = tile_x * TILE_SIZE + within_tile % TILE_SIZE
        pixel_y = tile_y * TILE_SIZE + within_tile // TILE_SIZE
        rows[pixel_y][pixel_x] = palette_index
    return rows


def rgba_rows(
    indexed_rows: list[list[int]],
    scale: int,
    palette: tuple[tuple[int, int, int, int], ...],
) -> list[bytearray]:
    output: list[bytearray] = []
    for indexed_row in indexed_rows:
        row = bytearray()
        for palette_index in indexed_row:
            row.extend(palette[palette_index] * scale)
        for _ in range(scale):
            output.append(bytearray(row))
    return output


def source_files(source: Path) -> list[Path]:
    if source.is_file():
        if source.suffix.lower() != ".iee":
            raise ValueError(f"source file must use the .iee extension: {source}")
        return [source]
    if source.is_dir():
        files = sorted(source.glob("*.iee"))
        if not files:
            raise ValueError(f"no .iee files found in: {source}")
        return files
    raise ValueError(f"source does not exist: {source}")


def load_species_numbers(data_dir: Path, language: str) -> dict[str, int]:
    languages_path = data_dir / "languages.csv"
    names_path = data_dir / "pokemon_species_names.csv"
    if not languages_path.is_file() or not names_path.is_file():
        raise ValueError(
            "species data must contain languages.csv and pokemon_species_names.csv: "
            f"{data_dir}"
        )

    with languages_path.open(encoding="utf-8-sig", newline="") as file:
        language_rows = list(csv.DictReader(file))
    language_ids = [row["id"] for row in language_rows if row["identifier"] == language]
    if len(language_ids) != 1:
        raise ValueError(f"expected one language named {language!r}, found {len(language_ids)}")

    names: dict[str, int] = {}
    with names_path.open(encoding="utf-8-sig", newline="") as file:
        for row in csv.DictReader(file):
            if row["local_language_id"] != language_ids[0]:
                continue
            name = row["name"]
            species_id = int(row["pokemon_species_id"])
            previous = names.get(name)
            if previous is not None and previous != species_id:
                raise ValueError(f"duplicate species name {name!r} in {names_path}")
            names[name] = species_id
    if not names:
        raise ValueError(f"no species names for language {language!r}: {names_path}")
    return names


def legacy_name_and_frame(stem: str) -> tuple[str, int]:
    if stem.endswith("2"):
        return stem[:-1], 1
    return stem, 0


def palette_for_species(species_id: int) -> str:
    if species_id in PALETTE_1_SPECIES:
        return "1"
    if species_id in PALETTE_2_SPECIES:
        return "2"
    return "0"


def build_conversions(
    inputs: list[Path],
    output: Path,
    palette_arg: str,
    species_numbers: dict[str, int] | None,
) -> list[Conversion]:
    conversions: list[Conversion] = []
    output_paths: set[Path] = set()
    for input_path in inputs:
        pixels, missing_count = read_iee(input_path)
        legacy_name, frame = legacy_name_and_frame(input_path.stem)
        canonical_name: str | None = None
        national_dex: int | None = None
        output_name = f"{input_path.stem}.png"

        if species_numbers is not None:
            canonical_name = LEGACY_NAME_ALIASES.get(legacy_name, legacy_name)
            national_dex = species_numbers.get(canonical_name)
            if national_dex is None:
                raise ValueError(
                    f"could not match {legacy_name!r} as {canonical_name!r} in species data"
                )
            output_name = f"{national_dex:03d}_{frame}.png"

        if palette_arg == "auto":
            if national_dex is None:
                raise ValueError("--palette auto requires --number-by-species")
            palette_id = palette_for_species(national_dex)
        else:
            palette_id = palette_arg

        output_path = output / output_name
        if output_path in output_paths:
            raise ValueError(f"multiple inputs resolve to the same output: {output_path}")
        output_paths.add(output_path)
        conversions.append(
            Conversion(
                source=input_path,
                output=output_path,
                pixels=pixels,
                missing_count=missing_count,
                palette_id=palette_id,
                national_dex=national_dex,
                canonical_name=canonical_name,
                legacy_name=legacy_name,
                frame=frame if national_dex is not None else None,
            )
        )
    if species_numbers is not None:
        conversions.sort(
            key=lambda conversion: (
                conversion.national_dex or 0,
                conversion.frame or 0,
            )
        )
    return conversions


def write_index(path: Path, conversions: list[Conversion]) -> None:
    with path.open("w", encoding="utf-8-sig", newline="") as file:
        writer = csv.DictWriter(
            file,
            fieldnames=(
                "national_dex",
                "canonical_name",
                "legacy_name",
                "frame",
                "palette",
                "file",
            ),
        )
        writer.writeheader()
        for conversion in conversions:
            writer.writerow(
                {
                    "national_dex": conversion.national_dex,
                    "canonical_name": conversion.canonical_name,
                    "legacy_name": conversion.legacy_name,
                    "frame": conversion.frame,
                    "palette": conversion.palette_id,
                    "file": conversion.output.name,
                }
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="an IEE file or a directory of IEE files")
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--scale",
        type=int,
        default=1,
        help="nearest-neighbor output scale (default: 1)",
    )
    parser.add_argument(
        "--palette",
        choices=("auto", *sorted(ICON_PALETTES)),
        default="0",
        help="Pokemon Emerald menu icon palette (default: 0)",
    )
    parser.add_argument(
        "--number-by-species",
        action="store_true",
        help="name frames by national Pokedex number using PokeAPI species data",
    )
    parser.add_argument(
        "--species-data",
        type=Path,
        default=DEFAULT_SPECIES_DATA,
        help="directory containing PokeAPI languages and species names CSV files",
    )
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args()

    if args.scale < 1:
        parser.error("--scale must be at least 1")

    source = args.source.resolve()
    default_output = DEFAULT_NUMBERED_OUTPUT if args.number_by_species else DEFAULT_OUTPUT
    output = (args.output or default_output).resolve()
    try:
        inputs = source_files(source)
        species_numbers = (
            load_species_numbers(args.species_data.resolve(), "zh-hans")
            if args.number_by_species
            else None
        )
        conversions = build_conversions(inputs, output, args.palette, species_numbers)
    except ValueError as error:
        parser.error(str(error))

    existing = [conversion.output for conversion in conversions if conversion.output.exists()]
    if existing and not args.overwrite:
        parser.error(
            f"{len(existing)} output files already exist; pass --overwrite to replace them"
        )

    output.mkdir(parents=True, exist_ok=True)
    for conversion in conversions:
        write_rgba_png(
            conversion.output,
            rgba_rows(
                untile(conversion.pixels),
                args.scale,
                ICON_PALETTES[conversion.palette_id],
            ),
        )
        suffix = (
            f"; filled {conversion.missing_count} trailing pixels with transparency"
            if conversion.missing_count
            else ""
        )
        size = ICON_SIZE * args.scale
        print(
            f"Converted {conversion.source.name} -> {conversion.output.name} "
            f"({size}x{size}, palette={conversion.palette_id}{suffix})"
        )

    if args.number_by_species:
        write_index(output / "index.csv", conversions)
    print(f"Converted {len(conversions)} files into {output}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
