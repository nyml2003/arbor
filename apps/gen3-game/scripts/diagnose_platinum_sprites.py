#!/usr/bin/env python3
"""Diagnose whether Platinum sprite PNG files can be normalized consistently."""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import zlib

from platinum_sprite_common import normalize_frames, read_png


def diagnose(source: Path) -> dict[str, object]:
    files = sorted(source.rglob("*.png"))
    folders: Counter[str] = Counter()
    formats: Counter[str] = Counter()
    backgrounds: Counter[str] = Counter()
    issues: list[dict[str, str]] = []
    input_with_alpha = 0
    output_frames = 0

    for path in files:
        folders[path.parent.name] += 1
        try:
            image = read_png(path)
            formats[
                f"{image.width}x{image.height}/depth-{image.bit_depth}/type-{image.color_type}"
            ] += 1
            input_with_alpha += image.transparent_pixels > 0
            if image.background is None:
                issues.append(
                    {"file": str(path), "error": "no dominant opaque border color"}
                )
            else:
                backgrounds["#{:02X}{:02X}{:02X}".format(*image.background[:3])] += 1
            output_frames += len(normalize_frames(image))
        except (OSError, ValueError, zlib.error) as error:
            issues.append({"file": str(path), "error": str(error)})

    return {
        "source": str(source.resolve()),
        "input_files": len(files),
        "input_folders": dict(sorted(folders.items())),
        "input_formats": dict(sorted(formats.items())),
        "input_files_with_alpha": input_with_alpha,
        "unique_background_colors": len(backgrounds),
        "most_common_backgrounds": backgrounds.most_common(20),
        "planned_output_frames": output_frames,
        "output_format": "80x80 PNG, 8-bit RGBA, non-interlaced",
        "issues": issues,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    report = diagnose(args.source)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(f"Input files: {report['input_files']}")
        print(f"Folders: {report['input_folders']}")
        print(f"Formats: {report['input_formats']}")
        print(f"Files already containing alpha: {report['input_files_with_alpha']}")
        print(f"Unique detected backgrounds: {report['unique_background_colors']}")
        print(f"Planned output frames: {report['planned_output_frames']}")
        print(f"Output format: {report['output_format']}")
        issues = report["issues"]
        print(f"Issues: {len(issues)}")
        for issue in issues[:20]:
            print(f"  {issue['file']}: {issue['error']}")
    return 1 if report["issues"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
