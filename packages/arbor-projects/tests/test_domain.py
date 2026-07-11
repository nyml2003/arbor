from pathlib import PurePosixPath
import unittest

from arbor_projects.domain import (
    CommandResult,
    CoverageMetric,
    CoverageTarget,
    FileCoverage,
    Project,
    ProjectId,
    ProjectKind,
    ProjectPath,
    VerificationCommand,
    VerificationReport,
)


class DomainModelTests(unittest.TestCase):
    def test_project_id_accepts_lowercase_kebab_case(self) -> None:
        self.assertEqual(ProjectId("gen3-game").value, "gen3-game")

    def test_project_id_rejects_invalid_values(self) -> None:
        for value in ("", "Gen3", "gen3 game", "../gen3"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ProjectId(value)

    def test_project_path_accepts_a_repo_relative_path(self) -> None:
        path = ProjectPath("apps/tetris")

        self.assertEqual(path.value, PurePosixPath("apps/tetris"))

    def test_project_path_rejects_absolute_and_parent_paths(self) -> None:
        for value in ("C:/code/arbor", "/code/arbor", "../outside", "apps/../outside"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                ProjectPath(value)

    def test_command_requires_an_id_and_argv(self) -> None:
        with self.assertRaises(ValueError):
            VerificationCommand("", ("cargo", "test"))
        with self.assertRaises(ValueError):
            VerificationCommand("test", ())

    def test_project_rejects_duplicate_command_ids(self) -> None:
        command = VerificationCommand("test", ("cargo", "test"))

        with self.assertRaises(ValueError):
            Project(
                id=ProjectId("tetris"),
                name="Tetris",
                kind=ProjectKind.PROOF,
                root=ProjectPath("apps/tetris"),
                commands=(command, command),
            )

    def test_project_requires_a_name_and_unique_coverage_target_ids(self) -> None:
        target = CoverageTarget(
            "view",
            ProjectPath("examples/view.py"),
            ("view-*",),
        )
        with self.assertRaises(ValueError):
            Project(
                id=ProjectId("tetris"),
                name="",
                kind=ProjectKind.PROOF,
                root=ProjectPath("apps/tetris"),
                commands=(),
            )
        with self.assertRaises(ValueError):
            Project(
                id=ProjectId("tetris"),
                name="Tetris",
                kind=ProjectKind.PROOF,
                root=ProjectPath("apps/tetris"),
                commands=(),
                coverage_targets=(target, target),
            )

    def test_coverage_target_requires_objects_and_a_relative_source(self) -> None:
        with self.assertRaises(ValueError):
            CoverageTarget("", ProjectPath("examples/view.py"), ("view-*",))
        with self.assertRaises(ValueError):
            CoverageTarget("view", ProjectPath("examples/view.py"), ())
        with self.assertRaises(ValueError):
            CoverageTarget("view", ProjectPath("../view.py"), ("view-*",))

    def test_coverage_metric_rejects_invalid_counts(self) -> None:
        for count, covered in ((-1, 0), (1, -1), (1, 2)):
            with self.subTest(count=count, covered=covered), self.assertRaises(ValueError):
                CoverageMetric(count, covered)

    def test_file_coverage_is_complete_only_when_every_metric_is_complete(self) -> None:
        complete = FileCoverage(
            regions=CoverageMetric(10, 10),
            functions=CoverageMetric(4, 4),
            lines=CoverageMetric(20, 20),
        )
        incomplete = FileCoverage(
            regions=CoverageMetric(10, 9),
            functions=CoverageMetric(4, 4),
            lines=CoverageMetric(20, 20),
        )

        self.assertTrue(complete.complete)
        self.assertFalse(incomplete.complete)

    def test_verification_report_requires_all_commands_and_coverage_to_pass(self) -> None:
        complete = FileCoverage(
            CoverageMetric(1, 1), CoverageMetric(1, 1), CoverageMetric(1, 1)
        )
        report = VerificationReport(
            project_id=ProjectId("tetris"),
            found=True,
            command_results=(CommandResult("test", 0),),
            coverage_results=(complete,),
        )

        self.assertTrue(report.passed)
        self.assertFalse(
            VerificationReport(
                project_id=ProjectId("tetris"),
                found=True,
                command_results=(CommandResult("test", 1),),
            ).passed
        )


if __name__ == "__main__":
    unittest.main()
