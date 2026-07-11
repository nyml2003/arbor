from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from pathlib import PurePosixPath, PureWindowsPath


EMPTY_TEXT = ""
CURRENT_DIRECTORY = "."
PARENT_DIRECTORY = ".."
WINDOWS_SEPARATOR = "\\"
POSIX_SEPARATOR = "/"
EXIT_SUCCESS = 0
VALUE_ATTRIBUTE = "value"
PROJECT_ID_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
INVALID_PROJECT_ID_MESSAGE = "invalid project id: {value!r}"
INVALID_PROJECT_PATH_MESSAGE = "project path must be repository-relative: {value!r}"
EMPTY_COMMAND_ID_MESSAGE = "command id must not be empty"
EMPTY_COMMAND_ARGV_MESSAGE = "command argv must contain non-empty arguments"
EMPTY_COVERAGE_ID_MESSAGE = "coverage target id must not be empty"
EMPTY_COVERAGE_OBJECTS_MESSAGE = "coverage target must contain object patterns"
EMPTY_BRANCH_COVERAGE_ARGV_MESSAGE = "branch coverage argv must contain non-empty arguments"
EMPTY_BRANCH_COVERAGE_ROOTS_MESSAGE = "branch coverage target must contain source roots"
EMPTY_PROJECT_NAME_MESSAGE = "project name must not be empty"
DUPLICATE_COMMAND_MESSAGE = "project {project!r} has duplicate command ids"
DUPLICATE_COVERAGE_MESSAGE = "project {project!r} has duplicate coverage target ids"
INVALID_COVERAGE_COUNTS_MESSAGE = "coverage counts must satisfy 0 <= covered <= count"
INVALID_COVERAGE_MISSES_MESSAGE = "coverage miss counts must not be negative"


@dataclass(frozen=True, slots=True)
class ProjectId:
    value: str

    def __post_init__(self) -> None:
        if not PROJECT_ID_PATTERN.fullmatch(self.value):
            raise ValueError(INVALID_PROJECT_ID_MESSAGE.format(value=self.value))


@dataclass(frozen=True, slots=True, init=False)
class ProjectPath:
    value: PurePosixPath

    def __init__(self, value: str | PurePosixPath) -> None:
        raw = str(value).replace(WINDOWS_SEPARATOR, POSIX_SEPARATOR)
        path = PurePosixPath(raw)
        is_invalid = (
            raw == EMPTY_TEXT
            or path == PurePosixPath(CURRENT_DIRECTORY)
            or path.is_absolute()
            or PureWindowsPath(raw).is_absolute()
            or PARENT_DIRECTORY in path.parts
        )
        if is_invalid:
            raise ValueError(INVALID_PROJECT_PATH_MESSAGE.format(value=raw))
        object.__setattr__(self, VALUE_ATTRIBUTE, path)


class ProjectKind(str, Enum):
    FRAMEWORK = "framework"
    PROOF = "proof"
    INFRASTRUCTURE = "infrastructure"
    PRODUCT = "product"


@dataclass(frozen=True, slots=True)
class VerificationCommand:
    id: str
    argv: tuple[str, ...]

    def __post_init__(self) -> None:
        if not self.id.strip():
            raise ValueError(EMPTY_COMMAND_ID_MESSAGE)
        if not self.argv or any(not argument for argument in self.argv):
            raise ValueError(EMPTY_COMMAND_ARGV_MESSAGE)


@dataclass(frozen=True, slots=True)
class CoverageTarget:
    id: str
    source: ProjectPath
    objects: tuple[str, ...]

    def __post_init__(self) -> None:
        if not self.id.strip():
            raise ValueError(EMPTY_COVERAGE_ID_MESSAGE)
        if not self.objects or any(not pattern for pattern in self.objects):
            raise ValueError(EMPTY_COVERAGE_OBJECTS_MESSAGE)


@dataclass(frozen=True, slots=True)
class BranchCoverageTarget:
    id: str
    argv: tuple[str, ...]
    source_roots: tuple[ProjectPath, ...]

    def __post_init__(self) -> None:
        if not self.id.strip():
            raise ValueError(EMPTY_COVERAGE_ID_MESSAGE)
        if not self.argv or any(not argument for argument in self.argv):
            raise ValueError(EMPTY_BRANCH_COVERAGE_ARGV_MESSAGE)
        if not self.source_roots:
            raise ValueError(EMPTY_BRANCH_COVERAGE_ROOTS_MESSAGE)


@dataclass(frozen=True, slots=True)
class Project:
    id: ProjectId
    name: str
    kind: ProjectKind
    root: ProjectPath
    commands: tuple[VerificationCommand, ...]
    coverage_targets: tuple[CoverageTarget, ...] = ()
    branch_coverage_targets: tuple[BranchCoverageTarget, ...] = ()

    def __post_init__(self) -> None:
        if not self.name.strip():
            raise ValueError(EMPTY_PROJECT_NAME_MESSAGE)
        command_ids = tuple(command.id for command in self.commands)
        if len(command_ids) != len(set(command_ids)):
            raise ValueError(DUPLICATE_COMMAND_MESSAGE.format(project=self.id.value))
        coverage_ids = tuple(target.id for target in self.coverage_targets) + tuple(
            target.id for target in self.branch_coverage_targets
        )
        if len(coverage_ids) != len(set(coverage_ids)):
            raise ValueError(DUPLICATE_COVERAGE_MESSAGE.format(project=self.id.value))


@dataclass(frozen=True, slots=True)
class CoverageMetric:
    count: int
    covered: int

    def __post_init__(self) -> None:
        invalid = (
            self.count < EXIT_SUCCESS
            or self.covered < EXIT_SUCCESS
            or self.covered > self.count
        )
        if invalid:
            raise ValueError(INVALID_COVERAGE_COUNTS_MESSAGE)

    @property
    def complete(self) -> bool:
        return self.covered == self.count


@dataclass(frozen=True, slots=True)
class FileCoverage:
    regions: CoverageMetric
    functions: CoverageMetric
    lines: CoverageMetric

    @property
    def complete(self) -> bool:
        return self.regions.complete and self.functions.complete and self.lines.complete


@dataclass(frozen=True, slots=True)
class BranchCoverage:
    lines: CoverageMetric
    functions: CoverageMetric
    branches: CoverageMetric
    missed_lines: int
    missed_branches: int

    def __post_init__(self) -> None:
        if self.missed_lines < EXIT_SUCCESS or self.missed_branches < EXIT_SUCCESS:
            raise ValueError(INVALID_COVERAGE_MISSES_MESSAGE)

    @property
    def complete(self) -> bool:
        return (
            self.lines.complete
            and self.functions.complete
            and self.branches.complete
            and self.missed_lines == EXIT_SUCCESS
            and self.missed_branches == EXIT_SUCCESS
        )


@dataclass(frozen=True, slots=True)
class CommandResult:
    command_id: str
    exit_code: int

    @property
    def passed(self) -> bool:
        return self.exit_code == EXIT_SUCCESS


@dataclass(frozen=True, slots=True)
class Diagnostic:
    code: str
    message: str


@dataclass(frozen=True, slots=True)
class VerificationReport:
    project_id: ProjectId
    found: bool
    command_results: tuple[CommandResult, ...] = ()
    coverage_results: tuple[FileCoverage, ...] = ()
    branch_coverage_results: tuple[BranchCoverage, ...] = ()
    diagnostics: tuple[Diagnostic, ...] = ()

    @property
    def passed(self) -> bool:
        return (
            self.found
            and not self.diagnostics
            and all(result.passed for result in self.command_results)
            and all(result.complete for result in self.coverage_results)
            and all(result.complete for result in self.branch_coverage_results)
        )


__all__ = [
    "BranchCoverage",
    "BranchCoverageTarget",
    "CommandResult",
    "CoverageMetric",
    "CoverageTarget",
    "Diagnostic",
    "FileCoverage",
    "Project",
    "ProjectId",
    "ProjectKind",
    "ProjectPath",
    "VerificationCommand",
    "VerificationReport",
]
