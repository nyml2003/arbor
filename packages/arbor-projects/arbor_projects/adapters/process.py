from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

from arbor_projects.domain import (
    CommandResult,
    Project,
    VerificationCommand,
)


COMMAND_NOT_FOUND_EXIT_CODE = 127
SUBPROCESS_CHECK = False


@dataclass(frozen=True, slots=True)
class SubprocessCommandRunner:
    def run(
        self,
        repo_root: Path,
        project: Project,
        command: VerificationCommand,
    ) -> CommandResult:
        project_root = repo_root / project.root.value
        try:
            completed = subprocess.run(
                command.argv,
                cwd=project_root,
                check=SUBPROCESS_CHECK,
            )
            exit_code = completed.returncode
        except FileNotFoundError:
            exit_code = COMMAND_NOT_FOUND_EXIT_CODE
        return CommandResult(command_id=command.id, exit_code=exit_code)
