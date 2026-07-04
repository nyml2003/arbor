"""tep-ops 入口——使得 `python tep-ops <cmd>` 可用。

用法:
    cd apps/tui-framework
    python tep-ops tep create "新提案"
    python tep-ops tep list
    python tep-ops tep show TEP-0001
    python tep-ops tep update TEP-0001 --status Review
"""

import sys
from pathlib import Path

# 把 tep-ops/ 自身加入 sys.path，使 cli/ app/ domain/ infra/ 可被 import
_ops_root = Path(__file__).resolve().parent
if str(_ops_root) not in sys.path:
    sys.path.insert(0, str(_ops_root))

from cli.main import main

if __name__ == "__main__":
    sys.exit(main())
