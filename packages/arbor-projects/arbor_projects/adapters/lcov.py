from __future__ import annotations

from dataclasses import dataclass

from arbor_projects.domain import (
    BranchCoverage,
    BranchCoverageTarget,
    CoverageMetric,
    ProjectPath,
)


SOURCE_PREFIX = "SF:"
FUNCTION_DATA_PREFIX = "FNDA:"
FUNCTIONS_FOUND_PREFIX = "FNF:"
FUNCTIONS_HIT_PREFIX = "FNH:"
BRANCH_DATA_PREFIX = "BRDA:"
BRANCHES_FOUND_PREFIX = "BRF:"
BRANCHES_HIT_PREFIX = "BRH:"
LINE_DATA_PREFIX = "DA:"
LINES_FOUND_PREFIX = "LF:"
LINES_HIT_PREFIX = "LH:"
END_OF_RECORD = "end_of_record"
NOT_TAKEN = "-"
WINDOWS_SEPARATOR = "\\"
POSIX_SEPARATOR = "/"
FIELD_SEPARATOR = ","
EMPTY_TEXT = ""
BRANCH_DATA_FIELDS = 4
BRANCH_LOCATION_FIELDS = 3
LINE_DATA_FIELDS = 2
FIRST_LINE_NUMBER = 1
ZERO_COUNT = 0
INVALID_LCOV_MESSAGE = "invalid LCOV at line {line}: {reason}"
MISSING_RECORD_MESSAGE = "expected LCOV records under {roots}"
NESTED_RECORD_REASON = "source record started before the previous record ended"
ORPHAN_RECORD_REASON = "record ended without a source"
UNCLOSED_RECORD_REASON = "source record is missing end_of_record"
EMPTY_SOURCE_REASON = "source path must not be empty"
MISSING_SUMMARY_REASON = "source record must contain FNF/FNH, BRF/BRH, and LF/LH"
DUPLICATE_SUMMARY_REASON = "source record contains duplicate {field}"
INVALID_INTEGER_REASON = "{field} must be a non-negative integer"
INVALID_BRANCH_DATA_REASON = "BRDA must contain line, block, branch, and taken"
INVALID_FUNCTION_DATA_REASON = "FNDA must contain execution count and function name"
INVALID_LINE_DATA_REASON = "DA must contain line and execution count"
INVALID_SUMMARY_REASON = "coverage summary has invalid counts"
LINES_ATTRIBUTE = "lines"
FUNCTIONS_ATTRIBUTE = "functions"
BRANCHES_ATTRIBUTE = "branches"
MISSED_LINES_ATTRIBUTE = "missed_lines"
MISSED_BRANCHES_ATTRIBUTE = "missed_branches"


class LcovFormatError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class _LcovRecord:
    source: str
    lines: CoverageMetric
    functions: CoverageMetric
    branches: CoverageMetric
    missed_lines: int
    missed_branches: int


def _invalid(line: int, reason: str) -> LcovFormatError:
    return LcovFormatError(INVALID_LCOV_MESSAGE.format(line=line, reason=reason))


def _non_negative_integer(value: str, field: str, line: int) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise _invalid(
            line,
            INVALID_INTEGER_REASON.format(field=field),
        ) from error
    if parsed < ZERO_COUNT:
        raise _invalid(line, INVALID_INTEGER_REASON.format(field=field))
    return parsed


def _branch_taken(value: str, line: int) -> bool:
    fields = value.split(FIELD_SEPARATOR)
    if len(fields) != BRANCH_DATA_FIELDS:
        raise _invalid(line, INVALID_BRANCH_DATA_REASON)
    for field in fields[:BRANCH_LOCATION_FIELDS]:
        _non_negative_integer(field, BRANCH_DATA_PREFIX, line)
    taken = fields[BRANCH_LOCATION_FIELDS]
    if taken == NOT_TAKEN:
        return False
    return _non_negative_integer(taken, BRANCH_DATA_PREFIX, line) > ZERO_COUNT


def _function_hit(value: str, line: int) -> bool:
    count, separator, name = value.partition(FIELD_SEPARATOR)
    if separator == EMPTY_TEXT or name == EMPTY_TEXT:
        raise _invalid(line, INVALID_FUNCTION_DATA_REASON)
    return _non_negative_integer(count, FUNCTION_DATA_PREFIX, line) > ZERO_COUNT


def _line_hit(value: str, line: int) -> bool:
    fields = value.split(FIELD_SEPARATOR)
    if len(fields) < LINE_DATA_FIELDS:
        raise _invalid(line, INVALID_LINE_DATA_REASON)
    _non_negative_integer(fields[ZERO_COUNT], LINE_DATA_PREFIX, line)
    return _non_negative_integer(
        fields[FIRST_LINE_NUMBER],
        LINE_DATA_PREFIX,
        line,
    ) > ZERO_COUNT


def _summary_metric(
    found: int | None,
    hit: int | None,
    line: int,
) -> CoverageMetric:
    if found is None or hit is None:
        raise _invalid(line, MISSING_SUMMARY_REASON)
    try:
        return CoverageMetric(count=found, covered=hit)
    except ValueError as error:
        raise _invalid(line, INVALID_SUMMARY_REASON) from error


def _finish_record(
    source: str | None,
    summaries: dict[str, int],
    branch_entries: list[bool],
    line_entries: list[bool],
    line: int,
) -> _LcovRecord:
    if source is None:
        raise _invalid(line, ORPHAN_RECORD_REASON)
    functions = _summary_metric(
        summaries.get(FUNCTIONS_FOUND_PREFIX),
        summaries.get(FUNCTIONS_HIT_PREFIX),
        line,
    )
    _summary_metric(
        summaries.get(BRANCHES_FOUND_PREFIX),
        summaries.get(BRANCHES_HIT_PREFIX),
        line,
    )
    _summary_metric(
        summaries.get(LINES_FOUND_PREFIX),
        summaries.get(LINES_HIT_PREFIX),
        line,
    )
    lines = CoverageMetric(
        count=len(line_entries),
        covered=sum(line_entries),
    )
    branches = CoverageMetric(
        count=len(branch_entries),
        covered=sum(branch_entries),
    )
    return _LcovRecord(
        source,
        lines,
        functions,
        branches,
        missed_lines=sum(not entry for entry in line_entries),
        missed_branches=sum(not entry for entry in branch_entries),
    )


def _under_source_root(source: str, root: ProjectPath) -> bool:
    normalized_source = source.replace(WINDOWS_SEPARATOR, POSIX_SEPARATOR)
    normalized_root = str(root.value)
    root_prefix = normalized_root + POSIX_SEPARATOR
    root_segment = POSIX_SEPARATOR + root_prefix
    return normalized_source.startswith(root_prefix) or root_segment in normalized_source


def _matches_target(record: _LcovRecord, target: BranchCoverageTarget) -> bool:
    return any(_under_source_root(record.source, root) for root in target.source_roots)


def _set_summary(
    summaries: dict[str, int],
    field: str,
    value: str,
    line: int,
) -> None:
    if field in summaries:
        raise _invalid(
            line,
            DUPLICATE_SUMMARY_REASON.format(field=field),
        )
    summaries[field] = _non_negative_integer(value, field, line)


def _aggregate(matching: tuple[_LcovRecord, ...], field: str) -> CoverageMetric:
    metrics = tuple(getattr(record, field) for record in matching)
    return CoverageMetric(
        count=sum(metric.count for metric in metrics),
        covered=sum(metric.covered for metric in metrics),
    )


def _aggregate_count(matching: tuple[_LcovRecord, ...], field: str) -> int:
    return sum(getattr(record, field) for record in matching)


def parse_lcov_branch_coverage(
    text: str,
    target: BranchCoverageTarget,
) -> BranchCoverage:
    records: list[_LcovRecord] = []
    source: str | None = None
    summaries: dict[str, int] = {}
    branch_entries: list[bool] = []
    line_entries: list[bool] = []
    last_line = ZERO_COUNT
    summary_prefixes = (
        FUNCTIONS_FOUND_PREFIX,
        FUNCTIONS_HIT_PREFIX,
        BRANCHES_FOUND_PREFIX,
        BRANCHES_HIT_PREFIX,
        LINES_FOUND_PREFIX,
        LINES_HIT_PREFIX,
    )

    for line_number, line in enumerate(text.splitlines(), start=FIRST_LINE_NUMBER):
        last_line = line_number
        if line.startswith(SOURCE_PREFIX):
            if source is not None:
                raise _invalid(line_number, NESTED_RECORD_REASON)
            source = line.removeprefix(SOURCE_PREFIX)
            if source == EMPTY_TEXT:
                raise _invalid(line_number, EMPTY_SOURCE_REASON)
        elif line == END_OF_RECORD:
            records.append(
                _finish_record(
                    source,
                    summaries,
                    branch_entries,
                    line_entries,
                    line_number,
                )
            )
            source = None
            summaries = {}
            branch_entries = []
            line_entries = []
        elif line.startswith(FUNCTION_DATA_PREFIX):
            _function_hit(line.removeprefix(FUNCTION_DATA_PREFIX), line_number)
        elif line.startswith(BRANCH_DATA_PREFIX):
            branch_entries.append(
                _branch_taken(line.removeprefix(BRANCH_DATA_PREFIX), line_number)
            )
        elif line.startswith(LINE_DATA_PREFIX):
            line_entries.append(
                _line_hit(line.removeprefix(LINE_DATA_PREFIX), line_number)
            )
        else:
            prefix = next(
                (item for item in summary_prefixes if line.startswith(item)),
                None,
            )
            if prefix is not None:
                _set_summary(
                    summaries,
                    prefix,
                    line.removeprefix(prefix),
                    line_number,
                )

    if source is not None:
        raise _invalid(last_line, UNCLOSED_RECORD_REASON)
    matching = tuple(record for record in records if _matches_target(record, target))
    if not matching:
        roots = tuple(str(root.value) for root in target.source_roots)
        raise LcovFormatError(MISSING_RECORD_MESSAGE.format(roots=roots))
    return BranchCoverage(
        lines=_aggregate(matching, LINES_ATTRIBUTE),
        functions=_aggregate(matching, FUNCTIONS_ATTRIBUTE),
        branches=_aggregate(matching, BRANCHES_ATTRIBUTE),
        missed_lines=_aggregate_count(matching, MISSED_LINES_ATTRIBUTE),
        missed_branches=_aggregate_count(matching, MISSED_BRANCHES_ATTRIBUTE),
    )
