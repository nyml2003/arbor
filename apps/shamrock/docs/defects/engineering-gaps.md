# 工程化缺陷

## 1. 超长文件问题仍然明显

现状：

- `battle-cli/src/main.rs` 已拆到 500 行以内，但 `demo_loop.rs` / `rendering.rs` 仍然偏大。[status.md](../current/status.md)

风险：

- 后续继续补规则或外壳时，复杂度会继续堆高

## 2. 注释规范没有真正沉淀

现状：

- 当前代码大量使用块注释 `/** ... */`，也夹杂局部 `/* ... */` 解释块。[lib.rs](../../crates/battle-core/src/lib.rs) [main.rs](../../crates/battle-cli/src/main.rs)
- 新 crate 里有些文件已经明显变成“无注释默认 + 个别解释”，风格开始漂。[lib.rs](../../crates/battle-view/src/lib.rs) [lib.rs](../../crates/battle-mechanics/src/lib.rs)

问题：

- 规范没有正式写进文档
- 什么该注释、什么不该注释、块注释和行注释边界，都还靠习惯

风险：

- 后面多人或多轮修改后，注释风格会继续碎裂

## 3. `battle-cli` 仍然混着 demo 运行、渲染适配和 AI 协调

现状：

- `battle-cli/src/main.rs` 已经收缩，但 `demo_loop.rs` 和 `rendering.rs` 仍然承接较多壳层职责。[main.rs](../../crates/battle-cli/src/main.rs)

问题：

- 这不是单纯文件长，而是职责仍然偏混

风险：

- 未来继续做 GUI / AI / 观战时，这一层会再次成为耦合点

## 4. `battle-core` 虽然已拆分，但流程协调仍然集中

现状：

- `battle-core/src/lib.rs` 已经降到较小体积，但 `turn.rs` / `move_resolution.rs` / `ops.rs` 仍然共同承接核心流程协调。[lib.rs](../../crates/battle-core/src/lib.rs)

问题：

- 文件规模问题已经缓解
- 但流程复杂度问题还没有消失

风险：

- 后续继续扩规则时，如果没有再抽更细的流程边界，复杂度会重新堆高

## 5. 当前流程规范还不够“自动约束”

现状：

- 现在已经有分层文档和缺陷文档，但多数约束仍靠人工遵守。[roadmap.md](../current/roadmap.md)

问题：

- 还没有更强的工程守卫，例如：
  - 文件规模门槛
  - 注释规范检查
  - 架构依赖约束检查

风险：

- 规则一多，代码会再次漂回“靠自觉维护”
