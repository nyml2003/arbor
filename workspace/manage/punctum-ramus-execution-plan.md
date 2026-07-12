# Punctum / Ramus 第一期执行计划

- 当前状态：`F3a`、`B2a`、`B2b`、`T2` 与 `F3b` 已完成
- 下一节点：`F3d Game Screen Model`
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
                                          +-> T2 Ghost projection --------------------+
                                          |                                           |
                                          +-> F3b UI foundation                       |
                                                   |                                  |
                                                   +-> F3d screen model                |
                                                          -> F3e GPU GUI probe         |
                                                          -> B3a boundary decision ----+
                                                                                      +-> T3 Tetris page --+
                                                                                      +-> F3c interaction -+-> B3 -> F4 -> F5
```

`B2a`、`F3a`、`B2b`、`T2` 和 `F3b` 已完成。`T3` 与 `F3c` 暂停。先用真实 Game GUI 取得边界证据，再由 `B3a` 重写两个任务的最终合同。

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
| `T2` | Ghost Piece 纯投影完成；预测与 hard drop 使用同一落点算法，Terminal/GPU 分别使用弱字符和低透明度同色方块 |
| `F3b` | `punctum-ui` 无状态整数布局和只读文本基础完成；measure/paint 复用 backend text-layout 结果 |

`B2b` 使用上游任务已经记录的本地验证证据完成接收。本轮按用户要求没有重新运行 Cargo test 或 fmt。

## `T2`：Ghost Piece Projection

### 完成结果

- `ghost_piece` 从当前活动方块和锁定棋盘纯计算落点，不写入 `TetrisState`。
- hard drop 与 Ghost Piece 复用同一落点算法。transition、碰撞、锁定、消行和方块序列行为不变。
- `TetrisCell` 区分 `Locked`、`Ghost` 和 `Active`。绘制顺序为 locked、ghost、active。
- Terminal 使用暗背景上的同色 `░`。GPU 使用 alpha 64 的同色方块。
- 行为测试覆盖空棋盘、堆叠、边界、旋转后位置、已落地、game over、纯读取和 hard drop 一致性。

聚焦验证：

- `cargo test --manifest-path apps/tetris/Cargo.toml --all-targets --locked`：47 项测试通过，退出码 0。
- `cargo clippy --manifest-path apps/tetris/Cargo.toml --all-targets --locked -- -D warnings`：退出码 0。
- `cargo fmt --manifest-path apps/tetris/Cargo.toml --package punctum-tetris -- --check`：退出码 0。
- `cargo llvm-cov --manifest-path apps/tetris/Cargo.toml --tests --locked --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100`：core line、function 和 region 均为 100%，退出码 0。
- Terminal pure view 单文件 coverage 为 100%。GPU Ghost projection 新增路径已覆盖；该文件仍有既有非 Ghost 测试辅助分支未达到 100%。
- 项目注册的 `core-coverage --lib` 不执行 `tests/tetris_contract.rs`。本任务按写入范围未修改 `projects.json`；`B3` 需要接收并修正该验证命令。

公开 API 变化：新增 `ghost_piece`；`TetrisCell::Tetromino` 拆为 `Locked`、`Ghost` 和 `Active`。没有 manifest 或 lockfile 变化。

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

### 完成结果

- 新增单一 `punctum-ui` pure crate，并接入 Punctum workspace manifest 和 lockfile。
- 实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
- `Ui::measure` 保存 backend `TextLayout`。`Measured::layout` 只负责放置。`Frame::paint` 复用同一布局结果，不重新测量文本。
- 每层容器与外部 clip 求交。resize 使用新 constraints 重新执行无状态 measure/layout pipeline。
- constraints、text-layout 越界和 backend paint 失败使用结构化错误。空间不足采用确定性收缩与裁剪。
- `punctum-ui` 只依赖 `punctum-grid`，不依赖 Terminal、GPU、winit、wgpu 或产品类型。
- 用户在执行中取消 PEP 0002。合同保存在 crate rustdoc、public types 和合同测试中。

聚焦验证：

- `cargo test -p punctum-ui`：13 项合同测试通过，退出码 0。
- `cargo clippy -p punctum-ui --all-targets --all-features -- -D warnings`：退出码 0。
- `cargo fmt --all -- --check`：退出码 0。
- `cargo llvm-cov -p punctum-ui --all-features --summary-only`：line、function 和 region 均为 100%，退出码 0。
- `cargo tree -p punctum-ui`：依赖树只有 `punctum-grid`。

### 目标

建立 backend-neutral 的无状态布局和只读文本能力，不修改 Tetris 页面。

### 行为

1. 用户取消 PEP 0002；合同写入 crate rustdoc、public types 和合同测试。
2. 新增单一 `punctum-ui` pure crate。
3. 实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
4. measure 和 paint 使用同一份 backend text-layout 结果。
5. UI core 不依赖 Terminal、GPU、winit、wgpu 或产品类型。
6. 系统字体发现不进入 UI core，也不阻塞本任务。

### 写入范围

- `apps/punctum/crates/punctum-ui/**`
- Punctum workspace member 与 dependency 接入
- `apps/punctum/Cargo.lock`

不得修改 Tetris、Game、Ramus 或管理文档。

### Agent Prompt

以下是初始 Prompt。执行中用户明确取消 PEP 0002，最终结果以上述完成结果为准。

```text
你叫 UiFoundation。

阅读 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-program.md、C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-architecture-plan.md 和 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-execution-plan.md。

执行原子任务 F3b：实现 Punctum 无状态 UI foundation。

先创建并批准 PEP 0002，明确整数 constraints、measure、layout、paint、clip、resize、文本布局结果和结构化错误。新增单一 pure crate punctum-ui，实现 Text、Row、Column、Border、Padding、Spacer、Align 和 SurfaceView。

Text 接收只读内容，不接收 TextEvent。measure 与 paint 必须使用同一份 backend text-layout 结果。punctum-ui 不得依赖 punctum-terminal、punctum-gpu、winit、wgpu、Tetris、Battle 或 Ramus。

按 TDD 实现，覆盖精确布局、零尺寸、空间不足、嵌套 clip 和 resize。只修改 PEP 0002、apps/punctum/crates/punctum-ui/**、Punctum workspace manifest 和 apps/punctum/Cargo.lock。不要修改 Tetris 或管理文档。完成后停止。
```

## `F3d`：Game Screen Model

### 目标

在 `game-ui` 中建立第一期 Battle screen 的纯展示模型。该模型只表达玩家需要看到的信息和可以触发的 action，不包含布局单位或 backend 类型。

### 行为

1. 从 `BattleObservation`、legal actions 和公开 event log 生成 `BattleScreenModel`。
2. 模型包含双方当前宝可梦、等级、HP、已公开状态、玩家合法操作、战斗日志、当前阶段和胜负结果。
3. 图片只使用产品资源键。模型不携带 atlas rect、纹理句柄或像素尺寸。
4. 操作项使用稳定的产品 action ID。模型不包含焦点位置、Terminal 行列或 GPU bounds。
5. 对手隐藏信息继续服从 `battle-application` observation 合同。UI model 不重新读取或推断 domain state。

### 写入范围

- `apps/gen3-game/crates/game-ui/**`
- `apps/gen3-game/Cargo.toml`
- `apps/gen3-game/Cargo.lock`

不得修改 Punctum、Tetris、Battle domain/application 实现、Ramus 或管理文档。

### 验证

- screen model 的 pure contract tests。
- observation 隐藏信息和 action 映射测试。
- 中文、ASCII、空日志、game over、无合法操作和边界 HP fixture。
- Game workspace test、Clippy、fmt。
- pure model 的 line、function 和 region coverage 为 100%。

### Agent Prompt

```text
你叫 GameScreenModel。

阅读三份 Punctum/Ramus 管理文档和现有 battle-application observation 合同。执行 F3d：在 game-ui 中建立第一期 BattleScreenModel。

模型只表达可见内容、产品资源键和产品 action。不能包含 GridSize、TerminalCell、GpuCell、像素、DPI、atlas、glyph、纹理句柄或平台事件。隐藏信息必须继续由 BattleObservation 决定，不能从 domain state 自行补全。

先写 pure contract tests，再实现最小模型和投影。只修改 game-ui、Game workspace manifest 和 lockfile。完成后停止。
```

## `F3e`：Native GPU GUI Probe

### 目标

在 Game 内实现一个最小但真实的 Native GPU Battle screen，验证 GPU GUI 的本质需求。该任务提供边界证据，不创建或修改 Punctum 公共 GUI 抽象。

### 场景

Probe 必须同时显示：

- 中文和 ASCII 文本。
- 双方当前宝可梦信息。
- 非 Cell 等比的头像图片。
- HP fill bar 和边框矩形。
- 合法操作选择列表及当前选择状态。
- 多行战斗日志。

窗口至少覆盖一个正常尺寸、一个窄尺寸和一个整数放大尺寸。内容不能重叠。允许按明确优先级裁剪或隐藏次要内容。

### 技术边界

1. GUI 几何使用独立的整数虚拟像素类型，例如 `UiPx`、`UiPoint`、`UiSize` 和 `UiRect`。不能使用 `GridSize` 冒充 GPU GUI 几何。
2. framebuffer 物理像素、DPI、整数 scale 和 letterbox 只存在于 host adapter。
3. paint list 至少区分 glyph run、image quad、fill rect 和 stroke rect。图片的 source rect 与 destination rect 分开。
4. 文本使用成熟 shaping/rasterization 实现。不得自行实现 Unicode shaping，也不得假设 char、grapheme 和 glyph 一一对应。
5. Probe 可以消费 `punctum-input` 和 grid 基础，但不能把文字、图片或矩形塞进 `GpuCell`。
6. Game 状态和 `BattleScreenModel` 不读取窗口、DPI、字体或 GPU resource。
7. 本任务形成的类型默认归 Game 所有。没有 `B3a` 决策不得迁入 Punctum。

### 写入范围

- `apps/gen3-game/crates/game-ui/**`
- `apps/gen3-game/crates/game-host/**`
- Probe 直接使用的 Game fixture 和 bitmap asset
- `apps/gen3-game/Cargo.toml`
- `apps/gen3-game/Cargo.lock`

不得修改 `apps/punctum/**`、Tetris、Battle domain/application、Ramus 或管理文档。

### 验证

- virtual-pixel layout 和 paint-list logical oracle。
- 三种 viewport 下无重叠、clip 有界、图片保持宽高比和选择项可见。
- 相同 `BattleScreenModel` 在 resize 前后不变。
- glyph、image、fill 和 stroke 使用不同 primitive 的合同测试。
- Game workspace test、Clippy、fmt。
- 本地 wgpu smoke。固定 GPU readback 继续受 `GPU-REF-v0.1` 阻塞。

### Agent Prompt

```text
你叫 NativeGuiProbe。

确认 F3d 已完成。阅读三份 Punctum/Ramus 管理文档、BattleScreenModel 和现有 Punctum GPU 边界。执行 F3e：在 Game 内建立 Native GPU GUI probe。

使用整数虚拟像素布局。分别表达 glyph run、image quad、fill rect 和 stroke rect。图片 source/destination 分离。窗口 DPI、整数缩放和 letterbox 留在 game-host。使用成熟文本 shaping/rasterization 实现，不手写 Unicode shaping。

不得修改 Punctum，也不得把普通 GUI 内容塞进 GpuCell。所有新 GUI 类型先归 Game 所有。完成 pure logical oracle、三种 viewport 测试和本地 wgpu smoke 后停止。
```

## `B3a`：UI Boundary Decision

### 目标

根据现有 Terminal、Tetris、`F3b` 和 `F3e` 证据，关闭“离散 Grid UI 与 Native GPU GUI”开放问题。该节点只做接收和决策，不实现或重构代码。

### 必须输出

1. 本质复杂度清单：Terminal 列占用、Unicode shaping、GPU virtual pixel、图片适配、DPI、scene/grid 投影。
2. 偶然复杂度清单：万能 Cell、混用单位、backend capability 分支、伪共享 geometry、为消除布局重复建立的 wrapper。
3. 稳定共享合同：明确 grid、input、screen model、action 和 interaction 中哪些内容可以共享。
4. backend 私有合同：明确 Terminal layout/frame 与 GPU layout/paint list 的所有权。
5. `punctum-ui` 处置：保留为离散 UI、拆分语义层，或停止提升。必须选定一个方向。
6. 新的 DAG 和可直接执行的 `T3/F3c` Prompt。

### 决策标准

- 只有输入、状态转换和 black-box oracle 相同的行为才能进入共享层。
- 两端都使用整数不等于单位语义相同。
- 布局代码重复但约束和 oracle 不同，属于本质差异，不强制抽取。
- 决策不能要求上层处理 Terminal width、continuation、DPI、glyph、atlas 或图片 fit。
- 如果 probe 没有覆盖文字、图片、矩形、选择和 resize，本节点不能通过。

### 写入范围

- `workspace/manage/punctum-ramus-program.md`
- `workspace/manage/punctum-ramus-architecture-plan.md`
- `workspace/manage/punctum-ramus-execution-plan.md`
- 决策直接需要的 Punctum/Game README 状态说明

不得修改 Rust 实现、manifest 或 lockfile。

### Agent Prompt

```text
你叫 UiBoundaryDecision。

确认 F3d 和 F3e 已完成。阅读三份 Punctum/Ramus 管理文档、punctum-ui/terminal/gpu 合同、Tetris 双后端 projection、Game BattleScreenModel、Native GPU GUI probe 及其验证证据。执行 B3a：关闭“离散 Grid UI 与 Native GPU GUI”开放问题。

先分别列出本质复杂度和偶然复杂度。再逐项判断 grid、input、screen model、action、interaction、layout geometry 和 paint primitive 的所有权。只有输入、状态转换和 black-box oracle 相同的行为可以提升到共享层。不能以代码重复或两端都是整数为抽取依据。

必须选定 punctum-ui 的处置方向，明确 Terminal 与 GPU 私有边界，并把 T3/F3c 改写成可直接执行的原子任务。只修改管理文档和决策直接需要的 README 状态。不要修改 Rust、manifest 或 lockfile。完成后停止。
```

## `T3`：Tetris Visual Composition（等待 `B3a` 重写）

`T3` 依赖 `T2` 与 `B3a`。它只组合页面，不增加框架能力或游戏规则。

页面必须包含标题、居中棋盘、消行数、状态、操作提示、game-over 和 Ghost Piece。Terminal/GPU 使用相同的信息层级和布局意图，但不要求像素一致。小窗口不能重叠。

`T3` 只验证离散 UI 组合。它不能把逻辑 grid slot、Terminal Cell 和 GPU instance 合并为公共 Cell，也不能把当前整数 `GridSize` 和 `GpuCell` 提升为 Native GPU GUI 合同。GPU 文字、普通图片和 tile 的最终几何仍属于架构计划中的开放问题。

### 初始 Prompt

以下 Prompt 只保留历史意图。`B3a` 完成前不得执行。`B3a` 必须根据最终共享边界替换它。

```text
你叫 TetrisVisual。

确认 T2 与 F3b 已通过，然后阅读三份 Punctum/Ramus 文档。执行原子任务 T3：使用 punctum-ui 重做 Tetris Terminal/GPU 页面。

组合 Text、Row、Column、Border、Padding、Spacer、Align 和 SurfaceView。页面显示标题、居中棋盘、累计消行数、运行或 game-over 状态、操作提示和 Ghost Piece。两端共享布局意图，各自完成 cell 或 glyph/sprite 投影。小窗口允许裁剪或收紧间距，但不能重叠或修改游戏状态。

不要新增万能 Cell，不要把 GPU 文字或普通图片塞进 `GpuCell`。不要修改 punctum-ui 实现，不增加 hold、next preview、score、level、动画、音效或新命令。只修改 apps/tetris/** 和必要的 Tetris lockfile。完成后停止。
```

## `F3c`：Interaction Foundation（等待 `B3a` 重写）

`F3c` 依赖 `B3a`。当前不能假定 focus registry、event region 和 Button 必须位于现有离散 `punctum-ui` 中。

### 初始 Prompt

以下 Prompt 只保留历史意图。`B3a` 完成前不得执行。`B3a` 必须明确共享 interaction 合同和 backend 私有命中区域后替换它。

```text
你叫 InteractionFoundation。

确认 F3b 已通过，然后阅读三份 Punctum/Ramus 文档。执行原子任务 F3c：实现 provisional focus、event dispatch 和 Button。

实现类型安全 WidgetId、focus scope、Tab/Shift+Tab、event consumption 和 Button。widget 只返回 action，不能修改 Tetris 或 Battle 状态。没有 Game 真实需求时不要实现 ChoiceList、List、ScrollView 或其他集合 widget。

只修改 punctum-ui 的实现、rustdoc、合同测试和 Punctum lockfile。不要修改 Tetris、Game 或管理文档。完成后停止。
```

## `B3`：UI Composition Barrier

`B3` 只依赖 `T3` 与 `F3c`。它接收 manifest、lockfile、项目注册、baseline 和文档状态，不增加功能。

## 后续节点

- `F4`：在 `B3` 后继续 Battle/Ramus harness、Game host 和完整对战 composition。`F3e` 的 probe 只有被 `B3a` 接受后才能成为正式 Game GUI 基础。
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
| `T2 Ghost Piece Projection` | 已通过；coverage 注册待 `B3` 接收 |
| `F3b Stateless UI Foundation` | 已通过 |
| `F3d Game Screen Model` | 可启动 |
| `F3e Native GPU GUI Probe` | 等待 `F3d` |
| `B3a UI Boundary Decision` | 等待 `F3e` |
| `T3 Tetris Visual Composition` | 暂停；等待 `B3a` 重写 |
| `F3c Interaction Foundation` | 暂停；等待 `B3a` 重写 |
| `B3 UI Composition` | 等待 `T3/F3c` |
| `Native GPU GUI Geometry` | Open；由 `F3d/F3e/B3a` 关闭 |
| `GPU-REF-v0.1` | Blocked；不阻塞本地工作 |

## Handoff

每个 task 完成后只报告：

- 修改路径。
- 实际行为结果。
- 执行的聚焦验证及退出码。
- 未通过门禁和残余风险。
- 下游需要接收的 manifest、lockfile 或公开 API 变化。
