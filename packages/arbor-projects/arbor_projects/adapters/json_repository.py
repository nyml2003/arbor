from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from arbor_projects.domain import Project, ProjectId

from .registry_format import parse_registry


TEXT_ENCODING = "utf-8"
PROJECTS_ATTRIBUTE = "_projects"


@dataclass(frozen=True, slots=True, init=False)
class JsonProjectRepository:
    _projects: tuple[Project, ...]

    def __init__(self, path: Path) -> None:
        projects = parse_registry(path.read_text(encoding=TEXT_ENCODING))
        object.__setattr__(self, PROJECTS_ATTRIBUTE, projects)

    def list(self) -> tuple[Project, ...]:
        return self._projects

    def get(self, project_id: ProjectId) -> Project | None:
        return next(
            (project for project in self._projects if project.id == project_id),
            None,
        )
