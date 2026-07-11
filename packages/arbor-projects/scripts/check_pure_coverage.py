from __future__ import annotations

from io import StringIO
from pathlib import Path
import sys
import trace
import unittest


PACKAGE_PARENT_INDEX = 1
TEST_DIRECTORY = "tests"
PURE_SOURCES = (
    "arbor_projects/domain/__init__.py",
    "arbor_projects/application/__init__.py",
    "arbor_projects/adapters/registry_format.py",
    "arbor_projects/adapters/llvm_export.py",
)
TRACE_COUNT_ENABLED = True
TRACE_OUTPUT_DISABLED = False
TEST_VERBOSITY = 0
COVERED_THRESHOLD = 0
NON_SOURCE_LINE = 0
PATH_START_INDEX = 0
EXIT_SUCCESS = 0
EXIT_FAILURE = 1
EXECUTABLE_LINES_FUNCTION = "_find_executable_linenos"
COVERAGE_ROW = "{source}: {covered}/{total} executable lines"
MISSING_ROW = "{source}: missing lines {lines}"
TEST_FAILURE_ROW = "unit tests failed under coverage"
MAIN_MODULE = "__main__"


def _run_tests(root: Path) -> bool:
    sys.path.insert(PATH_START_INDEX, str(root))
    suite = unittest.defaultTestLoader.discover(str(root / TEST_DIRECTORY))
    output = StringIO()
    result = unittest.TextTestRunner(
        stream=output,
        verbosity=TEST_VERBOSITY,
    ).run(suite)
    if not result.wasSuccessful():
        print(output.getvalue())
    sys.path.pop(PATH_START_INDEX)
    return result.wasSuccessful()


def _executable_lines(source: Path) -> set[int]:
    finder = getattr(trace, EXECUTABLE_LINES_FUNCTION)
    return {
        line
        for line in finder(str(source))
        if isinstance(line, int) and line > NON_SOURCE_LINE
    }


def _executed_lines(
    counts: dict[tuple[str, int], int],
    source: Path,
) -> set[int]:
    resolved_source = source.resolve()
    return {
        line
        for (filename, line), count in counts.items()
        if Path(filename).resolve() == resolved_source and count > COVERED_THRESHOLD
    }


def main() -> int:
    root = Path(__file__).resolve().parents[PACKAGE_PARENT_INDEX]
    tracer = trace.Trace(
        count=TRACE_COUNT_ENABLED,
        trace=TRACE_OUTPUT_DISABLED,
    )
    tests_passed = tracer.runfunc(_run_tests, root)
    counts = tracer.results().counts
    coverage_passed = True
    for relative_source in PURE_SOURCES:
        source = root / relative_source
        executable = _executable_lines(source)
        executed = _executed_lines(counts, source)
        missing = sorted(executable - executed)
        print(
            COVERAGE_ROW.format(
                source=relative_source,
                covered=len(executable) - len(missing),
                total=len(executable),
            )
        )
        if missing:
            print(MISSING_ROW.format(source=relative_source, lines=missing))
            coverage_passed = False
    if not tests_passed:
        print(TEST_FAILURE_ROW)
    return EXIT_SUCCESS if tests_passed and coverage_passed else EXIT_FAILURE


if __name__ == MAIN_MODULE:
    raise SystemExit(main())
