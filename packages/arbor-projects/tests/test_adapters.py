from pathlib import Path
from tempfile import TemporaryDirectory
import json
import unittest

from arbor_projects.adapters import (
    JsonProjectRepository,
    RegistryFormatError,
    parse_llvm_export,
)
from arbor_projects.domain import CoverageTarget, ProjectId, ProjectPath


class JsonProjectRepositoryTests(unittest.TestCase):
    def test_repository_loads_typed_projects(self) -> None:
        payload = {
            "schema_version": 1,
            "projects": [
                {
                    "id": "tetris",
                    "name": "Tetris",
                    "kind": "proof",
                    "root": "apps/tetris",
                    "commands": [
                        {"id": "test", "argv": ["cargo", "test", "--locked"]}
                    ],
                    "coverage_targets": [
                        {
                            "id": "terminal-view",
                            "source": "examples/terminal/view.rs",
                            "objects": ["debug/examples/terminal-*"],
                        }
                    ],
                }
            ],
        }
        with TemporaryDirectory() as directory:
            path = Path(directory, "projects.json")
            path.write_text(json.dumps(payload), encoding="utf-8")

            repository = JsonProjectRepository(path)

            loaded = repository.get(ProjectId("tetris"))
            self.assertIsNotNone(loaded)
            assert loaded is not None
            self.assertEqual(loaded.root, ProjectPath("apps/tetris"))
            self.assertEqual(loaded.commands[0].argv, ("cargo", "test", "--locked"))

    def test_repository_rejects_duplicate_project_ids(self) -> None:
        project = {
            "id": "tetris",
            "name": "Tetris",
            "kind": "proof",
            "root": "apps/tetris",
            "commands": [],
        }
        with TemporaryDirectory() as directory:
            path = Path(directory, "projects.json")
            path.write_text(
                json.dumps({"schema_version": 1, "projects": [project, project]}),
                encoding="utf-8",
            )

            with self.assertRaises(RegistryFormatError):
                JsonProjectRepository(path)

    def test_repository_rejects_unknown_schema_versions(self) -> None:
        with TemporaryDirectory() as directory:
            path = Path(directory, "projects.json")
            path.write_text(
                json.dumps({"schema_version": 2, "projects": []}), encoding="utf-8"
            )

            with self.assertRaises(RegistryFormatError):
                JsonProjectRepository(path)

    def test_repository_rejects_invalid_json_and_field_shapes(self) -> None:
        invalid_payloads = (
            "[",
            json.dumps([]),
            json.dumps({"schema_version": 1, "projects": {}}),
            json.dumps(
                {
                    "schema_version": 1,
                    "projects": [
                        {
                            "id": 1,
                            "name": "Tetris",
                            "kind": "proof",
                            "root": "apps/tetris",
                            "commands": [],
                        }
                    ],
                }
            ),
            json.dumps(
                {
                    "schema_version": 1,
                    "projects": [
                        {
                            "id": "tetris",
                            "name": "Tetris",
                            "kind": "proof",
                            "root": "apps/tetris",
                            "commands": [{"id": "test", "argv": [1]}],
                        }
                    ],
                }
            ),
            json.dumps(
                {
                    "schema_version": 1,
                    "projects": [
                        {
                            "id": "tetris",
                            "name": "Tetris",
                            "kind": "unknown",
                            "root": "apps/tetris",
                            "commands": [],
                        }
                    ],
                }
            ),
        )
        for payload in invalid_payloads:
            with self.subTest(payload=payload), TemporaryDirectory() as directory:
                path = Path(directory, "projects.json")
                path.write_text(payload, encoding="utf-8")
                with self.assertRaises(RegistryFormatError):
                    JsonProjectRepository(path)


class LlvmExportParserTests(unittest.TestCase):
    def setUp(self) -> None:
        self.target = CoverageTarget(
            "terminal-view",
            ProjectPath("examples/terminal/view.rs"),
            ("debug/examples/terminal-*",),
        )

    def test_parser_returns_exact_file_metrics(self) -> None:
        payload = {
            "data": [
                {
                    "files": [
                        {
                            "filename": "C:/repo/apps/tetris/examples/terminal/view.rs",
                            "summary": {
                                "regions": {"count": 10, "covered": 10},
                                "functions": {"count": 3, "covered": 3},
                                "lines": {"count": 20, "covered": 20},
                            },
                        }
                    ]
                }
            ]
        }

        coverage = parse_llvm_export(json.dumps(payload), self.target)

        self.assertTrue(coverage.complete)
        self.assertEqual(coverage.lines.count, 20)

    def test_parser_rejects_missing_source_records(self) -> None:
        payload = {"data": [{"files": []}]}

        with self.assertRaises(RegistryFormatError):
            parse_llvm_export(json.dumps(payload), self.target)

    def test_parser_rejects_invalid_json_and_field_shapes(self) -> None:
        valid_summary = {
            "regions": {"count": 1, "covered": 1},
            "functions": {"count": 1, "covered": 1},
            "lines": {"count": 1, "covered": 1},
        }
        invalid_payloads = (
            "[",
            json.dumps([]),
            json.dumps({"data": {}}),
            json.dumps({"data": [[]]}),
            json.dumps({"data": [{"files": {}}]}),
            json.dumps({"data": [{"files": [[]]}]}),
            json.dumps(
                {
                    "data": [
                        {
                            "files": [
                                {
                                    "filename": 1,
                                    "summary": valid_summary,
                                }
                            ]
                        }
                    ]
                }
            ),
            json.dumps(
                {
                    "data": [
                        {
                            "files": [
                                {
                                    "filename": "examples/terminal/view.rs",
                                    "summary": {
                                        **valid_summary,
                                        "lines": {"count": "1", "covered": 1},
                                    },
                                }
                            ]
                        }
                    ]
                }
            ),
        )
        for payload in invalid_payloads:
            with self.subTest(payload=payload), self.assertRaises(RegistryFormatError):
                parse_llvm_export(payload, self.target)


class RealRegistryTests(unittest.TestCase):
    def test_real_registry_contains_the_phase_projects(self) -> None:
        registry = Path(__file__).resolve().parents[1] / "projects.json"
        repository = JsonProjectRepository(registry)

        self.assertEqual(
            {project.id.value for project in repository.list()},
            {"punctum", "tetris", "ramus", "gen3-game", "tui-chater"},
        )


if __name__ == "__main__":
    unittest.main()
