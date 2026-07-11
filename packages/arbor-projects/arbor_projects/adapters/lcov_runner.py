from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

from arbor_projects.application import CoverageReadError
from arbor_projects.domain import BranchCoverage, BranchCoverageTarget, Project

from .lcov import LcovFormatError, parse_lcov_branch_coverage


PROCESS_SUCCESS = 0
SUBPROCESS_CAPTURE = True
SUBPROCESS_TEXT = True
SUBPROCESS_CHECK = False
COMMAND_NOT_FOUND_MESSAGE = "branch coverage command not found: {command}"
COMMAND_FAILED_MESSAGE = "branch coverage command failed with exit code {code}: {stderr}"
INVALID_REPORT_MESSAGE = "branch coverage report failed validation: {reason}"
FIRST_ARGUMENT_INDEX = 0


@dataclass(frozen=True, slots=True)
class LcovBranchCoverageReader:
    def read(
        self,
        repo_root: Path,
        project: Project,
        target: BranchCoverageTarget,
    ) -> BranchCoverage:
        project_root = repo_root / project.root.value
        try:
            completed = subprocess.run(
                target.argv,
                cwd=project_root,
                capture_output=SUBPROCESS_CAPTURE,
                text=SUBPROCESS_TEXT,
                check=SUBPROCESS_CHECK,
            )
        except FileNotFoundError as error:
            raise CoverageReadError(
                COMMAND_NOT_FOUND_MESSAGE.format(
                    command=target.argv[FIRST_ARGUMENT_INDEX],
                )
            ) from error
        if completed.returncode != PROCESS_SUCCESS:
            raise CoverageReadError(
                COMMAND_FAILED_MESSAGE.format(
                    code=completed.returncode,
                    stderr=completed.stderr.strip(),
                )
            )
        try:
            return parse_lcov_branch_coverage(completed.stdout, target)
        except LcovFormatError as error:
            raise CoverageReadError(
                INVALID_REPORT_MESSAGE.format(reason=error)
            ) from error
