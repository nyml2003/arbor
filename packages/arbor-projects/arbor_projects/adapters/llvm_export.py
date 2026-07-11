from __future__ import annotations

import json
from typing import Any

from arbor_projects.domain import CoverageMetric, CoverageTarget, FileCoverage

from .registry_format import RegistryFormatError


POSIX_SEPARATOR = "/"
WINDOWS_SEPARATOR = "\\"
PATH_SUFFIX_SEPARATOR = "/"
DATA_KEY = "data"
FILES_KEY = "files"
FILENAME_KEY = "filename"
SUMMARY_KEY = "summary"
COUNT_KEY = "count"
COVERED_KEY = "covered"
REGIONS_KEY = "regions"
FUNCTIONS_KEY = "functions"
LINES_KEY = "lines"
EXPECTED_ONE_SOURCE_MESSAGE = "expected one coverage record for {source}, found {count}"
INVALID_EXPORT_MESSAGE = "invalid LLVM coverage export: {reason}"
EXPECTED_EXPORT_OBJECT_MESSAGE = "LLVM coverage {field!r} must be an object"
EXPECTED_EXPORT_LIST_MESSAGE = "LLVM coverage {field!r} must be a list"
EXPECTED_EXPORT_STRING_MESSAGE = "LLVM coverage {field!r} must be a string"
EXPECTED_EXPORT_COUNT_MESSAGE = "LLVM coverage {field!r} must be an integer"
EXPECTED_SINGLE_RECORD = 1
FIRST_ITEM_INDEX = 0


def _require_object(value: object, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise RegistryFormatError(
            EXPECTED_EXPORT_OBJECT_MESSAGE.format(field=field)
        )
    return value


def _require_list(value: object, field: str) -> list[object]:
    if not isinstance(value, list):
        raise RegistryFormatError(EXPECTED_EXPORT_LIST_MESSAGE.format(field=field))
    return value


def _require_string(value: object, field: str) -> str:
    if not isinstance(value, str):
        raise RegistryFormatError(
            EXPECTED_EXPORT_STRING_MESSAGE.format(field=field)
        )
    return value


def _require_count(value: object, field: str) -> int:
    if type(value) is not int:
        raise RegistryFormatError(EXPECTED_EXPORT_COUNT_MESSAGE.format(field=field))
    return value


def _normalize_path(value: str) -> str:
    return value.replace(WINDOWS_SEPARATOR, POSIX_SEPARATOR)


def _matches_source(filename: str, source: str) -> bool:
    normalized_filename = _normalize_path(filename)
    normalized_source = _normalize_path(source)
    return (
        normalized_filename == normalized_source
        or normalized_filename.endswith(PATH_SUFFIX_SEPARATOR + normalized_source)
    )


def _parse_metric(summary: dict[str, Any], key: str) -> CoverageMetric:
    metric = _require_object(summary.get(key), key)
    return CoverageMetric(
        count=_require_count(metric.get(COUNT_KEY), COUNT_KEY),
        covered=_require_count(metric.get(COVERED_KEY), COVERED_KEY),
    )


def parse_llvm_export(text: str, target: CoverageTarget) -> FileCoverage:
    try:
        payload = json.loads(text)
        root = _require_object(payload, DATA_KEY)
        records = _require_list(root.get(DATA_KEY), DATA_KEY)
        matching_files: list[dict[str, Any]] = []
        for record in records:
            record_object = _require_object(record, DATA_KEY)
            for file_value in _require_list(record_object.get(FILES_KEY), FILES_KEY):
                file_object = _require_object(file_value, FILES_KEY)
                filename = _require_string(file_object.get(FILENAME_KEY), FILENAME_KEY)
                if _matches_source(filename, str(target.source.value)):
                    matching_files.append(file_object)
    except json.JSONDecodeError as error:
        raise RegistryFormatError(
            INVALID_EXPORT_MESSAGE.format(reason=error)
        ) from error

    if len(matching_files) != EXPECTED_SINGLE_RECORD:
        raise RegistryFormatError(
            EXPECTED_ONE_SOURCE_MESSAGE.format(
                source=target.source.value,
                count=len(matching_files),
            )
        )
    summary = _require_object(
        matching_files[FIRST_ITEM_INDEX].get(SUMMARY_KEY),
        SUMMARY_KEY,
    )
    return FileCoverage(
        regions=_parse_metric(summary, REGIONS_KEY),
        functions=_parse_metric(summary, FUNCTIONS_KEY),
        lines=_parse_metric(summary, LINES_KEY),
    )
