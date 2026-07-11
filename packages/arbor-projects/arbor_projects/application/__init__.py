from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol

from arbor_projects.domain import (
    BranchCoverage,
    BranchCoverageTarget,
    CommandResult,
    CoverageTarget,
    Diagnostic,
    FileCoverage,
    Project,
    ProjectId,
    VerificationCommand,
    VerificationReport,
)


PROJECT_FOUND = True
PROJECT_MISSING = False
PROJECT_NOT_FOUND_CODE = "project_not_found"
PROJECT_NOT_FOUND_MESSAGE = "project {project!r} is not registered"
COVERAGE_READ_FAILED_CODE = "coverage_read_failed"
COVERAGE_READ_FAILED_MESSAGE = "coverage target {target!r} failed: {reason}"
LAST_RESULT_INDEX = -1


class ProjectRepository(Protocol):
    def list(self) -> tuple[Project, ...]: ...

    def get(self, project_id: ProjectId) -> Project | None: ...


class CommandRunner(Protocol):
    def run(
        self,
        repo_root: Path,
        project: Project,
        command: VerificationCommand,
    ) -> CommandResult: ...


class CoverageReader(Protocol):
    def read(
        self,
        repo_root: Path,
        project: Project,
        target: CoverageTarget,
    ) -> FileCoverage: ...


class BranchCoverageReader(Protocol):
    def read(
        self,
        repo_root: Path,
        project: Project,
        target: BranchCoverageTarget,
    ) -> BranchCoverage: ...


class CoverageReadError(RuntimeError):
    pass


@dataclass(frozen=True, slots=True)
class ProjectVerifier:
    repository: ProjectRepository
    runner: CommandRunner
    coverage_reader: CoverageReader
    branch_coverage_reader: BranchCoverageReader
    repo_root: Path

    def verify(self, project_id: ProjectId) -> VerificationReport:
        project = self.repository.get(project_id)
        if project is None:
            return self._missing_project_report(project_id)

        command_results = self._run_commands(project)
        if command_results and not command_results[LAST_RESULT_INDEX].passed:
            return VerificationReport(
                project_id=project_id,
                found=PROJECT_FOUND,
                command_results=command_results,
            )

        coverage_report = self._verify_coverage(project, command_results)
        if not coverage_report.passed:
            return coverage_report
        return self._verify_branch_coverage(project, coverage_report)

    def _missing_project_report(self, project_id: ProjectId) -> VerificationReport:
        diagnostic = Diagnostic(
            code=PROJECT_NOT_FOUND_CODE,
            message=PROJECT_NOT_FOUND_MESSAGE.format(project=project_id.value),
        )
        return VerificationReport(
            project_id=project_id,
            found=PROJECT_MISSING,
            diagnostics=(diagnostic,),
        )

    def _run_commands(self, project: Project) -> tuple[CommandResult, ...]:
        results: list[CommandResult] = []
        for command in project.commands:
            result = self.runner.run(self.repo_root, project, command)
            results.append(result)
            if not result.passed:
                break
        return tuple(results)

    def _verify_coverage(
        self,
        project: Project,
        command_results: tuple[CommandResult, ...],
    ) -> VerificationReport:
        coverage_results: list[FileCoverage] = []
        for target in project.coverage_targets:
            try:
                coverage = self.coverage_reader.read(self.repo_root, project, target)
            except CoverageReadError as error:
                diagnostic = Diagnostic(
                    code=COVERAGE_READ_FAILED_CODE,
                    message=COVERAGE_READ_FAILED_MESSAGE.format(
                        target=target.id,
                        reason=error,
                    ),
                )
                return VerificationReport(
                    project_id=project.id,
                    found=PROJECT_FOUND,
                    command_results=command_results,
                    coverage_results=tuple(coverage_results),
                    diagnostics=(diagnostic,),
                )
            coverage_results.append(coverage)

        return VerificationReport(
            project_id=project.id,
            found=PROJECT_FOUND,
            command_results=command_results,
            coverage_results=tuple(coverage_results),
        )

    def _verify_branch_coverage(
        self,
        project: Project,
        coverage_report: VerificationReport,
    ) -> VerificationReport:
        branch_results: list[BranchCoverage] = []
        for target in project.branch_coverage_targets:
            try:
                coverage = self.branch_coverage_reader.read(
                    self.repo_root,
                    project,
                    target,
                )
            except CoverageReadError as error:
                diagnostic = Diagnostic(
                    code=COVERAGE_READ_FAILED_CODE,
                    message=COVERAGE_READ_FAILED_MESSAGE.format(
                        target=target.id,
                        reason=error,
                    ),
                )
                return VerificationReport(
                    project_id=project.id,
                    found=PROJECT_FOUND,
                    command_results=coverage_report.command_results,
                    coverage_results=coverage_report.coverage_results,
                    branch_coverage_results=tuple(branch_results),
                    diagnostics=(diagnostic,),
                )
            branch_results.append(coverage)

        return VerificationReport(
            project_id=project.id,
            found=PROJECT_FOUND,
            command_results=coverage_report.command_results,
            coverage_results=coverage_report.coverage_results,
            branch_coverage_results=tuple(branch_results),
        )


__all__ = [
    "BranchCoverageReader",
    "CommandRunner",
    "CoverageReadError",
    "CoverageReader",
    "ProjectRepository",
    "ProjectVerifier",
]
