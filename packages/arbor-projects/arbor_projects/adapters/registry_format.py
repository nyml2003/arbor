from __future__ import annotations

import json
from typing import Any

from arbor_projects.domain import (
    BranchCoverageTarget,
    CoverageTarget,
    Project,
    ProjectId,
    ProjectKind,
    ProjectPath,
    VerificationCommand,
)


SCHEMA_VERSION = 1
SCHEMA_VERSION_KEY = "schema_version"
PROJECTS_KEY = "projects"
ID_KEY = "id"
NAME_KEY = "name"
KIND_KEY = "kind"
ROOT_KEY = "root"
COMMANDS_KEY = "commands"
ARGV_KEY = "argv"
COVERAGE_TARGETS_KEY = "coverage_targets"
BRANCH_COVERAGE_TARGETS_KEY = "branch_coverage_targets"
SOURCE_KEY = "source"
OBJECTS_KEY = "objects"
SOURCE_ROOTS_KEY = "source_roots"
INVALID_JSON_MESSAGE = "invalid project registry JSON: {reason}"
INVALID_SCHEMA_MESSAGE = "unsupported project registry schema: {version!r}"
EXPECTED_OBJECT_MESSAGE = "{field!r} must be an object"
EXPECTED_LIST_MESSAGE = "{field!r} must be a list"
EXPECTED_STRING_MESSAGE = "{field!r} must be a string"
EXPECTED_STRING_LIST_MESSAGE = "{field!r} must contain only strings"
INVALID_PROJECT_MESSAGE = "invalid registered project: {reason}"
DUPLICATE_PROJECT_MESSAGE = "duplicate registered project id: {project!r}"
DUPLICATE_THRESHOLD = 1


class RegistryFormatError(ValueError):
    pass


def _empty_list() -> list[object]:
    return []


def _require_object(value: object, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise RegistryFormatError(EXPECTED_OBJECT_MESSAGE.format(field=field))
    return value


def _require_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise RegistryFormatError(EXPECTED_LIST_MESSAGE.format(field=field))
    return value


def _require_string(value: object, field: str) -> str:
    if not isinstance(value, str):
        raise RegistryFormatError(EXPECTED_STRING_MESSAGE.format(field=field))
    return value


def _require_string_list(value: object, field: str) -> tuple[str, ...]:
    items = _require_list(value, field)
    if any(not isinstance(item, str) for item in items):
        raise RegistryFormatError(EXPECTED_STRING_LIST_MESSAGE.format(field=field))
    return tuple(items)


def _parse_command(value: object) -> VerificationCommand:
    item = _require_object(value, COMMANDS_KEY)
    return VerificationCommand(
        id=_require_string(item.get(ID_KEY), ID_KEY),
        argv=_require_string_list(item.get(ARGV_KEY), ARGV_KEY),
    )


def _parse_coverage_target(value: object) -> CoverageTarget:
    item = _require_object(value, COVERAGE_TARGETS_KEY)
    return CoverageTarget(
        id=_require_string(item.get(ID_KEY), ID_KEY),
        source=ProjectPath(_require_string(item.get(SOURCE_KEY), SOURCE_KEY)),
        objects=_require_string_list(item.get(OBJECTS_KEY), OBJECTS_KEY),
    )


def _parse_branch_coverage_target(value: object) -> BranchCoverageTarget:
    item = _require_object(value, BRANCH_COVERAGE_TARGETS_KEY)
    source_roots = _require_string_list(item.get(SOURCE_ROOTS_KEY), SOURCE_ROOTS_KEY)
    return BranchCoverageTarget(
        id=_require_string(item.get(ID_KEY), ID_KEY),
        argv=_require_string_list(item.get(ARGV_KEY), ARGV_KEY),
        source_roots=tuple(ProjectPath(root) for root in source_roots),
    )


def _parse_project(value: object) -> Project:
    item = _require_object(value, PROJECTS_KEY)
    commands = tuple(
        _parse_command(command)
        for command in _require_list(item.get(COMMANDS_KEY), COMMANDS_KEY)
    )
    coverage_values = item.get(COVERAGE_TARGETS_KEY, _empty_list())
    coverage_targets = tuple(
        _parse_coverage_target(target)
        for target in _require_list(coverage_values, COVERAGE_TARGETS_KEY)
    )
    branch_coverage_values = item.get(BRANCH_COVERAGE_TARGETS_KEY, _empty_list())
    branch_coverage_targets = tuple(
        _parse_branch_coverage_target(target)
        for target in _require_list(
            branch_coverage_values,
            BRANCH_COVERAGE_TARGETS_KEY,
        )
    )
    try:
        return Project(
            id=ProjectId(_require_string(item.get(ID_KEY), ID_KEY)),
            name=_require_string(item.get(NAME_KEY), NAME_KEY),
            kind=ProjectKind(_require_string(item.get(KIND_KEY), KIND_KEY)),
            root=ProjectPath(_require_string(item.get(ROOT_KEY), ROOT_KEY)),
            commands=commands,
            coverage_targets=coverage_targets,
            branch_coverage_targets=branch_coverage_targets,
        )
    except (TypeError, ValueError) as error:
        raise RegistryFormatError(
            INVALID_PROJECT_MESSAGE.format(reason=error)
        ) from error


def parse_registry(text: str) -> tuple[Project, ...]:
    try:
        payload = json.loads(text)
    except json.JSONDecodeError as error:
        raise RegistryFormatError(
            INVALID_JSON_MESSAGE.format(reason=error)
        ) from error
    root = _require_object(payload, PROJECTS_KEY)
    schema_version = root.get(SCHEMA_VERSION_KEY)
    if schema_version != SCHEMA_VERSION:
        raise RegistryFormatError(
            INVALID_SCHEMA_MESSAGE.format(version=schema_version)
        )
    projects = tuple(
        _parse_project(project)
        for project in _require_list(root.get(PROJECTS_KEY), PROJECTS_KEY)
    )
    project_ids = tuple(project.id for project in projects)
    if len(project_ids) != len(set(project_ids)):
        duplicate = next(
            project_id
            for project_id in project_ids
            if project_ids.count(project_id) > DUPLICATE_THRESHOLD
        )
        raise RegistryFormatError(
            DUPLICATE_PROJECT_MESSAGE.format(project=duplicate.value)
        )
    return projects
