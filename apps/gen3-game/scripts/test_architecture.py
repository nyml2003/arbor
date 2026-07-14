from __future__ import annotations

import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


@dataclass(frozen=True)
class DependencyPolicy:
    dependency: str
    allowed_packages: frozenset[str]


@dataclass(frozen=True)
class ForbiddenDependency:
    package: str
    dependency: str


@dataclass(frozen=True)
class SourcePolicy:
    package: str
    forbidden_patterns: tuple[re.Pattern[str], ...]


@dataclass(frozen=True, order=True)
class Violation:
    location: str
    message: str

    def format(self) -> str:
        return f"{self.location}: {self.message}"


PLATFORM_DEPENDENCIES = (
    DependencyPolicy("glyphon", frozenset({"game-native-target"})),
    DependencyPolicy("pollster", frozenset({"game-native-target"})),
    DependencyPolicy("punctum-wgpu", frozenset({"game-native-target"})),
    DependencyPolicy("wgpu", frozenset({"game-native-target"})),
    DependencyPolicy(
        "winit", frozenset({"game-host", "game-native-target", "map-editor"})
    ),
)

FORBIDDEN_DEPENDENCIES = (
    ForbiddenDependency("game-session", "game-ui"),
    ForbiddenDependency("game-ui", "game-host"),
    ForbiddenDependency("game-native-target", "battle-domain"),
    ForbiddenDependency("game-native-target", "battle-application"),
    ForbiddenDependency("game-native-target", "battle-session"),
    ForbiddenDependency("game-native-target", "world-domain"),
    ForbiddenDependency("game-native-target", "world-application"),
)

PURE_PACKAGES = (
    "battle-application",
    "battle-domain",
    "battle-session",
    "game-session",
    "game-ui",
    "map-editor-core",
    "map-project",
    "world-application",
    "world-domain",
)

FORBIDDEN_SOURCE_PATTERNS = tuple(
    re.compile(pattern)
    for pattern in (
        r"\bstd::fs\b",
        r"\bstd::time::Instant\b",
        r"\bstd::time::SystemTime\b",
        r"\bwinit\b",
        r"\bwgpu\b",
        r"\bglyphon\b",
        r"\bpollster\b",
        r"\b(?:Mutex|RwLock|RefCell|OnceLock)\b",
        r"\bAtomic(?:Bool|Ptr|U8|U16|U32|U64|Usize|I8|I16|I32|I64|Isize)\b",
    )
)

ALLOWED_HOST_DEADLINE_FIELDS = frozenset({"next_wakeup"})
REMOVED_HOST_DEADLINE_FIELDS = frozenset(
    {"next_playback", "next_sprite_frame", "next_world_tick", "turn_hold_ends", "run_stop_ends"}
)


def load_metadata(workspace_root: Path) -> dict[str, Any]:
    result = subprocess.run(
        [
            "cargo",
            "metadata",
            "--manifest-path",
            str(workspace_root / "Cargo.toml"),
            "--format-version",
            "1",
            "--no-deps",
        ],
        cwd=workspace_root,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return json.loads(result.stdout)


def package_dependencies(package: dict[str, Any]) -> frozenset[str]:
    return frozenset(dependency["name"] for dependency in package["dependencies"])


def check_dependency_policies(metadata: dict[str, Any]) -> Iterable[Violation]:
    policies = {policy.dependency: policy for policy in PLATFORM_DEPENDENCIES}
    packages = {package["name"]: package for package in metadata["packages"]}

    for package_name, package in packages.items():
        for dependency in package_dependencies(package):
            policy = policies.get(dependency)
            if policy is not None and package_name not in policy.allowed_packages:
                yield Violation(
                    f"crates/{package_name}/Cargo.toml",
                    f"must not depend on platform crate {dependency}",
                )

    for rule in FORBIDDEN_DEPENDENCIES:
        package = packages.get(rule.package)
        if package is None:
            continue
        if rule.dependency in package_dependencies(package):
            yield Violation(
                f"crates/{rule.package}/Cargo.toml",
                f"must not depend on {rule.dependency}",
            )


def check_source_policies(workspace_root: Path) -> Iterable[Violation]:
    policies = (
        SourcePolicy(package, FORBIDDEN_SOURCE_PATTERNS) for package in PURE_PACKAGES
    )
    for policy in policies:
        source_root = workspace_root / "crates" / policy.package / "src"
        if not source_root.exists():
            continue
        for source in source_root.rglob("*.rs"):
            content = source.read_text(encoding="utf-8")
            for pattern in policy.forbidden_patterns:
                if pattern.search(content) is not None:
                    yield Violation(
                        source.relative_to(workspace_root).as_posix(),
                        f"contains forbidden pure-crate pattern {pattern.pattern}",
                    )


def check_host_deadlines(workspace_root: Path) -> Iterable[Violation]:
    host_main = workspace_root / "crates" / "game-host" / "src" / "main.rs"
    content = host_main.read_text(encoding="utf-8")
    deadline_pattern = re.compile(
        r"^\s*(?P<name>[a-z_]+):\s*Option<Instant>,", re.MULTILINE
    )
    for match in deadline_pattern.finditer(content):
        field = match.group("name")
        if field not in ALLOWED_HOST_DEADLINE_FIELDS:
            yield Violation(
                host_main.relative_to(workspace_root).as_posix(),
                f"added unapproved Instant deadline field {field}",
            )
    for field in REMOVED_HOST_DEADLINE_FIELDS:
        if re.search(rf"\b{re.escape(field)}\b", content) is not None:
            yield Violation(
                host_main.relative_to(workspace_root).as_posix(),
                f"contains removed effect deadline {field}",
            )


def main() -> int:
    workspace_root = Path(__file__).resolve().parent.parent
    try:
        metadata = load_metadata(workspace_root)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        print(f"failed to read Cargo metadata: {error}", file=sys.stderr)
        return 2

    violations = sorted(
        [
            *check_dependency_policies(metadata),
            *check_source_policies(workspace_root),
            *check_host_deadlines(workspace_root),
        ]
    )
    if violations:
        for violation in violations:
            print(violation.format(), file=sys.stderr)
        return 1

    print("Gen3 architecture gates passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
