from pathlib import Path
from tempfile import TemporaryDirectory
import json
import unittest

from arbor_projects.adapters import (
    JsonProjectRepository,
    LcovFormatError,
    RegistryFormatError,
    parse_lcov_branch_coverage,
    parse_llvm_export,
)
from arbor_projects.domain import (
    BranchCoverageTarget,
    CoverageTarget,
    ProjectId,
    ProjectPath,
)


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
                    "branch_coverage_targets": [
                        {
                            "id": "pure-branch-coverage",
                            "argv": ["cargo", "llvm-cov", "--branch", "--lcov"],
                            "source_roots": ["src"],
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
            self.assertEqual(
                loaded.branch_coverage_targets[0].source_roots,
                (ProjectPath("src"),),
            )

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


class LcovBranchParserTests(unittest.TestCase):
    def setUp(self) -> None:
        self.target = BranchCoverageTarget(
            "pure-branch-coverage",
            ("cargo", "llvm-cov", "--branch", "--lcov"),
            (ProjectPath("src"),),
        )

    def test_parser_returns_exact_branch_coverage_for_matching_sources(self) -> None:
        report = "\n".join(
            (
                "TN:",
                "SF:C:\\repo\\packages\\ramus\\crates\\ramus-core\\src\\lib.rs",
                "FNDA:1,ramus_core::run",
                "FNF:1",
                "FNH:1",
                "BRDA:10,0,0,1",
                "BRDA:10,0,1,2",
                "BRF:2",
                "BRH:2",
                "DA:10,2",
                "LF:1",
                "LH:1",
                "end_of_record",
                "SF:C:/repo/packages/ramus/crates/ramus-core/tests/public.rs",
                "FNF:0",
                "FNH:0",
                "BRF:0",
                "BRH:0",
                "LF:0",
                "LH:0",
                "end_of_record",
            )
        )

        coverage = parse_lcov_branch_coverage(report, self.target)

        self.assertTrue(coverage.complete)
        self.assertEqual(coverage.branches.count, 2)
        self.assertEqual(coverage.branches.covered, 2)
        self.assertEqual(coverage.missed_branches, 0)

    def test_parser_rejects_missing_branch_summary_fields(self) -> None:
        report = "\n".join(
            (
                "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                "FNDA:1,ramus_core::run",
                "FNF:1",
                "FNH:1",
                "BRDA:10,0,0,1",
                "BRF:1",
                "DA:10,1",
                "LF:1",
                "LH:1",
                "end_of_record",
            )
        )

        with self.assertRaises(LcovFormatError):
            parse_lcov_branch_coverage(report, self.target)

    def test_parser_rejects_invalid_lcov_content(self) -> None:
        invalid_reports = (
            "\n".join(
                (
                    "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                    "BRDA:line,0,0,1",
                    "BRF:1",
                    "BRH:1",
                    "end_of_record",
                )
            ),
            "\n".join(
                (
                    "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                    "BRDA:10,0,0",
                    "BRF:1",
                    "BRH:1",
                    "end_of_record",
                )
            ),
            "\n".join(
                (
                    "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                    "FNDA:1,ramus_core::run",
                    "FNF:1",
                    "FNH:1",
                    "BRDA:10,0,0,1",
                    "BRF:2",
                    "BRH:3",
                    "DA:10,1",
                    "LF:1",
                    "LH:1",
                    "end_of_record",
                )
            ),
        )
        for report in invalid_reports:
            with self.subTest(report=report), self.assertRaises(LcovFormatError):
                parse_lcov_branch_coverage(report, self.target)

    def test_parser_reports_incomplete_branch_coverage(self) -> None:
        report = "\n".join(
            (
                "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                "FNDA:1,ramus_core::run",
                "FNF:1",
                "FNH:1",
                "BRDA:10,0,0,1",
                "BRDA:10,0,1,0",
                "BRDA:11,0,0,-",
                "BRF:3",
                "BRH:1",
                "DA:10,1",
                "LF:1",
                "LH:1",
                "end_of_record",
            )
        )

        coverage = parse_lcov_branch_coverage(report, self.target)

        self.assertFalse(coverage.complete)
        self.assertEqual(coverage.branches.covered, 1)
        self.assertEqual(coverage.missed_branches, 2)

    def test_parser_reports_incomplete_line_coverage(self) -> None:
        report = "\n".join(
            (
                "SF:C:/repo/packages/ramus/crates/ramus-core/src/lib.rs",
                "FNF:0",
                "FNH:0",
                "BRF:0",
                "BRH:0",
                "DA:10,0",
                "LF:1",
                "LH:0",
                "end_of_record",
            )
        )

        coverage = parse_lcov_branch_coverage(report, self.target)

        self.assertFalse(coverage.complete)
        self.assertEqual(coverage.missed_lines, 1)

    def test_parser_rejects_invalid_record_structure(self) -> None:
        invalid_reports = (
            "SF:\nBRF:0\nBRH:0\nend_of_record",
            "end_of_record",
            "SF:src/lib.rs\nSF:src/main.rs",
            "SF:src/lib.rs\nBRF:0\nBRH:0",
            "SF:src/lib.rs\nBRF:-1\nBRH:0\nend_of_record",
            "SF:src/lib.rs\nBRF:value\nBRH:0\nend_of_record",
            "SF:src/lib.rs\nBRF:0\nBRF:0\nBRH:0\nend_of_record",
            "SF:src/lib.rs\nBRF:0\nBRH:0\nBRH:0\nend_of_record",
            "SF:src/lib.rs\nBRF:0\nBRH:0\nLF:0\nLH:0\nend_of_record",
            (
                "SF:other/lib.rs\nFNF:0\nFNH:0\nBRF:0\nBRH:0"
                "\nLF:0\nLH:0\nend_of_record"
            ),
        )
        for report in invalid_reports:
            with self.subTest(report=report), self.assertRaises(LcovFormatError):
                parse_lcov_branch_coverage(report, self.target)

    def test_parser_rejects_invalid_function_and_line_data(self) -> None:
        invalid_reports = (
            "SF:src/lib.rs\nFNDA:1\nend_of_record",
            "SF:src/lib.rs\nDA:1\nend_of_record",
            "SF:src/lib.rs\nDA:line,1\nend_of_record",
            "SF:src/lib.rs\nDA:1,value\nend_of_record",
        )
        for report in invalid_reports:
            with self.subTest(report=report), self.assertRaises(LcovFormatError):
                parse_lcov_branch_coverage(report, self.target)


class RealRegistryTests(unittest.TestCase):
    def test_real_registry_contains_the_phase_projects(self) -> None:
        registry = Path(__file__).resolve().parents[1] / "projects.json"
        repository = JsonProjectRepository(registry)

        self.assertEqual(
            {project.id.value for project in repository.list()},
            {"punctum", "tetris", "ramus", "gen3-game", "tui-chater"},
        )

    def test_punctum_registry_covers_each_completed_pure_crate(self) -> None:
        registry = Path(__file__).resolve().parents[1] / "projects.json"
        repository = JsonProjectRepository(registry)
        punctum = repository.get(ProjectId("punctum"))

        self.assertIsNotNone(punctum)
        assert punctum is not None
        self.assertEqual(
            {
                command.id
                for command in punctum.commands
                if command.id.endswith("-coverage")
            },
            {
                "grid-coverage",
                "input-coverage",
                "terminal-coverage",
                "gpu-coverage",
            },
        )
        coverage_commands = tuple(
            command
            for command in punctum.commands
            if command.id.endswith("-coverage")
        )
        self.assertTrue(
            all("--ignore-filename-regex" not in command.argv for command in coverage_commands)
        )

    def test_punctum_platform_crates_use_contract_tests_and_smoke(self) -> None:
        registry = Path(__file__).resolve().parents[1] / "projects.json"
        repository = JsonProjectRepository(registry)
        punctum = repository.get(ProjectId("punctum"))

        self.assertIsNotNone(punctum)
        assert punctum is not None
        commands = {command.id: command.argv for command in punctum.commands}
        self.assertIn("punctum-crossterm", commands["crossterm-test"])
        self.assertIn("punctum-wgpu", commands["wgpu-test"])
        self.assertIn("--ignored", commands["wgpu-headless-smoke"])
        for command_id in ("crossterm-test", "wgpu-test", "wgpu-headless-smoke"):
            self.assertNotIn("--fail-under-lines", commands[command_id])

    def test_ramus_registry_requires_real_branch_coverage(self) -> None:
        registry = Path(__file__).resolve().parents[1] / "projects.json"
        repository = JsonProjectRepository(registry)
        ramus = repository.get(ProjectId("ramus"))

        self.assertIsNotNone(ramus)
        assert ramus is not None
        target = ramus.branch_coverage_targets[0]
        self.assertIn("--branch", target.argv)
        self.assertIn("--lcov", target.argv)
        self.assertEqual(
            target.source_roots,
            (ProjectPath("src"),),
        )


if __name__ == "__main__":
    unittest.main()
