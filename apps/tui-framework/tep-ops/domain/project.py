"""聚合根：TuiFrameworkProject——TUI 框架项目的工程模型。

所有路径映射只在此处维护。
改目录结构只改 config.json，所有用例自动生效。
"""

import json
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class TuiFrameworkProject:
    """聚合根：TUI 框架项目的完整工程模型。"""

    root: Path

    # 项目路径 —— 由 config.json 填充
    teps_dir: Path = field(init=False)

    def __post_init__(self):
        config = self._load_config()
        paths = config["paths"]
        self.teps_dir = self.root / paths["teps_dir"]

    def _load_config(self) -> dict:
        config_path = Path(__file__).resolve().parent.parent / "config.json"
        if not config_path.exists():
            raise FileNotFoundError(f"配置文件不存在: {config_path}")
        return json.loads(config_path.read_text(encoding="utf-8"))
