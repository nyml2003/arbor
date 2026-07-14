from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


STATEFUL_OR_EFFECT_PACKAGES = (
    "game-data-import",
    "game-fs-assets",
    "game-host",
    "game-native-target",
    "map-editor",
)
MAX_STATEFUL_OR_EFFECT_LINES = 1_500

PURE_FUNCTIONAL_PACKAGES = (
    "battle-application",
    "battle-domain",
    "battle-ramus-adapter",
    "battle-session",
    "game-asset-plan",
    "game-assets",
    "game-data",
    "game-data-import-core",
    "game-e2e",
    "game-native-plan",
    "game-scene-view",
    "game-session",
    "game-ui",
    "game-view",
    "map-assets",
    "map-editor-core",
    "map-editor-view",
    "map-project",
    "map-render",
    "world-domain",
    "world-application",
)


def production_lines(source: Path) -> list[tuple[int, str]]:
    result: list[tuple[int, str]] = []
    for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
        if line.strip() == "#[cfg(test)]":
            break
        result.append((number, line))
    return result


def declaration_lines(lines: list[tuple[int, str]]) -> set[int]:
    excluded: set[int] = set()
    mode: str | None = None
    brace_depth = 0
    saw_brace = False
    for number, line in lines:
        stripped = line.strip()
        if mode is None:
            if re.match(r"^(?:pub(?:\([^)]*\))?\s+)?use\s", stripped):
                mode = "use"
            elif re.match(
                r"^(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum)\s", stripped
            ):
                mode = "type"
            else:
                continue
        excluded.add(number)
        brace_depth += line.count("{") - line.count("}")
        saw_brace = saw_brace or "{" in line
        if mode == "use" and ";" in line and brace_depth == 0:
            mode = None
            saw_brace = False
        elif mode == "type" and (
            (saw_brace and brace_depth == 0) or (not saw_brace and ";" in line)
        ):
            mode = None
            saw_brace = False
    return excluded


def count_source_lines(package: Path) -> int:
    count = 0
    for source in (package / "src").rglob("*.rs"):
        if source.name == "tests.rs" or "tests" in source.parts:
            continue
        lines = production_lines(source)
        excluded = declaration_lines(lines)
        count += sum(
            1
            for number, line in lines
            if number not in excluded
            and line.strip()
            and not line.lstrip().startswith("//")
        )
    return count


def run_coverage(workspace: Path) -> Path:
    output = workspace / "target" / "pure-coverage.lcov"
    command = ["cargo", "llvm-cov"]
    for package in PURE_FUNCTIONAL_PACKAGES:
        command.extend(["-p", package])
    command.extend(["--tests", "--lcov", "--output-path", str(output)])
    subprocess.run(command, cwd=workspace, check=True)
    return output


def lcov_hits(report: Path) -> dict[Path, dict[int, int]]:
    result: dict[Path, dict[int, int]] = {}
    current: Path | None = None
    for line in report.read_text(encoding="utf-8").splitlines():
        if line.startswith("SF:"):
            current = Path(line[3:]).resolve()
            result.setdefault(current, {})
        elif line.startswith("DA:") and current is not None:
            number, hits, *_ = line[3:].split(",")
            line_number = int(number)
            result[current][line_number] = result[current].get(line_number, 0) + int(hits)
    return result


def is_executable(line: str) -> bool:
    stripped = line.strip()
    if not stripped or stripped.startswith(("//", "#[")):
        return False
    if stripped in {
        "{",
        "}",
        "};",
        ");",
        "));",
        "]",
        "],",
        "),",
        ")?",
        ")?;",
        ")? {",
        "})",
        "})?",
        "})?;",
    }:
        return False
    return not stripped.startswith(
        ("use ", "mod ", "pub mod ", "struct ", "pub struct ", "enum ", "pub enum ", "impl ")
    )


def main() -> int:
    workspace = Path(__file__).resolve().parent.parent
    failed = False
    stateful_counts = {
        package: count_source_lines(workspace / "crates" / package)
        for package in STATEFUL_OR_EFFECT_PACKAGES
    }
    for package, count in stateful_counts.items():
        print(f"crates/{package}: {count} stateful/effect production lines")
    stateful_total = sum(stateful_counts.values())
    if stateful_total > MAX_STATEFUL_OR_EFFECT_LINES:
        print(
            f"stateful/effect total: {stateful_total} production lines exceed "
            f"{MAX_STATEFUL_OR_EFFECT_LINES}",
            file=sys.stderr,
        )
        failed = True
    else:
        print(
            f"stateful/effect total: {stateful_total}/{MAX_STATEFUL_OR_EFFECT_LINES} "
            "production lines"
        )

    try:
        hits = lcov_hits(run_coverage(workspace))
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"pure coverage command failed: {error}", file=sys.stderr)
        return 2

    for package in PURE_FUNCTIONAL_PACKAGES:
        missed: list[str] = []
        covered = 0
        total = 0
        for source in (workspace / "crates" / package / "src").rglob("*.rs"):
            source_hits = hits.get(source.resolve(), {})
            for number, line in production_lines(source):
                if number not in source_hits or not is_executable(line):
                    continue
                total += 1
                if source_hits[number] > 0:
                    covered += 1
                else:
                    missed.append(f"{source.relative_to(workspace)}:{number}")
        if missed:
            print(
                f"crates/{package}: pure production coverage {covered}/{total}; "
                f"missed {', '.join(missed)}",
                file=sys.stderr,
            )
            failed = True
        else:
            print(f"crates/{package}: pure production coverage {covered}/{total} (100%)")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
