from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

from arbor_projects.application import CoverageReadError
from arbor_projects.domain import CoverageTarget, FileCoverage, Project

from .llvm_export import parse_llvm_export


COVERAGE_DIRECTORY = "target/llvm-cov-target"
PROFILE_PATTERN = "*.profdata"
RUSTC_COMMAND = "rustc"
SYSROOT_ARGUMENTS = (RUSTC_COMMAND, "--print", "sysroot")
VERBOSE_VERSION_ARGUMENTS = (RUSTC_COMMAND, "-vV")
HOST_PREFIX = "host:"
LLVM_COV_NAME = "llvm-cov"
WINDOWS_EXECUTABLE_SUFFIX = ".exe"
LLVM_COV_EXPORT = "export"
LLVM_COV_FORMAT = "-format=text"
PROFILE_ARGUMENT = "-instr-profile={profile}"
OBJECT_ARGUMENT = "-object={object}"
RUSTLIB_PATH = "lib/rustlib"
BIN_DIRECTORY = "bin"
EXCLUDED_OBJECT_SUFFIXES = (".d", ".pdb", ".profraw", ".json")
PROCESS_SUCCESS = 0
SUBPROCESS_CAPTURE = True
SUBPROCESS_TEXT = True
SUBPROCESS_CHECK = False
TOOL_FAILED_MESSAGE = "{tool} failed with exit code {code}: {stderr}"
MISSING_HOST_MESSAGE = "rustc did not report a host triple"
MISSING_TOOL_MESSAGE = "LLVM coverage tool not found: {path}"
MISSING_PROFILE_MESSAGE = "coverage profile not found under {path}"
MISSING_OBJECT_MESSAGE = "coverage object not found for pattern {pattern!r}"
AMBIGUOUS_OBJECT_MESSAGE = "multiple coverage objects found for pattern {pattern!r}"
EXPECTED_SINGLE_RECORD = 1
FIRST_ITEM_INDEX = 0
EMPTY_SUFFIX = ""


def _run_tool(arguments: tuple[str, ...], cwd: Path | None = None) -> str:
    completed = subprocess.run(
        arguments,
        cwd=cwd,
        capture_output=SUBPROCESS_CAPTURE,
        text=SUBPROCESS_TEXT,
        check=SUBPROCESS_CHECK,
    )
    if completed.returncode != PROCESS_SUCCESS:
        raise CoverageReadError(
            TOOL_FAILED_MESSAGE.format(
                tool=arguments[FIRST_ITEM_INDEX],
                code=completed.returncode,
                stderr=completed.stderr.strip(),
            )
        )
    return completed.stdout


def _rust_host() -> str:
    output = _run_tool(VERBOSE_VERSION_ARGUMENTS)
    host_line = next(
        (line for line in output.splitlines() if line.startswith(HOST_PREFIX)),
        None,
    )
    if host_line is None:
        raise CoverageReadError(MISSING_HOST_MESSAGE)
    return host_line.removeprefix(HOST_PREFIX).strip()


def _llvm_cov_path() -> Path:
    sysroot = Path(_run_tool(SYSROOT_ARGUMENTS).strip())
    executable = LLVM_COV_NAME + WINDOWS_EXECUTABLE_SUFFIX
    windows_candidate = sysroot / RUSTLIB_PATH / _rust_host() / BIN_DIRECTORY / executable
    portable_candidate = windows_candidate.with_suffix(EMPTY_SUFFIX)
    for candidate in (windows_candidate, portable_candidate):
        if candidate.is_file():
            return candidate
    raise CoverageReadError(MISSING_TOOL_MESSAGE.format(path=windows_candidate))


def _latest_profile(coverage_root: Path) -> Path:
    profiles = tuple(coverage_root.glob(PROFILE_PATTERN))
    if not profiles:
        raise CoverageReadError(MISSING_PROFILE_MESSAGE.format(path=coverage_root))
    return max(profiles, key=lambda path: path.stat().st_mtime_ns)


def _is_coverage_object(path: Path) -> bool:
    return path.is_file() and path.suffix.lower() not in EXCLUDED_OBJECT_SUFFIXES


def _resolve_object(coverage_root: Path, pattern: str) -> Path:
    candidates = tuple(
        path for path in coverage_root.glob(pattern) if _is_coverage_object(path)
    )
    if not candidates:
        raise CoverageReadError(MISSING_OBJECT_MESSAGE.format(pattern=pattern))
    newest_time = max(path.stat().st_mtime_ns for path in candidates)
    newest = tuple(
        path for path in candidates if path.stat().st_mtime_ns == newest_time
    )
    if len(newest) != EXPECTED_SINGLE_RECORD:
        raise CoverageReadError(AMBIGUOUS_OBJECT_MESSAGE.format(pattern=pattern))
    return newest[FIRST_ITEM_INDEX]


@dataclass(frozen=True, slots=True)
class LlvmCoverageReader:
    def read(
        self,
        repo_root: Path,
        project: Project,
        target: CoverageTarget,
    ) -> FileCoverage:
        project_root = repo_root / project.root.value
        coverage_root = project_root / COVERAGE_DIRECTORY
        profile = _latest_profile(coverage_root)
        objects = tuple(
            _resolve_object(coverage_root, pattern) for pattern in target.objects
        )
        arguments = (
            str(_llvm_cov_path()),
            LLVM_COV_EXPORT,
            LLVM_COV_FORMAT,
            PROFILE_ARGUMENT.format(profile=profile),
            *(OBJECT_ARGUMENT.format(object=path) for path in objects),
        )
        export = _run_tool(arguments, cwd=project_root)
        return parse_llvm_export(export, target)
