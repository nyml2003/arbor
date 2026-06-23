# Shamrock

Shamrock 是一个面向宝可梦对战的 Rust 模拟引擎项目。  
当前目标是先做 `Gen1 内容包`，同时把核心设计成可扩展、可回放、可测试的通用对战内核。

文档入口：

- [文档总览](./docs/README.md)
- [当前状态](./docs/current/status.md)
- [系统设计](./docs/architecture/system-design.md)
- [架构图](./docs/architecture/overview.md)
- [迭代计划](./docs/current/roadmap.md)

当前设计重点：

- 纯函数式 `step` 内核
- 数据包、机制包、格式包分层
- 技能、天气、状态等通过统一 hook 和 `BattleOp` 接入
- 从第一版开始保证确定性和可回放

运行 demo：

- `cargo run -p battle-cli -- --plain`
- `cargo run -p battle-cli -- --tui`
- 或设置 `SHAMROCK_UI=plain|tui`

plain CLI 命令：

- 数字：选择对应操作
- `history`：查看完整对战历史
- `help`：查看可用命令
