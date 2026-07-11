from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory
import json
import unittest

from arbor_projects.cli import EXIT_FAILURE, EXIT_SUCCESS, main


class CliTests(unittest.TestCase):
    def write_registry(self, directory: str) -> Path:
        registry = Path(directory, "projects.json")
        registry.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "projects": [
                        {
                            "id": "tetris",
                            "name": "Tetris",
                            "kind": "proof",
                            "root": "apps/tetris",
                            "commands": [],
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )
        return registry

    def test_list_prints_registered_projects(self) -> None:
        with TemporaryDirectory() as directory:
            registry = self.write_registry(directory)
            output = StringIO()

            with redirect_stdout(output):
                exit_code = main(["--registry", str(registry), "list"])

        self.assertEqual(exit_code, EXIT_SUCCESS)
        self.assertIn("tetris", output.getvalue())
        self.assertIn("apps/tetris", output.getvalue())

    def test_verify_missing_project_returns_failure_without_running_tools(self) -> None:
        with TemporaryDirectory() as directory:
            registry = self.write_registry(directory)
            output = StringIO()

            with redirect_stdout(output):
                exit_code = main(
                    [
                        "--registry",
                        str(registry),
                        "--repo-root",
                        directory,
                        "verify",
                        "missing",
                    ]
                )

        self.assertEqual(exit_code, EXIT_FAILURE)
        self.assertIn("project_not_found", output.getvalue())


if __name__ == "__main__":
    unittest.main()
