# Punctum / Ramus 第一期执行计划

- 当前状态：`F3a`、`B2a` 与 `B2b` 已完成
- 下一节点：并行启动 `T2 Ghost Piece Projection` 与 `F3b Stateless UI Foundation`
- 架构依据：[第一期架构计划](./punctum-ramus-architecture-plan.md)
- 产品事实：[项目群总控](./punctum-ramus-program.md)
- 更新日期：2026-07-12

## 文档职责

本文只记录执行 DAG、原子任务、Prompt、写入范围、门禁和当前状态。

产品范围写入项目群总控。稳定边界和依赖规则写入架构计划。完成节点的详细聊天和调试过程不进入本文。

## 原子任务规则

Agent 是执行者，task 是 DAG 节点。

- 每个 task 只有一个目标、一个写入范围和一个完成条件。
- Prompt 不能要求 Agent 等待其他 Agent 后继续。
- 并行 task 不得修改同一文件。
- 汇合、manifest 接收、lockfile 接收和状态更新必须是新的 task。
- Agent 可以复用，但每个节点必须收到新的 Prompt。
- 测试先表达行为、不变量和失败场景。coverage 只用于发现遗漏。
- 平台和 IO 不设置 coverage 百分比，不为覆盖率制造无意义 mock。

## 当前 DAG

```text
S0 -> F1 -> B1 -> PT1 -> F2 -> R2
                         |
                         +-> F3a --------+
                         |               +-> B2b
                         +-> B2a --------+
                                          |
                                          +-> T2 Ghost projection --+
                                          |                         +-> T3 Tetris page --+
                                          +-> F3b UI foundation ----+                    +-> B3
                                                                    +-> F3c interaction -+
                                                                                         |
                                                                                         +-> F4 -> F5
```

`B2a`、`F3a` 和 `B2b` 已完成。现在可以直接并行启动 `T2` 与 `F3b`。

## 已完成节点

| 节点 | 结果 |
| --- | --- |
| `S0` | workspace、fixture record、baseline 和本地验证入口建立 |
| `F1/B1` | grid/input、Battle、Ramus 核心和跨 workspace 接收完成 |
| `PT1` | Tetris 纯核心、paint 和输入映射完成 |
| `F2` | Terminal Unicode、GPU planner/runtime 和 winit 输入完成 |
| `R2` | pure crate 与 Crossterm/wgpu 平台 crate 物理隔离 |
| `F3a` | `write_text` 接收 `&str`；`punctum-terminal` 不再依赖 `punctum-input` |
| `B2a` | Tetris GPU host、projection、整数 viewport 和 full/diff submission 完成 |
| `B2b` | 最终 manifest/lockfile 接收完成；Tetris coverage 区分 core、纯 view 和平台 host |

`B2b` 使用上游任务已经记录的本地验证证据完成接收。本轮按用户要求没有重新运行 Cargo test 或 fmt。

## `T2`：Ghost Piece Projection

### 目标

为 Tetris 增加纯下落预测。预测只影响显示，不进入业务状态。

### 行为

1. 从当前活动方块向下投影，直到下一步会越界或碰撞。
2. 活动方块已在落点时不重复显示。
3. game over 时不显示。
4. hard drop 的实际落点与预测一致。
5. Terminal 使用较弱字符或颜色；GPU 使用低透明度同色方块。
6. 不增加 hold、next preview、score、level、动画或音效。

### 写入范围

- `apps/tetris/src/**`
- `apps/tetris/tests/**`
- Ghost Piece 直接需要的 Terminal/GPU projection 测试

不得修改 Punctum、root manifest、lockfile、项目注册或管理文档。

### Agent Prompt

```text
你叫 GhostProjection。

阅读 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-program.md、C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-architecture-plan.md 和 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-execution-plan.md。

执行原子任务 T2：为 Tetris 增加 Ghost Piece 纯投影。

从当前活动方块计算最终下落位置。预测不能写入 TetrisState，不能改变 transition、碰撞、锁定、消行或方块序列。活动方块已经落地或 game over 时不显示。hard drop 的实际落点必须与预测一致。

让 Tetris 的逻辑投影能够区分 locked、ghost 和 active。Terminal 使用较弱字符或颜色，GPU 使用低透明度同色方块。算法不能依赖 Terminal、GPU、像素或 atlas 类型。

先写行为测试，覆盖空棋盘、堆叠、边界、旋转后位置、已落地、game over 和 hard drop 一致性。

只修改 apps/tetris/src/**、apps/tetris/tests/** 和 Ghost Piece 直接需要的 projection 测试。不要修改 Punctum、Cargo.toml、Cargo.lock、projects.json 或管理文档。完成后停止。
```

## `F3b`：Stateless UI Foundation

### 目标

建立 backend-neutral 的无状态布局和只读文本能力，不修改 Tetris 页面。

### 行为

1. 先建立并批准 `PEP 0002`。
2. 新增单一 `punctum-ui` pure crate。
3. 实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
4. measure 和 paint 使用同一份 backend text-layout 结果。
5. UI core 不依赖 Terminal、GPU、winit、wgpu 或产品类型。
6. 系统字体发现不进入 UI core，也不阻塞本任务。

### 写入范围

- `apps/punctum/peps/0002-*.md`
- `apps/punctum/crates/punctum-ui/**`
- Punctum workspace member 与 dependency 接入
- `apps/punctum/Cargo.lock`

不得修改 Tetris、Game、Ramus 或管理文档。

### Agent Prompt

```text
你叫 UiFoundation。

阅读 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-program.md、C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-architecture-plan.md 和 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-execution-plan.md。

执行原子任务 F3b：实现 Punctum 无状态 UI foundation。

先创建并批准 PEP 0002，明确整数 constraints、measure、layout、paint、clip、resize、文本布局结果和结构化错误。新增单一 pure crate punctum-ui，实现 Text、Row、Column、Border、Padding、Spacer、Align 和 SurfaceView。

Text 接收只读内容，不接收 TextEvent。measure 与 paint 必须使用同一份 backend text-layout 结果。punctum-ui 不得依赖 punctum-terminal、punctum-gpu、winit、wgpu、Tetris、Battle 或 Ramus。

按 TDD 实现，覆盖精确布局、零尺寸、空间不足、嵌套 clip 和 resize。只修改 PEP 0002、apps/punctum/crates/punctum-ui/**、Punctum workspace manifest 和 apps/punctum/Cargo.lock。不要修改 Tetris 或管理文档。完成后停止。
```

## `T3`：Tetris Visual Composition

`T3` 依赖 `T2` 与 `F3b`。它只组合页面，不增加框架能力或游戏规则。

页面必须包含标题、居中棋盘、消行数、状态、操作提示、game-over 和 Ghost Piece。Terminal/GPU 使用相同的信息层级和布局意图，但不要求像素一致。小窗口不能重叠。

### Agent Prompt

```text
你叫 TetrisVisual。

确认 T2 与 F3b 已通过，然后阅读三份 Punctum/Ramus 文档。执行原子任务 T3：使用 punctum-ui 重做 Tetris Terminal/GPU 页面。

组合 Text、Row、Column、Border、Padding、Spacer、Align 和 SurfaceView。页面显示标题、居中棋盘、累计消行数、运行或 game-over 状态、操作提示和 Ghost Piece。两端共享布局意图，各自完成 cell 或 glyph/sprite 投影。小窗口允许裁剪或收紧间距，但不能重叠或修改游戏状态。

不要修改 punctum-ui 实现，不增加 hold、next preview、score、level、动画、音效或新命令。只修改 apps/tetris/** 和必要的 Tetris lockfile。完成后停止。
```

## `F3c`：Interaction Foundation

`F3c` 只依赖 `F3b`，可以与 `T3` 并行。

### Agent Prompt

```text
你叫 InteractionFoundation。

确认 F3b 已通过，然后阅读三份 Punctum/Ramus 文档。执行原子任务 F3c：实现 provisional focus、event dispatch 和 Button。

实现类型安全 WidgetId、focus scope、Tab/Shift+Tab、event consumption 和 Button。widget 只返回 action，不能修改 Tetris 或 Battle 状态。没有 Game 真实需求时不要实现 ChoiceList、List、ScrollView 或其他集合 widget。

只修改 punctum-ui、PEP 0002 的交互补充和 Punctum lockfile。不要修改 Tetris、Game 或管理文档。完成后停止。
```

## `B3`：UI Composition Barrier

`B3` 只依赖 `T3` 与 `F3c`。它接收 manifest、lockfile、项目注册、baseline 和文档状态，不增加功能。

## 后续节点

- `F4`：Game UI 与 Battle/Ramus harness 并行，随后建立独立 Game composition task。
- `F5`：只读验证 Punctum、Tetris、Ramus 和 Game；固定 GPU reference 仍受 `GPU-REF-v0.1` 阻塞。

## 验证政策

- pure crate：TDD 后使用 line、function、region 100% 检查遗漏。
- pure view：使用 LLVM 单文件结果检查。
- Crossterm、winit、wgpu、窗口和 host：合同、fixture、smoke、E2E，不设 coverage 百分比。
- lane 只做聚焦验证。汇合节点检查完整 workspace、路径、lockfile 和反向依赖。

Tetris 当前注册策略：

- `core-coverage` 只覆盖 library。
- `terminal-view` 与 `gpu-view` 检查纯 projection 文件。
- Terminal/GPU host 只参加 test、Clippy 和本地 smoke。

## 当前门禁

| Gate | 状态 |
| --- | --- |
| `R2 Adapter Crate Boundary` | 已通过 |
| `F3a Terminal Text Boundary` | 已通过 |
| `B2a Tetris GPU` | 已通过 |
| `B2b Dual Backend Composition` | 已通过 |
| `T2 Ghost Piece Projection` | 可启动 |
| `F3b Stateless UI Foundation` | 可启动 |
| `T3 Tetris Visual Composition` | 等待 `T2/F3b` |
| `F3c Interaction Foundation` | 等待 `F3b` |
| `B3 UI Composition` | 等待 `T3/F3c` |
| `GPU-REF-v0.1` | Blocked；不阻塞本地工作 |

## Handoff

每个 task 完成后只报告：

- 修改路径。
- 实际行为结果。
- 执行的聚焦验证及退出码。
- 未通过门禁和残余风险。
- 下游需要接收的 manifest、lockfile 或公开 API 变化。
