"""TEP 管理用例——创建、查看、列表、状态转换 TUI Enhancement Proposals。

所有 TEP 文件操作通过注入的 FileSystem 进行，不使用 pathlib 直接操作。
"""

import re
from datetime import date
from pathlib import Path

from domain.project import TuiFrameworkProject
from domain.tep import TepProposal, TepStatus, VALID_TEP_AREAS
from infra.command import CommandRunner
from infra.filesystem import FileSystem
from infra.events import EventBus


# YAML frontmatter 模式：文件以 --- 开头，第二个 --- 结束 frontmatter
_FRONTMATTER_RE = re.compile(r"^---\s*\n(.*?)\n---\s*\n", re.DOTALL)

# TEP 文件名模式
_TEP_FILE_RE = re.compile(r"^TEP-(\d{4})")


class ManageTep:
    """TEP 管理用例——create / list / show / update 四个子操作。"""

    TEMPLATE_NAME = "TEP-TEMPLATE.md"

    def __init__(self, runner: CommandRunner, fs: FileSystem, events: EventBus):
        self.runner = runner
        self.fs = fs
        self.events = events

    # ── 入口 ─────────────────────────────────────────────────

    def run(self, project: TuiFrameworkProject, action: str, **kwargs) -> bool:
        """路由到具体子操作。

        Args:
            action: "create" | "list" | "show" | "update"
        """
        if action == "create":
            return self._create(project, kwargs.get("title", ""))
        elif action == "list":
            return self._list(project, kwargs.get("status"))
        elif action == "show":
            return self._show(project, kwargs.get("tep_id", ""))
        elif action == "update":
            return self._update(
                project,
                kwargs.get("tep_id", ""),
                kwargs.get("new_status", ""),
            )
        else:
            self.events.emit("error", f"Unknown TEP action: {action}")
            return False

    # ── Create ───────────────────────────────────────────────

    def _create(self, project: TuiFrameworkProject, title: str) -> bool:
        """从模板创建新 TEP。"""
        if not title.strip():
            self.events.emit("error", "TEP title cannot be empty")
            return False

        teps_dir = project.teps_dir
        self.events.emit("step", f"Create TEP: {title}")

        # 确保目录存在
        if not self.fs.exists(teps_dir):
            self.fs.mkdir_p(teps_dir)

        # 计算下一个 ID
        existing = self._list_tep_filenames(teps_dir)
        next_id = TepProposal.next_id(existing)
        # 文件名格式：TEP-NNNN-{安全标题}.md
        safe_title = re.sub(r"[^\w一-鿿-]", "", title.replace(" ", "-"))
        filename = f"{next_id}-{safe_title}.md" if safe_title else f"{next_id}.md"
        filepath = teps_dir / filename

        if self.fs.exists(filepath):
            self.events.emit("error", f"TEP file already exists: {filename}")
            return False

        # 读取模板
        template_path = teps_dir / self.TEMPLATE_NAME
        if not self.fs.exists(template_path):
            self.events.emit("error", f"Template not found: {template_path}")
            return False
        template = self.fs.read_text(template_path)

        # 替换占位符
        today = date.today().isoformat()
        content = template.replace("{{TEP_ID}}", next_id)
        content = content.replace("{{TITLE}}", title)
        content = content.replace("{{DATE}}", today)

        self.fs.write_text(filepath, content)
        self.events.emit("info", f"Created: {filepath}")
        self.events.emit("success", f"TEP {next_id} created: {title}")
        return True

    # ── List ─────────────────────────────────────────────────

    def _list(self, project: TuiFrameworkProject, status: str | None = None) -> bool:
        """列出 TEP 目录中所有提案，可选按状态过滤。"""
        teps_dir = project.teps_dir
        self.events.emit("step", "TEP List")

        if not self.fs.exists(teps_dir):
            self.events.emit("info", f"No TEPs directory: {teps_dir}")
            return True

        filter_status = None
        if status:
            try:
                filter_status = TepStatus.from_label(status)
            except ValueError as e:
                self.events.emit("error", str(e))
                return False

        files = self._list_tep_filenames(teps_dir)
        # 过滤：只要 TEP-NNNN.md，排除模板
        tep_files = [f for f in files if _TEP_FILE_RE.match(f)]

        if not tep_files:
            self.events.emit("info", "No TEPs found")
            return True

        found = 0
        for fname in sorted(tep_files):
            filepath = teps_dir / fname
            try:
                tep = self._parse_frontmatter(self.fs.read_text(filepath))
            except Exception:
                continue  # 跳过格式损坏的文件
            tep.file_path = filepath

            if filter_status and tep.status != filter_status:
                continue

            found += 1
            label = f"{tep.id:<10} [{tep.status.value:<12}] {tep.title}"
            print(label)

        print()
        self.events.emit("info", f"Total: {found} TEP(s)")
        self.events.emit("success", "List complete")
        return True

    # ── Show ─────────────────────────────────────────────────

    def _show(self, project: TuiFrameworkProject, tep_id: str) -> bool:
        """显示单个 TEP 的完整内容。"""
        if not tep_id:
            self.events.emit("error", "TEP ID required (e.g. TEP-0001)")
            return False

        if not TepProposal.validate_id(tep_id):
            self.events.emit(
                "error",
                f"Invalid TEP ID format: {tep_id}. Expected: TEP-NNNN",
            )
            return False

        teps_dir = project.teps_dir
        filepath = self._find_tep_file(teps_dir, tep_id)

        if filepath is None:
            self.events.emit("error", f"TEP not found: {tep_id}")
            return False

        content = self.fs.read_text(filepath)
        self.events.emit("info", f"=== {filepath} ===")
        print(content)
        self.events.emit("success", f"Displayed {tep_id}")
        return True

    # ── Update ───────────────────────────────────────────────

    def _update(
        self, project: TuiFrameworkProject, tep_id: str, new_status_str: str
    ) -> bool:
        """更新 TEP 状态，带合法性校验。"""
        if not tep_id:
            self.events.emit("error", "TEP ID required (e.g. TEP-0001)")
            return False
        if not TepProposal.validate_id(tep_id):
            self.events.emit("error", f"Invalid TEP ID format: {tep_id}")
            return False
        if not new_status_str:
            self.events.emit("error", "New status required (--status)")
            return False

        try:
            new_status = TepStatus.from_label(new_status_str)
        except ValueError as e:
            self.events.emit("error", str(e))
            return False

        teps_dir = project.teps_dir
        filepath = self._find_tep_file(teps_dir, tep_id)

        if filepath is None:
            self.events.emit("error", f"TEP not found: {tep_id}")
            return False

        self.events.emit("step", f"Update {tep_id}: status -> {new_status.value}")

        content = self.fs.read_text(filepath)
        current = self._parse_frontmatter(content)

        # 校验转换
        if not TepProposal.validate_transition(current.status, new_status):
            valid = TepStatus.valid_transitions_from(current.status)
            self.events.emit(
                "error",
                f"Invalid transition: {current.status.value} -> {new_status.value}"
                f"\n  Valid from {current.status.value}: "
                f"{[s.value for s in valid]}",
            )
            return False

        # 更新 frontmatter
        today = date.today().isoformat()
        updated_content = self._update_frontmatter(content, new_status, today)
        self.fs.write_text(filepath, updated_content)

        self.events.emit(
            "info",
            f"Updated {tep_id}: {current.status.value} -> {new_status.value}",
        )
        self.events.emit("success", "Status updated")
        return True

    # ── Helpers ──────────────────────────────────────────────

    def _find_tep_file(self, teps_dir: Path, tep_id: str) -> Path | None:
        """在 teps_dir 中查找以 tep_id 开头的文件。"""
        if not self.fs.exists(teps_dir):
            return None
        entries = self.fs.listdir(teps_dir)
        prefix = f"{tep_id}-"
        alt_prefix = f"{tep_id}."
        for entry in entries:
            if entry.name.startswith(prefix) or entry.name.startswith(alt_prefix):
                return entry
        return None

    def _list_tep_filenames(self, teps_dir: Path) -> list[str]:
        """列出 teps_dir 中所有文件名（不含路径前缀）。"""
        if not self.fs.exists(teps_dir):
            return []
        entries = self.fs.listdir(teps_dir)
        return [e.name for e in entries]

    def _parse_frontmatter(self, content: str) -> TepProposal:
        """从 YAML frontmatter 解析 TepProposal。

        简化版 YAML 解析——只支持我们定义的那几个 key，
        避免引入 PyYAML 依赖。
        """
        m = _FRONTMATTER_RE.match(content)
        if not m:
            raise ValueError(
                "TEP file missing YAML frontmatter (must start with --- ... ---)"
            )

        raw = m.group(1)
        data = self._parse_simple_yaml(raw)

        return TepProposal(
            id=data.get("id", "TEP-0000"),
            title=data.get("title", "Untitled"),
            status=TepStatus.from_label(data.get("status", "Draft")),
            created=_parse_date(data.get("created", "2000-01-01")),
            updated=_parse_date(data.get("updated", "2000-01-01")),
            author=data.get("author", "unknown"),
            area=data.get("area", "architecture"),
            affects=_parse_list(data.get("affects", "")),
            related=_parse_list(data.get("related", "")),
        )

    @staticmethod
    def _parse_simple_yaml(raw: str) -> dict[str, str]:
        """解析简化版 YAML（只支持 key: value 行格式）。"""
        result: dict[str, str] = {}
        for line in raw.split("\n"):
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            if ":" in stripped:
                key, _, val = stripped.partition(":")
                key = key.strip()
                val = val.strip()
                # 去除引号
                if val.startswith('"') and val.endswith('"'):
                    val = val[1:-1]
                elif val.startswith("'") and val.endswith("'"):
                    val = val[1:-1]
                result[key] = val
        return result

    def _update_frontmatter(
        self, content: str, new_status: TepStatus, today: str
    ) -> str:
        """替换 frontmatter 中的 status 和 updated 字段。

        只在第一个 frontmatter 块内替换，不影响正文中出现的相同 key。
        """
        m = _FRONTMATTER_RE.match(content)
        if not m:
            raise ValueError("TEP file missing YAML frontmatter")

        old_fm = m.group(1)
        new_fm_lines = []
        for line in old_fm.split("\n"):
            stripped = line.strip()
            if stripped.startswith("status:"):
                new_fm_lines.append(f"status: {new_status.value}")
            elif stripped.startswith("updated:"):
                new_fm_lines.append(f"updated: {today}")
            else:
                new_fm_lines.append(line)

        new_fm = "\n".join(new_fm_lines)
        # 只替换第一个 occurence（frontmatter 块）
        return content.replace(old_fm, new_fm, 1)


# ── 模块级 helper ────────────────────────────────────────────


def _parse_date(raw: str) -> date:
    """解析 ISO 格式日期字符串，错误时返回 epoch。"""
    try:
        return date.fromisoformat(raw.strip())
    except (ValueError, AttributeError):
        return date(2000, 1, 1)


def _parse_list(raw: str) -> list[str]:
    """解析 YAML 列表字符串：'[AST, Parser]' -> ['AST', 'Parser']。

    也支持空列表 '[]' 或空字符串。
    """
    if not raw or raw.strip() in ("[]", ""):
        return []
    inner = raw.strip().strip("[]")
    items = inner.split(",")
    return [item.strip().strip('"').strip("'") for item in items if item.strip()]
