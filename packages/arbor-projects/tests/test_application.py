from pathlib import Path
import unittest

from arbor_projects.application import CoverageReadError, ProjectVerifier
from arbor_projects.domain import (
    BranchCoverage,
    BranchCoverageTarget,
    CommandResult,
    CoverageMetric,
    CoverageTarget,
    FileCoverage,
    Project,
    ProjectId,
    ProjectKind,
    ProjectPath,
    VerificationCommand,
)


def project() -> Project:
    return Project(
        id=ProjectId("tetris"),
        name="Tetris",
        kind=ProjectKind.PROOF,
        root=ProjectPath("apps/tetris"),
        commands=(
            VerificationCommand("fmt", ("cargo", "fmt", "--check")),
            VerificationCommand("test", ("cargo", "test")),
        ),
        coverage_targets=(
            CoverageTarget(
                "terminal-view",
                ProjectPath("examples/terminal/view.rs"),
                ("debug/examples/terminal-*",),
            ),
        ),
    )


class MemoryRepository:
    def __init__(self, projects: tuple[Project, ...]) -> None:
        self.projects = projects

    def list(self) -> tuple[Project, ...]:
        return self.projects

    def get(self, project_id: ProjectId) -> Project | None:
        return next((item for item in self.projects if item.id == project_id), None)


class FakeRunner:
    def __init__(self, exit_codes: dict[str, int]) -> None:
        self.exit_codes = exit_codes
        self.calls: list[str] = []

    def run(self, repo_root: Path, target: Project, command: VerificationCommand) -> CommandResult:
        self.calls.append(command.id)
        return CommandResult(command.id, self.exit_codes.get(command.id, 0))


class FakeCoverageReader:
    def __init__(self, coverage: FileCoverage) -> None:
        self.coverage = coverage
        self.calls: list[str] = []

    def read(
        self, repo_root: Path, target: Project, coverage_target: CoverageTarget
    ) -> FileCoverage:
        self.calls.append(coverage_target.id)
        return self.coverage


class FakeBranchCoverageReader:
    def __init__(self, coverage: BranchCoverage) -> None:
        self.coverage = coverage
        self.calls: list[str] = []

    def read(
        self,
        repo_root: Path,
        target: Project,
        coverage_target: BranchCoverageTarget,
    ) -> BranchCoverage:
        self.calls.append(coverage_target.id)
        return self.coverage


class FailingCoverageReader:
    def read(
        self, repo_root: Path, target: Project, coverage_target: CoverageTarget
    ) -> FileCoverage:
        raise CoverageReadError("missing profile")


class FailingBranchCoverageReader:
    def read(
        self,
        repo_root: Path,
        target: Project,
        coverage_target: BranchCoverageTarget,
    ) -> BranchCoverage:
        raise CoverageReadError("invalid lcov")


class ProjectVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.complete = FileCoverage(
            CoverageMetric(1, 1), CoverageMetric(1, 1), CoverageMetric(1, 1)
        )
        self.complete_branches = BranchCoverage(
            CoverageMetric(1, 1),
            CoverageMetric(1, 1),
            CoverageMetric(2, 2),
            0,
            0,
        )

    def test_verify_runs_commands_in_order_then_checks_coverage(self) -> None:
        runner = FakeRunner({})
        coverage = FakeCoverageReader(self.complete)
        verifier = ProjectVerifier(
            MemoryRepository((project(),)),
            runner,
            coverage,
            FakeBranchCoverageReader(self.complete_branches),
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("tetris"))

        self.assertTrue(report.passed)
        self.assertEqual(runner.calls, ["fmt", "test"])
        self.assertEqual(coverage.calls, ["terminal-view"])

    def test_verify_stops_after_the_first_failed_command(self) -> None:
        runner = FakeRunner({"fmt": 2})
        coverage = FakeCoverageReader(self.complete)
        verifier = ProjectVerifier(
            MemoryRepository((project(),)),
            runner,
            coverage,
            FakeBranchCoverageReader(self.complete_branches),
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("tetris"))

        self.assertFalse(report.passed)
        self.assertEqual(runner.calls, ["fmt"])
        self.assertEqual(coverage.calls, [])

    def test_verify_returns_a_structured_missing_project_report(self) -> None:
        verifier = ProjectVerifier(
            MemoryRepository(()),
            FakeRunner({}),
            FakeCoverageReader(self.complete),
            FakeBranchCoverageReader(self.complete_branches),
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("missing"))

        self.assertFalse(report.found)
        self.assertFalse(report.passed)
        self.assertEqual(report.diagnostics[0].code, "project_not_found")

    def test_incomplete_file_coverage_fails_the_report(self) -> None:
        incomplete = FileCoverage(
            CoverageMetric(2, 1), CoverageMetric(1, 1), CoverageMetric(1, 1)
        )
        verifier = ProjectVerifier(
            MemoryRepository((project(),)),
            FakeRunner({}),
            FakeCoverageReader(incomplete),
            FakeBranchCoverageReader(self.complete_branches),
            Path("C:/repo"),
        )

        self.assertFalse(verifier.verify(ProjectId("tetris")).passed)

    def test_coverage_reader_failure_becomes_a_diagnostic(self) -> None:
        verifier = ProjectVerifier(
            MemoryRepository((project(),)),
            FakeRunner({}),
            FailingCoverageReader(),
            FakeBranchCoverageReader(self.complete_branches),
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("tetris"))

        self.assertFalse(report.passed)
        self.assertEqual(report.diagnostics[0].code, "coverage_read_failed")

    def test_branch_coverage_is_checked_after_other_coverage(self) -> None:
        target = BranchCoverageTarget(
            "pure-branch-coverage",
            ("cargo", "llvm-cov", "--branch", "--lcov"),
            (ProjectPath("crates/ramus-core/src"),),
        )
        ramus = Project(
            id=ProjectId("ramus"),
            name="Ramus",
            kind=ProjectKind.INFRASTRUCTURE,
            root=ProjectPath("packages/ramus"),
            commands=(),
            branch_coverage_targets=(target,),
        )
        branch_reader = FakeBranchCoverageReader(
            BranchCoverage(
                CoverageMetric(1, 1),
                CoverageMetric(1, 1),
                CoverageMetric(2, 1),
                0,
                1,
            )
        )
        verifier = ProjectVerifier(
            MemoryRepository((ramus,)),
            FakeRunner({}),
            FakeCoverageReader(self.complete),
            branch_reader,
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("ramus"))

        self.assertFalse(report.passed)
        self.assertEqual(branch_reader.calls, ["pure-branch-coverage"])

    def test_branch_coverage_reader_failure_becomes_a_diagnostic(self) -> None:
        ramus = Project(
            id=ProjectId("ramus"),
            name="Ramus",
            kind=ProjectKind.INFRASTRUCTURE,
            root=ProjectPath("packages/ramus"),
            commands=(),
            branch_coverage_targets=(
                BranchCoverageTarget(
                    "pure-branch-coverage",
                    ("cargo", "llvm-cov", "--branch", "--lcov"),
                    (ProjectPath("crates/ramus-core/src"),),
                ),
            ),
        )
        verifier = ProjectVerifier(
            MemoryRepository((ramus,)),
            FakeRunner({}),
            FakeCoverageReader(self.complete),
            FailingBranchCoverageReader(),
            Path("C:/repo"),
        )

        report = verifier.verify(ProjectId("ramus"))

        self.assertFalse(report.passed)
        self.assertEqual(report.diagnostics[0].code, "coverage_read_failed")


if __name__ == "__main__":
    unittest.main()
