"""TEP CLI——TUI Enhancement Proposal 管理命令行接口。"""

import argparse
import sys
from pathlib import Path

from infra.command import RealCommandRunner
from infra.events import ConsoleEventBus
from infra.filesystem import RealFileSystem
from domain.project import TuiFrameworkProject


def main() -> int:
    parser = argparse.ArgumentParser(
        prog="tep-ops",
        description="Arbor TUI Framework 工程编排系统",
    )
    sub = parser.add_subparsers(dest="command")

    # ── TEP ─────────────────────────────────────────────────
    tep = sub.add_parser("tep", help="管理 TUI Enhancement Proposals")
    tep_subs = tep.add_subparsers(dest="tep_action")

    tep_create = tep_subs.add_parser("create", help="创建新 TEP")
    tep_create.add_argument("title", help="TEP 标题")

    tep_list = tep_subs.add_parser("list", help="列出所有 TEP")
    tep_list.add_argument(
        "--status", "-s",
        choices=["Draft", "Review", "Accepted", "Implemented", "Final", "Rejected", "Withdrawn"],
        help="按状态过滤",
    )

    tep_show = tep_subs.add_parser("show", help="显示 TEP 内容")
    tep_show.add_argument("id", help="TEP ID（如 TEP-0001）")

    tep_update = tep_subs.add_parser("update", help="更新 TEP 状态")
    tep_update.add_argument("id", help="TEP ID（如 TEP-0001）")
    tep_update.add_argument(
        "--status", "-s", required=True,
        choices=["Draft", "Review", "Accepted", "Implemented", "Final", "Rejected", "Withdrawn"],
        help="新状态",
    )

    args = parser.parse_args()

    # 确定项目根目录（tep-ops 的父目录，即 tui-framework/）
    ops_root = Path(__file__).resolve().parent.parent
    project_root = ops_root.parent

    try:
        project = TuiFrameworkProject(root=project_root)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    runner = RealCommandRunner()
    fs = RealFileSystem()
    events = ConsoleEventBus()

    cmd = args.command
    if cmd == "tep":
        from app.manage_tep import ManageTep
        tep_action = args.tep_action
        if tep_action == "create":
            ok = ManageTep(runner, fs, events).run(
                project, action="create", title=args.title
            )
        elif tep_action == "list":
            ok = ManageTep(runner, fs, events).run(
                project, action="list", status=args.status
            )
        elif tep_action == "show":
            ok = ManageTep(runner, fs, events).run(
                project, action="show", tep_id=args.id
            )
        elif tep_action == "update":
            ok = ManageTep(runner, fs, events).run(
                project, action="update", tep_id=args.id, new_status=args.status
            )
        else:
            tep.print_help()
            return 1
    else:
        parser.print_help()
        return 1

    return 0 if ok else 1
