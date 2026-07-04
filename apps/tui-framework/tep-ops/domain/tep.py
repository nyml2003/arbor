"""TEP 领域模型——TUI Enhancement Proposal 的状态机和文件格式。

每个 TEP 是一个 Markdown 文件，YAML frontmatter 承载机器可读元数据。
状态转换由状态机保证合法性。
"""

import re
from dataclasses import dataclass, field
from datetime import date
from enum import Enum
from pathlib import Path


class TepStatus(Enum):
    """TEP 状态枚举。

    Draft -> Review -> Accepted -> Implemented -> Final
    任意节点可转换到 Rejected 或 Withdrawn。
    """

    DRAFT = "Draft"
    REVIEW = "Review"
    ACCEPTED = "Accepted"
    IMPLEMENTED = "Implemented"
    FINAL = "Final"
    REJECTED = "Rejected"
    WITHDRAWN = "Withdrawn"

    @classmethod
    def from_label(cls, label: str) -> "TepStatus":
        """大小写不敏感的标签解析。"""
        label_lower = label.strip().lower()
        for s in cls:
            if s.value.lower() == label_lower:
                return s
        raise ValueError(
            f"Unknown TEP status: {label}. Valid: {[s.value for s in cls]}"
        )

    @classmethod
    def valid_transitions_from(cls, status: "TepStatus") -> list["TepStatus"]:
        """返回从给定状态出发的所有合法目标状态。"""
        return list(_TRANSITIONS.get(status, set()))


# 合法状态转换表
_TRANSITIONS: dict[TepStatus, set[TepStatus]] = {
    TepStatus.DRAFT:       {TepStatus.REVIEW, TepStatus.WITHDRAWN, TepStatus.REJECTED},
    TepStatus.REVIEW:      {TepStatus.ACCEPTED, TepStatus.REJECTED, TepStatus.WITHDRAWN},
    TepStatus.ACCEPTED:    {TepStatus.IMPLEMENTED, TepStatus.WITHDRAWN},
    TepStatus.IMPLEMENTED: {TepStatus.FINAL, TepStatus.WITHDRAWN},
    TepStatus.FINAL:       set(),
    TepStatus.REJECTED:    set(),
    TepStatus.WITHDRAWN:   set(),
}

# TEP ID 校验正则
_TEP_ID_RE = re.compile(r"^TEP-(\d{4})")

# 合法 area 值
VALID_TEP_AREAS = {
    "architecture",
    "rendering",
    "input",
    "layout",
    "widgets",
    "styling",
    "ecosystem",
}


@dataclass
class TepProposal:
    """TEP 提案值对象——对应一个 TEP-NNNN.md 文件的元数据。

    只包含 YAML frontmatter 的机器可读字段。
    正文部分留在 Markdown 文件中按需读取。
    """

    id: str                     # e.g. "TEP-0001"
    title: str
    status: TepStatus
    created: date
    updated: date
    author: str
    area: str                   # "architecture" | "rendering" | "input" | ...
    affects: list[str] = field(default_factory=list)
    related: list[str] = field(default_factory=list)
    file_path: Path | None = None

    @property
    def number(self) -> int:
        """提取数字部分。TEP-0001 -> 1。"""
        return int(self.id.split("-")[1])

    @property
    def is_active(self) -> bool:
        """非终态即为活跃提案。"""
        return self.status not in (
            TepStatus.FINAL,
            TepStatus.REJECTED,
            TepStatus.WITHDRAWN,
        )

    def transition(self, new_status: TepStatus) -> "TepProposal":
        """返回一个状态变更后的新 TepProposal（不可变语义）。

        Raises ValueError if the transition is invalid.
        """
        if not TepProposal.validate_transition(self.status, new_status):
            valid = TepStatus.valid_transitions_from(self.status)
            raise ValueError(
                f"Invalid status transition: {self.status.value} -> {new_status.value}"
                f"\n  Valid from {self.status.value}: {[s.value for s in valid]}"
            )
        import dataclasses
        return dataclasses.replace(self, status=new_status, updated=date.today())

    @staticmethod
    def validate_transition(from_status: TepStatus, to_status: TepStatus) -> bool:
        """检查状态转换是否合法。"""
        return to_status in _TRANSITIONS.get(from_status, set())

    @staticmethod
    def validate_id(tep_id: str) -> bool:
        """验证 TEP ID 格式是否为 TEP-NNNN（4 位数字）。"""
        return bool(_TEP_ID_RE.match(tep_id))

    @staticmethod
    def next_id(existing_filenames: list[str]) -> str:
        """给定已有 TEP 文件名列表，返回下一个可用 ID（纯计算，无 I/O）。

        文件名应以 "TEP-NNNN" 开头，后缀任意（`.md` 或 `-中文标题.md`）。
        """
        max_num = 0
        for name in existing_filenames:
            m = _TEP_ID_RE.match(name)
            if m:
                num = int(m.group(1))
                if num != 9999:  # TEP-9999 是决策汇总，不参与编号
                    max_num = max(max_num, num)
        return f"TEP-{max_num + 1:04d}"
