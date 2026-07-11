from __future__ import annotations

import argparse
from pathlib import Path
from typing import Sequence

from arbor_projects.adapters import (
    JsonProjectRepository,
    LcovBranchCoverageReader,
    LlvmCoverageReader,
    RegistryFormatError,
    SubprocessCommandRunner,
)
from arbor_projects.application import ProjectVerifier
from arbor_projects.domain import ProjectId, VerificationReport


EXIT_SUCCESS = 0
EXIT_FAILURE = 1
EXIT_USAGE = 2
PACKAGE_PARENT_INDEX = 1
REPOSITORY_PARENT_INDEX = 3
DEFAULT_REGISTRY_NAME = "projects.json"
PROGRAM_NAME = "arbor-projects"
PROGRAM_DESCRIPTION = "Run local verification for registered Arbor projects."
REGISTRY_OPTION = "--registry"
REPOSITORY_OPTION = "--repo-root"
REGISTRY_DESTINATION = "registry"
REPOSITORY_DESTINATION = "repo_root"
COMMAND_DESTINATION = "command"
PROJECT_DESTINATION = "project"
LIST_COMMAND = "list"
VERIFY_COMMAND = "verify"
REGISTRY_HELP = "path to the project registry"
REPOSITORY_HELP = "path to the Arbor repository root"
LIST_HELP = "list registered projects"
VERIFY_HELP = "verify one registered project"
PROJECT_HELP = "registered project id"
ARGUMENT_REQUIRED = True
LIST_ROW = "{id}\t{kind}\t{root}"
COMMAND_ROW = "{status}\tcommand\t{id}\texit={exit_code}"
COVERAGE_ROW = (
    "{status}\tcoverage\tregions={regions_covered}/{regions_count}"
    "\tfunctions={functions_covered}/{functions_count}"
    "\tlines={lines_covered}/{lines_count}"
)
BRANCH_COVERAGE_ROW = (
    "{status}\tbranch-coverage"
    "\tlines={lines_covered}/{lines_count}"
    "\tfunctions={functions_covered}/{functions_count}"
    "\tbranches={branches_covered}/{branches_count}"
    "\tmissed-lines={missed_lines}\tmissed-branches={missed_branches}"
)
DIAGNOSTIC_ROW = "FAIL\t{code}\t{message}"
STATUS_PASS = "PASS"
STATUS_FAIL = "FAIL"
REGISTRY_ERROR_ROW = "ERROR\tregistry_invalid\t{message}"
PROJECT_ID_ERROR_ROW = "ERROR\tproject_id_invalid\t{message}"


def _package_root() -> Path:
    return Path(__file__).resolve().parents[PACKAGE_PARENT_INDEX]


def _repository_root() -> Path:
    return Path(__file__).resolve().parents[REPOSITORY_PARENT_INDEX]


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=PROGRAM_NAME,
        description=PROGRAM_DESCRIPTION,
    )
    parser.add_argument(
        REGISTRY_OPTION,
        type=Path,
        default=_package_root() / DEFAULT_REGISTRY_NAME,
        help=REGISTRY_HELP,
    )
    parser.add_argument(
        REPOSITORY_OPTION,
        type=Path,
        default=_repository_root(),
        help=REPOSITORY_HELP,
    )
    commands = parser.add_subparsers(
        dest=COMMAND_DESTINATION,
        required=ARGUMENT_REQUIRED,
    )
    commands.add_parser(LIST_COMMAND, help=LIST_HELP)
    verify = commands.add_parser(VERIFY_COMMAND, help=VERIFY_HELP)
    verify.add_argument(PROJECT_DESTINATION, help=PROJECT_HELP)
    return parser


def _status(passed: bool) -> str:
    return STATUS_PASS if passed else STATUS_FAIL


def _print_report(report: VerificationReport) -> None:
    for result in report.command_results:
        print(
            COMMAND_ROW.format(
                status=_status(result.passed),
                id=result.command_id,
                exit_code=result.exit_code,
            )
        )
    for coverage in report.coverage_results:
        print(
            COVERAGE_ROW.format(
                status=_status(coverage.complete),
                regions_covered=coverage.regions.covered,
                regions_count=coverage.regions.count,
                functions_covered=coverage.functions.covered,
                functions_count=coverage.functions.count,
                lines_covered=coverage.lines.covered,
                lines_count=coverage.lines.count,
            )
        )
    for coverage in report.branch_coverage_results:
        print(
            BRANCH_COVERAGE_ROW.format(
                status=_status(coverage.complete),
                lines_covered=coverage.lines.covered,
                lines_count=coverage.lines.count,
                functions_covered=coverage.functions.covered,
                functions_count=coverage.functions.count,
                branches_covered=coverage.branches.covered,
                branches_count=coverage.branches.count,
                missed_lines=coverage.missed_lines,
                missed_branches=coverage.missed_branches,
            )
        )
    for diagnostic in report.diagnostics:
        print(
            DIAGNOSTIC_ROW.format(
                code=diagnostic.code,
                message=diagnostic.message,
            )
        )


def _list_projects(repository: JsonProjectRepository) -> int:
    for project in repository.list():
        print(
            LIST_ROW.format(
                id=project.id.value,
                kind=project.kind.value,
                root=project.root.value,
            )
        )
    return EXIT_SUCCESS


def _verify_project(
    repository: JsonProjectRepository,
    repo_root: Path,
    project_value: str,
) -> int:
    try:
        project_id = ProjectId(project_value)
    except ValueError as error:
        print(PROJECT_ID_ERROR_ROW.format(message=error))
        return EXIT_USAGE
    verifier = ProjectVerifier(
        repository=repository,
        runner=SubprocessCommandRunner(),
        coverage_reader=LlvmCoverageReader(),
        branch_coverage_reader=LcovBranchCoverageReader(),
        repo_root=repo_root,
    )
    report = verifier.verify(project_id)
    _print_report(report)
    return EXIT_SUCCESS if report.passed else EXIT_FAILURE


def main(argv: Sequence[str] | None = None) -> int:
    arguments = build_parser().parse_args(argv)
    registry_path = getattr(arguments, REGISTRY_DESTINATION)
    try:
        repository = JsonProjectRepository(registry_path)
    except (OSError, RegistryFormatError) as error:
        print(REGISTRY_ERROR_ROW.format(message=error))
        return EXIT_USAGE

    command = getattr(arguments, COMMAND_DESTINATION)
    if command == LIST_COMMAND:
        return _list_projects(repository)
    return _verify_project(
        repository,
        getattr(arguments, REPOSITORY_DESTINATION),
        getattr(arguments, PROJECT_DESTINATION),
    )
