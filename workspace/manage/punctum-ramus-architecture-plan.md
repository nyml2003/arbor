# Punctum / Ramus 第一期架构计划

- 状态：已批准
- 批准日期：2026-07-11
- 实现状态：`S0`、三个 `F1` lane、`B1`、`PT1`、`F2`、`R2` 与 `B2a` 已完成；等待 `F3a` 完成后进入汇合任务 `B2b`
- 评审结果：Planner、Architect、Critic 共识通过，最终 Critic 结论为 `APPROVE`
- 产品事实来源：[项目群总控](./punctum-ramus-program.md)

## 文档职责

本文是第一期实现的正式架构与 Agent 编排依据。它定义共享边界、workspace 拓扑、依赖方向、验证门禁、写入所有权和执行 wave。

产品范围、玩家可见行为和权限政策仍以项目群总控为准。本文不能反向修改产品事实。实现中的技术决定一旦改变产品行为，必须返回用户确认。

本计划覆盖 Punctum、Ramus、游戏、游戏内控制台和 Tetris 基座验证。TUI AI Chater 保留 workspace 与历史记录，但暂停实现，不进入当前 wave 和 barrier。仓库中其他已有项目不在范围内，也不构成兼容约束。

Punctum 在接入 Poke Game 前先使用独立 `apps/tetris` 项目验证 grid、input、Terminal、GPU adapter 和 provisional UI。Tetris 不扩大最终游戏范围，也不能独自证明公共 UI API 已经跨产品稳定。

## 已批准结论

第一期采用 `grid/input only` 方案。Punctum 的强制共享基础只有：

- 二维离散空间、geometry、`Surface<T>`、diff 和 `Patch<T>`。
- 规范化键盘事件与文本输入事件。

第一期不创建共享 `interaction` crate，也不冻结以下上层概念：

- component tree。
- component lifecycle。
- focus runtime。
- widget system。
- layout tree。
- event routing。
- retained render tree。
- 共享 `NodeId` 或 `TargetId`。

Tetris 可以先验证 selection、focus 和 routing 的 provisional 合同。只有 Game 实际消费并完成 API 收窄，或两个独立真实消费者出现同构重复时，才能通过新 ADR 冻结上层 interaction 能力。

触发新 ADR 必须同时满足：

1. 两个独立真实消费者具有相同 input alphabet、state shape、transition 和 output oracle。
2. 同一套 black-box suite 可以原样运行于两个实现。
3. 候选 public contract 不含 battle、chat 或 backend 类型。
4. 抽取会删除两套重复实现，而不是增加 wrapper。

Tetris 是 Punctum 的 proof project，不是通用需求来源。Tetris 的棋盘、方块、计分、tick 和控制命令属于业务代码。它们不能进入 `punctum-grid`、`punctum-input`、`punctum-terminal` 或 `punctum-gpu`，也不能单独触发稳定 interaction API。

## 上层 UI 组件候选

本节记录后续设计方向，不改变当前 `grid/input only` 强制共享合同，也不把候选 API 视为已经实现。公共 component tree、widget、focus 和 event routing 仍需新 ADR 与真实消费者证据。

### 分层与依赖

上层 UI 使用四层模型：

```text
Feature Component
        -> Shared Pattern
        -> Framework Widget
        -> UI Primitive
        -> layout / surface / input contracts
        -> Terminal or GPU adapter
```

- UI Primitive 只负责约束、布局、绘制、裁剪和样式，不知道业务。
- Framework Widget 在 Primitive 上增加稳定 identity、焦点、状态和通用交互，不知道业务。
- Shared Pattern 表达多个界面重复使用的组合模式。它可以带产品样式和轻量语义，但不能执行业务规则。
- Feature Component 显示业务状态并产生业务 action。业务状态转换仍由 application 或 domain 完成。
- adapter 解释最终 surface、patch 或绘制计划。组件不得判断底层使用全帧、diff、ANSI 或 GPU submission。

概念分层不等于一个层级对应一个 crate。初期候选物理结构为一个 `punctum-ui` crate，内部使用 `primitives` 和 `widgets` 模块。产品 Shared Pattern 与 Feature Component 留在消费方。只有独立依赖、编译成本、API 生命周期或多个消费者证明需要物理隔离时，才评估拆为 `punctum-ui-core` 和 `punctum-widgets`。

### UI Primitive 候选

| 类别 | 候选 | 责任 |
| --- | --- | --- |
| 基础容器 | `Box` | 约束单个 child，并承载基础背景或边框属性 |
| 线性布局 | `Row`、`Column` | 在主轴排列 children，处理 gap、对齐和剩余空间 |
| 叠放与换行 | `Stack`、`Wrap`、`Grid` | 处理覆盖、换行和规则行列布局 |
| 空间控制 | `Padding`、`Spacer`、`SizedBox`、`ConstrainedBox` | 增加留白并收紧或声明尺寸约束 |
| 弹性控制 | `Expanded`、`Flexible` | 参与 Row/Column 的剩余空间分配 |
| 对齐 | `Align`、`Center` | 在已分配区域内定位 child |
| 文本与装饰 | `Text`、`Border`、`Background`、`Divider`、`Fill` | 生成只读文本和基础视觉单元 |
| 绘制接入 | `SurfaceView`、`CustomPaint` | 把已有 surface 或受限自定义绘制接入组件树 |
| 裁剪与样式 | `Clip`、`Style` | 限制绘制区域，并向子树提供样式值 |
| 树辅助 | `Empty`、`Fragment`、`Visibility`、`Keyed` | 表达空节点、组合、可见性和稳定身份 |
| 交互区域 | `FocusRegion`、`EventRegion` | 声明焦点或命中边界，不执行产品 action |

表中条目不要求全部成为独立 Rust 类型。`Center` 可以是 `Align` 的构造函数，条件分支可以由普通 Rust 控制流生成，不能为了目录完整而创建空抽象。

### Framework Widget 候选

| 类别 | 候选 |
| --- | --- |
| 基础操作 | `Button`、`ToggleButton`、`Checkbox`、`RadioGroup`、`Switch`、`Link`、`IconButton` |
| 文本输入 | `TextInput`、`TextArea`、`PasswordInput`、`SearchInput`、`NumberInput` |
| 选择与导航 | `Select`、`Dropdown`、`Tabs`、`Menu`、`MenuBar`、`ContextMenu`、`Breadcrumb`、`Pagination` |
| 集合与视口 | `List`、`Table`、`Tree`、`GridView`、`ScrollView`、`Scrollbar`、`VirtualList` |
| 浮层与反馈 | `Dialog`、`Popover`、`Tooltip`、`Toast`、`ProgressBar`、`Spinner`、`StatusMessage` |
| 数值操作 | `Slider`、`Stepper` |

Widget 依赖的 `WidgetId`、focus registry、event dispatch、disabled/hover/pressed/focused 状态、action message、overlay、clip stack 和 theme 属于 runtime contract，不应伪装成业务组件。

### Shared Pattern 与 Feature Component

Shared Pattern 默认位于消费方的 shared UI 模块，例如 `FormField`、`LabeledValue`、`ShortcutHint`、`StatusBar` 和基于通用 `Dialog` 组合的 `ConfirmationDialog`。当两个独立消费者对同一 pattern 使用相同 input alphabet、state shape、transition 和 output oracle 时，才允许通过 ADR 提升到 Punctum。

Feature Component 永远位于业务项目。例如：

- Tetris：`TetrisBoard`、`ScorePanel`、`GameOverOverlay`、`TetrisScreen`。
- Game：`BattleCommandPanel`、`PartyStatus`、`BattleLog`。
- Chater：`ChatMessage`、`ConversationView`、`ModelSelector`、`Composer`。

Feature Component 只能返回业务 action。以重开为例，`Button` 产生激活事件，`RestartButton` 映射为 `TetrisCommand::Restart`，Tetris application/core 负责生成新状态。UI 不能直接清空棋盘。

### 实施顺序

完整 UI foundation 不得插入已经完成的 adapter lane。`F3a` 只修复现有 Terminal 输出 API 的职责错误，不创建 `punctum-ui`，因此可以与 `B2a` 并行。`B2b` 完成 Terminal/GPU 基础验证后，才启动 `F3b` 的新 ADR 和 UI 实现。

第一组只验证无状态布局和绘制：

1. 定义 constraints、measure、layout、paint、clip 和 resize 合同。
2. 实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
3. 用 Tetris 组合棋盘、标题、累计消行数、快捷键提示和 game-over 表现。
4. 使用精确 layout fixture、surface golden、零尺寸、空间不足、嵌套 clip 和 resize 测试验证行为。

第二组验证通用交互：

1. 定义 `WidgetId`、event dispatch、focus registry、`Tab`/`Shift+Tab` 和 action message。
2. 先实现 `Button`，用 Tetris restart 验证从键盘事件到业务 action 的完整链路。
3. 只有 Game 提供真实需求和 oracle 后，才实现 `Checkbox`、`List` 和 `ScrollView`。

Terminal 只读文本投影已经完成 grapheme 分割、Unicode width、continuation cell、覆盖修复、裁剪、resize 清理和 cursor 停放，并由 `punctum-terminal` 的 28 项合同测试验证。该结果不等于共享 `Text` Primitive 已完成。

当前 `write_text` 仍接收输入侧 `TextEvent`。这属于 provisional adapter API，只能用于已经提交的文本；共享只读 `Text` 必须接收文本内容或独立文本模型，不能依赖输入事件。冻结 `Text` 前还要统一 measure 与 paint 使用的文本布局结果。

`F3a` 先修复现有输出边界：`write_text` 接收 `&str`，固定标题和业务文字直接传入字符串，输入组件需要显示已提交内容时显式传入 `event.text()`。`punctum-terminal` 随后删除对 `punctum-input` 的依赖。该任务不实现系统字体、GPU 文字、公共 `Text` Primitive 或 `TextInput`。

`TextInput` 继续后置。进入实现前仍须明确 IME composition、光标、选区、删除语义和滚动合同。Terminal 与 GPU 可以采用不同文本投影，但组件层共享文本内容、布局约束和编辑意图，不能强求两个 backend 产生相同 glyph 布局。

### 提升门禁

第一组可以作为 `punctum-ui` 的 provisional API 实验，但不能因为 Tetris 页面可用就冻结。提升为第一期公共合同必须满足：

1. 至少由 Game 的一个真实 feature 使用；稳定跨产品合同还需要第二个真实消费者完成 API 适配评审。
2. public contract 不包含 Tetris、battle、chat、Crossterm、winit 或 wgpu 类型。
3. Terminal 与 GPU 的提交模式不会泄漏到组件 API。
4. pure layout 和 paint 行为具有 100% line、function 和 region coverage。
5. 新 ADR 明确 component identity、state ownership、layout overflow、text clipping 和 error model。

## 场景渲染边界

Scene 与 UI Primitive 并列。`Text`、`Row`、`Column` 和 `Border` 不负责地图坐标、相机或纹理图集。

```text
game domain: TilePos / TileId / collision / actor state
        -> scene view: Camera / Viewport / TileLayer / SpriteLayer
        -> adapter projection
             Terminal: visible object -> TerminalCell
             GPU: visible object -> atlas region + instance + pixel rect
```

- `punctum-grid` 只提供离散坐标、尺寸、矩形、裁剪和 `Surface<T>`，不认识像素、DPI、atlas 或 sprite。
- 游戏领域持有地图尺寸、tile 数据、碰撞和角色逻辑位置，不读取 backend 类型。
- Scene 决定可见范围、层顺序和逻辑对象，不决定 Terminal ANSI 或 GPU submission。
- adapter 执行最终单位转换。Terminal 通常把一个逻辑槽投影为一个或多个 Cell；GPU 根据 viewport、cell size 和 DPI 投影为像素。
- 稠密 `TileMap` 可以使用 `Surface<TileId>`，角色、特效和天气使用独立 Sprite/Effect layer，不把所有场景对象压成万能 Cell。

当前不创建 `punctum-scene` crate。`F2/B2a/B2b` 只冻结 grid 到 GPU viewport 的转换边界，并用 Tetris 验证修改 cell pixel size 不改变业务状态。完整 `Camera`、`TileLayer` 和 `SpriteLayer` 在宝可梦地图垂直切片前实现；出现稳定概念后再决定留在 Game 还是提取为 Punctum scene crate。

## Workspace 拓扑

第一期当前使用五个独立 Cargo workspace。不在 Arbor 根创建 Cargo workspace，也不创建 umbrella workspace。

| Workspace | Root manifest | 计划成员 | Lockfile |
| --- | --- | --- | --- |
| Punctum | `apps/punctum/Cargo.toml` | `punctum-grid`、`punctum-input`、`punctum-terminal`、`punctum-crossterm`、`punctum-gpu`、`punctum-wgpu` | `apps/punctum/Cargo.lock` |
| Tetris | `apps/tetris/Cargo.toml` | `punctum-tetris` | `apps/tetris/Cargo.lock` |
| Ramus | `packages/ramus/Cargo.toml` | `ramus-core` | `packages/ramus/Cargo.lock` |
| Game | `apps/gen3-game/Cargo.toml` | battle、UI、Ramus adapter、host、E2E crates | `apps/gen3-game/Cargo.lock` |
| Chater | `apps/tui-chater/Cargo.toml` | 暂停；只保留既有 workspace 空壳和历史 lockfile | `apps/tui-chater/Cargo.lock` |

五 workspace 方案用于隔离项目所有权。当前没有足够证据支持根 workspace 或专用 umbrella workspace。只有实际出现不可接受的 dependency、lockfile 或验证成本时，才能通过新 ADR 重新评估。

Tetris 项目固定为 `apps/tetris`，package name 为 `punctum-tetris`。它拥有独立 manifest 和 lockfile，通过 canonical path dependency 使用 Punctum，只依赖 `punctum-grid` 和 `punctum-input` 作为正常依赖。

两个副作用入口归 Tetris 项目所有，不放进 adapter crate：

- `apps/tetris/examples/terminal/main.rs`。
- `apps/tetris/examples/gpu/main.rs`。

`punctum-tetris` 的正常依赖仍只有 grid/input。可执行 example 通过 dev-dependency 消费 adapter。adapter crate 不依赖 Tetris 业务代码。汇合任务在 barrier 接受 canonical path dependency、Tetris dev-dependency 和最终 lockfile 状态。

2026-07-12 用户决定把 Tetris 从 Punctum 内部 example package 迁为 `apps/tetris` 独立项目。本决定覆盖后续拓扑和写入所有权，但不改写 `S0/B1` 对迁移前四 workspace 状态的历史记录。

## 写入所有权

写入所有权属于 task，不属于常驻 Agent 角色。每个 DAG 节点在启动前列出完整写入范围；同一时刻一个文件只能有一个 owner。所有权只在依赖边上传给后续节点，Agent 复用不会自动继承前一个任务的写入权限。

root manifest、lockfile、canonical path dependency、composition root 和跨域 E2E 可以由原子实现任务直接拥有，也可以由汇合任务拥有。选择标准只有两个：该文件是否是任务完成结果的一部分，以及是否会与并行节点冲突。

当前 wave 固定如下：

- `F3a` 独占 Punctum Terminal 相关文件、`apps/punctum/Cargo.lock` 和 Tetris Terminal view，不写 Tetris manifest/lockfile。
- `B2a` 独占 Tetris GPU 文件、`apps/tetris/Cargo.toml` 和 `apps/tetris/Cargo.lock`，不写 Terminal 或 Punctum 文件。
- `B2b` 在两个上游结束后接管最终 manifest、lockfile、验证注册和管理文档。它不修改业务实现。

两个并行 task 不得修改同一文件。需要扩大写入范围时，当前 task 停止并提出新的 DAG 节点，不能在原 Prompt 内追加职责。

## Path dependency 与版本门禁

path dependency 只在消费方 workspace root 的 `[workspace.dependencies]` 中定义。member manifest 只使用 `{ workspace = true }`。

| Consumer | Dependency | Cargo path | Canonical repo-relative target |
| --- | --- | --- | --- |
| Game | `punctum-grid` | `../punctum/crates/punctum-grid` | `apps/punctum/crates/punctum-grid` |
| Game | `punctum-input` | `../punctum/crates/punctum-input` | `apps/punctum/crates/punctum-input` |
| Game | `punctum-gpu` | `../punctum/crates/punctum-gpu` | `apps/punctum/crates/punctum-gpu` |
| Game | `ramus-core` | `../../packages/ramus/crates/ramus-core` | `packages/ramus/crates/ramus-core` |
| Chater | `punctum-grid` | `../punctum/crates/punctum-grid` | `apps/punctum/crates/punctum-grid` |
| Chater | `punctum-input` | `../punctum/crates/punctum-input` | `apps/punctum/crates/punctum-input` |
| Chater | `punctum-terminal` | `../punctum/crates/punctum-terminal` | `apps/punctum/crates/punctum-terminal` |

verifier 必须 canonicalize path，并确认目标位于 repo 内且等于批准路径。symlink、absolute dependency 或解析到其他副本全部拒绝。

每个 wave 为五个 workspace 分别记录：

```text
root_manifest_sha256
sorted_member_list_sha256
member_manifest_sha256_by_path
lockfile_sha256
approved_upstream_export_sha256
```

`upstream_export_sha256` 覆盖批准 crate 的 manifest、`src`、public fixtures 和 contract tests。consumer task packet 固定所需 hash。任务开始、handoff 和 verifier 重跑前都要复核。hash 改变时，下游任务进入 `Blocked`。

## 合同边界

### `punctum-grid`

提供 `GridPos`、`GridSize`、`GridRect`、`Surface<T>`、clip、blit、diff 和 `Patch<T>`。

不包含 identity、component state、focus、input、backend cell 或产品类型。

核心不变量：

- 容量计算不溢出。
- patch 始终有界。
- span 排序且不重叠。
- `apply(previous, diff(previous, next)) == next`。

### `punctum-input`

```text
KeyEvent { physical, logical, modifiers, phase }
TextEvent { text }
```

adapter 只能表达 host 实际提供的 press、repeat 和 release，不能伪造缺失事件。`punctum-input` 不负责 focus、dispatch、command binding 或 application state。`LogicalKey::Character` 只表达键盘布局给出的字符标签；只有 `TextEvent` 表达已经提交、可以插入的 Unicode 文本。

### Tetris proof example

Tetris example 使用一套共享业务核心和两个本地运行入口：

```text
TetrisState + injected piece sequence + command -> next TetrisState
TetrisState -> Surface<TetrisCell>
KeyEvent -> TetrisCommand

terminal host -> punctum-terminal -> shared Tetris core
gpu host      -> punctum-gpu      -> shared Tetris core
```

- Tetris 核心包含棋盘、方块、移动、旋转、下落、锁定、消行、结束和重开。
- tick、时间和方块序列由 host 注入。纯核心不读取系统时钟、随机源、文件、终端或窗口。
- `TetrisState -> Surface<TetrisCell>` 和 `KeyEvent -> TetrisCommand` 是纯函数。
- Terminal 与 GPU 入口只负责事件循环、tick 驱动和 adapter 调用，不复制规则、状态转换或绘制逻辑。
- example 不向 Punctum 内核加入 Tetris 类型，不创建 widget、focus、layout 或 routing 抽象。
- Tetris 的成功只证明 grid/input、text、adapter 和 provisional UI 可用。Punctum 的稳定公共 UI 合同仍需 Poke Game 实际消费和收窄。

`PT1` 使用以下最小规则，不追求 Tetris Guideline 完整兼容：

- 棋盘为 10 列 x 20 行，不设置隐藏行。
- 方块包含 `I`、`O`、`T`、`S`、`Z`、`J`、`L`。构造状态时注入非空方块序列；核心循环使用该序列，restart 时游标归零。
- 新方块在顶部水平居中生成。生成位置发生碰撞时进入 game over。
- 命令只有 left、right、rotate clockwise、soft drop、hard drop、tick 和 restart。
- 旋转只做顺时针局部矩阵变换。越界或碰撞时保持原状态。第一期不做 wall kick。
- tick 和 soft drop 每次下降一格。无法下降时立即锁定。hard drop 落到最低合法位置后立即锁定。
- 锁定后同时删除所有完整行，记录累计消行数，再生成下一个方块。
- restart 清空棋盘、消行数和 game-over 状态，并重置方块序列。其他命令在 game over 时不改变状态。
- 第一阶段不做 hold、ghost piece、next preview、level、score、lock delay、音效或动画。

`paint` 固定产生 12 x 22 的 `Surface<TetrisCell>`：10 x 20 棋盘外加一格边框。`TetrisCell` 只区分 empty、border 和带 `PieceKind` 的 tetromino。活动方块覆盖在已锁定棋盘上。

input mapping 固定为：Left/Right/Down 的 press 与 repeat 分别映射移动和 soft drop；Up press 映射顺时针旋转；Space press 映射 hard drop；physical `KeyR` press 映射 restart。没有 physical-key channel 的来源允许用 logical `r/R` press 作为 restart 回退，不能伪造 physical key。release 和其他事件返回无命令。tick 只由 host 注入，不由键盘事件生成。

### Terminal adapter

- `punctum-terminal` 保留 grapheme `TerminalCell`、Unicode width、continuation、覆盖修复、整 grapheme 裁剪、resize 清理和 patch planning。
- `punctum-crossterm` 把 raw Terminal event 转换为 `punctum-input`，并把 `Surface<TerminalCell>` 和 `Patch` 转换为 ANSI 输出。
- presenter、raw-mode session、cursor 和真实终端 IO 只位于 `punctum-crossterm`。
- 两个 crate 都不持有 chat state。

`R2` 后 `punctum-terminal` 有 19 项纯合同测试，`punctum-crossterm` 有 9 项平台合同测试。原 28 项合同按所属边界迁移，没有复制同类测试。

### GPU adapter

- `punctum-gpu` 保留 atlas、viewport、cell、resource lookup、submission planning、instance encoding 和 uniform encoding。
- `punctum-wgpu` 把 winit keyboard event 转换为 `punctum-input`，并执行 wgpu submission。
- texture、shader、pipeline、surface、device 和真实 GPU 操作只位于 `punctum-wgpu`。
- 两个 crate 都不持有 game state。

Terminal 与 GPU backend 共享 geometry、surface 和 diff，不共享万能 `Cell`。

### Battle

`battle-domain` 持有 deterministic state 和 rule。`battle-application` 暴露由 `BattlePerspective` 绑定的 observation、legal action、submit 和 event log。它不公开完整 state query、raw domain event 或由调用方传入任意 `Side` 的操作入口。

`BattleApplication` 只由 trusted host 持有。host 创建双方 perspective，并把其中一个交给 Human keyboard UI，把另一个交给 `battle-ramus-adapter`。两条路径都把输入映射为 `Action`，通过各自 perspective 调用同一 application API。Player、Agent 和 adapter 不能自行调用 `perspectives()`，也不能访问 domain 内部状态。

observation 由 `battle-application` 生成，不能由 UI 或 Ramus adapter 从完整状态自行裁剪：

- 己方可读取完整队伍、上场槽位、当前 HP、能力值、招式和 PP。
- 对方只显示当前上场成员、按揭示顺序保存的已见后备成员和未见成员数量。
- 对方已见成员公开身份、等级、属性、HP 和实际执行过的招式。隐藏能力值、PP、队伍顺序、内部 `TeamSlot` 和未见成员详情。
- 对手 command、switch 和 PP event 使用相对投影，不暴露 action、招式槽、队伍槽或剩余 PP。
- pending command 在双方锁定前不属于任何 Player/Agent observation 或公开 event。
- Human 与 Agent 使用同一 observation 合同。principal capability 决定可以获得哪一个 perspective，不改变 observation 语义。

### Ramus

```text
ShellText -> AST -> PlanDraft
Agent output -------> PlanDraft
PlanDraft -> resolve -> schema/type validation -> sealed TypedPlan -> execute
```

- `PlanDraft` 永远不可信。
- `TypedPlan` 不能公开反序列化，也不能绕过 validator 构造。
- capability 使用 default-deny。
- discover、complete、read、write 和 invoke 分别授权。
- `resolve`、schema lookup 和 diagnostic 必须使用 capability-filtered registry view。
- 未授权 command 对 principal 表现为不可发现，不能从错误、补全或 schema diagnostic 泄漏存在性。
- `TypedPlan` 记录 `PrincipalId`、provider/command identity、registry generation、schema version 和 effect requirement。
- sealing 不构成永久授权。

每个 read、write 或 invoke effect 执行前，authorization service 原子校验 principal、registry/schema version 和 capability generation，并签发不可序列化、不可复制、单次消费的 `EffectPermit`。provider 必须消费 permit 才能执行。

permit 签发是 authorization linearization point。撤权先发生则 effect 拒绝；permit 先签发则当前 effect 可以完成，后续 effect 仍需重新授权。多 effect plan 在首次拒绝处停止，已完成 effect 不自动回滚。需要原子性的 command 由 application 或 provider 提供 transaction。

### TUI AI Chater（暂停）

Chater workspace 和历史架构记录保留，但当前不实现 `chat-application`、model port、UI、host 或 E2E。恢复时重新定义产品合同；Terminal adapter 仍不得持有 chat state。

## 依赖方向

```text
# A -> B 表示 A depends on B

game-host -> game-ui -> battle-application -> battle-domain
game-host -> punctum-wgpu -> punctum-gpu + punctum-grid + punctum-input
game-ui -> punctum-gpu -> punctum-grid
game-ui -> punctum-input

battle-agent -> battle-ramus-adapter -> battle-application
game-console -> battle-ramus-adapter -> ramus-core
battle-ramus-adapter -> ramus-core
ramus-core -X-> battle-domain

# paused: tui-host -> chater-ui -> chat-application -> model-port
# paused: tui-host -> punctum-crossterm -> punctum-terminal + punctum-grid + punctum-input
# paused: chater-ui -> punctum-grid + punctum-input

tetris-terminal-host -> punctum-crossterm -> punctum-terminal + punctum-grid + punctum-input
tetris-gpu-host -> punctum-wgpu -> punctum-gpu + punctum-grid + punctum-input
tetris-terminal-host + tetris-gpu-host -> tetris-core -> punctum-grid + punctum-input

punctum-terminal -X-> Crossterm
punctum-gpu -X-> punctum-input / winit / wgpu
punctum-grid/input -X-> game / chat / Ramus / Crossterm / winit / wgpu
```

## Battle Rule Fixture Gate

- semantic owner：用户或 Product Owner。
- custodian：Program Integration Agent。
- identity：`BATTLE-RULES-v0.1`。
- tracked approval record 保存 canonical fixture bundle 的 SHA-256。

fixture 未批准、缺失或 hash 不符时，`battle-domain` 和所有 game downstream task 标记 `Blocked`。Agent 不得自行补规则，也不得修改 fixture 迁就实现。grid/input、Ramus、Terminal 和 GPU adapter lane 可以继续。

本门禁已于 2026-07-11 由用户批准。canonical fixture 为 `apps/gen3-game/fixtures/battle-rules-v0.1.json`，hash 以 `workspace/manage/punctum-vsh-s0/records.json` 为准。批准范围是第三世代 6v6 单打核心规则，暂不实现特性、道具和复杂状态。

## GPU Reference Gate

GPU release oracle 使用 tracked record `GPU-REF-v0.1`。主 adapter 固定为 pinned Linux CI image 中的 Mesa `llvmpipe` Vulkan software adapter。

record 必须记录并精确匹配：

```text
OCI image digest
Mesa package version
LLVM version
wgpu version
backend = Vulkan
AdapterInfo.name
AdapterInfo.vendor
AdapterInfo.device
AdapterInfo.device_type = Cpu
AdapterInfo.driver
AdapterInfo.driver_info
approved_fixture_sha256
```

任一字段为空、CI image 使用 mutable tag 或 runtime identity 不匹配时，GPU readback 和 release gate 标记 `Blocked`。

普通 hardware adapter 只运行 logical oracle 和 smoke test。fallback 必须有独立 pinned image、identity record、golden 和 Product Owner 批准。第一期不预先批准 fallback。

## 测试矩阵

| 范围 | Oracle |
| --- | --- |
| grid | scalar full-frame reference；property test 验证 diff/apply |
| input | Terminal/GPU raw fixture 与 canonical event fixture 精确相等 |
| Tetris core | deterministic piece fixture；移动、旋转、下落、锁定、消行、结束和重开 |
| Tetris render/input | 状态到 `Surface<TetrisCell>` golden；canonical input 到 `TetrisCommand` fixture |
| Terminal | in-memory terminal golden，覆盖 resize、wide cell、cursor |
| GPU logical | CPU reference 的 coordinate、resource ID、clip、order 和 instance data 精确相等 |
| GPU readback | `GPU-REF-v0.1`；固定 `Rgba8Unorm`、viewport、scissor、MSAA 1、nearest sampling、atlas 和 clear color |
| Battle | 已批准 `BATTLE-RULES-v0.1` vector 与 replay hash |
| Ramus | capability matrix、malformed draft、bypass 和 TOCTOU concurrency |
| Human/Ramus | 相同 `BattleCommand` 产生相同 application event log |
| Chater | 暂停；恢复后重新定义 deterministic oracle |

GPU readback 去除 row padding，归一为 top-left RGBA8。逐通道绝对误差不超过 1。普通 hardware adapter 结果不属于 release oracle。

Ramus TOCTOU 必须覆盖：

- seal 后、首个 effect 前撤权，handler 调用 0 次。
- effect 1 后撤权，effect 2 不执行并返回 `AuthorizationRevoked`。
- registry 或 schema version 变化时拒绝旧 `TypedPlan`。
- revoke 与 permit issuance 并发时符合 linearization order。
- principal 和 context 没有伪造或反序列化路径。
- principal 与 operation authorization matrix 覆盖 100%。

### TDD 与 coverage 原则

Punctum 使用 TDD。失败测试必须先表达可观察行为、不变量、边界合同或正常失败，再实现最小通过代码，最后重构。不得只为了执行某一行、getter、平台错误分支或 IO 路径而增加没有行为价值的单元测试，也不要求一个函数对应一个测试。

纯逻辑 crate 使用 stable toolchain 下的 `cargo llvm-cov`，line、function 和 region coverage 必须全部达到 100%。这是 TDD 完成后的遗漏检查，不是测试需求来源。实验性 branch coverage 只有在已批准的 nightly toolchain 可用时才作为附加证据，不能替代 stable coverage 门槛。

平台和 IO crate 不设置 coverage 百分比。Terminal event loop、系统 IO、窗口生命周期、wgpu device/surface/pipeline 和 GPU submission 使用合同测试、fixture、in-memory backend、错误场景、logical oracle、smoke 和 E2E 验证。不能为了提高 coverage 把平台调用包装成没有业务意义的 mock 测试。

纯逻辑与副作用的边界必须由 crate 表达。完成 `R2` 后，不再使用 `ignore-filename-regex`、`runtime.rs` 或其他文件名约定决定 coverage 范围。能独立确定输入输出的转换、布局、资源解析、提交规划和字节编码应位于纯逻辑 crate；真正调用平台 API 的代码位于平台 crate。

第一期先在 Windows 11 本地跑通。Terminal 人工验证使用 Windows Terminal；GPU 人工验证使用本机可用的 wgpu adapter。当前不建设 CI。`GPU-REF-v0.1`、llvmpipe readback 和正式 release oracle 保持 `Blocked`，但不阻塞本地 GPU adapter、Tetris GPU 入口和 smoke test。Ollama 和 DeepSeek live smoke test 不作为 deterministic oracle。

## 2026-07-12 本地验证记录

本期项目的本地验证已统一进入标准库 Python 工程 `packages/arbor-projects`。入口为 `python packages/arbor-projects/run.py verify <project-id>`。注册表包含 `punctum`、`tetris`、`ramus`、`gen3-game` 和 `tui-chater`。

Python 工程使用不可变 dataclass 领域模型和 typing.Protocol 端口。文件、进程和 LLVM 工具调用只位于 adapter。生产函数体内不得出现字面常量；AST 门禁负责检查该约束。

本地结果：

- Python 单元测试 41 项全部通过。
- Python 纯逻辑覆盖率：domain 170/170、application 130/130、JSON registry parser 118/118、LLVM export parser 85/85、LCOV parser 227/227。
- Punctum 的 format、workspace test、Clippy 和四个 coverage target 门禁全部通过。以下 Terminal/GPU 数字是 `R2` 前使用 `runtime.rs` 排除规则取得的历史基线，不代表最终 crate 边界。
- punctum-grid 为 450/450 regions、40/40 functions、325/325 lines。
- punctum-input 为 19/19 regions、3/3 functions、14/14 lines。
- punctum-terminal 为 457/457 regions、30/30 functions、347/347 lines。
- punctum-gpu pure planner 与输入转换为 560/560 regions、37/37 functions、460/460 lines。
- Punctum GPU headless smoke 在本机 adapter 上通过。该测试实际创建 device、shader、pipeline 和 atlas texture。测试发现并修复 WGSL 保留字 `target` 导致的 shader parse failure。
- `R2` 后 `punctum-terminal` 不使用文件名排除，结果为 385/385 regions、26/26 functions、290/290 lines。
- `R2` 后 `punctum-gpu` 不使用文件名排除，结果为 449/449 regions、32/32 functions、332/332 lines。该结果包含 instance/uniform encoding。
- `R2` 后 Punctum workspace 仍为 109 项测试；`punctum-crossterm` 的 9 项合同、`punctum-wgpu` 的 8 项输入合同和单独运行的 ignored headless smoke 均通过。Clippy 以 `-D warnings` 通过。
- Tetris 的 format、29 项测试、Clippy、core 覆盖率和 Terminal view 单文件覆盖率全部通过。core 为 328/328 regions、35/35 functions、257/257 lines；Terminal view 为 150/150 regions、11/11 functions、105/105 lines。
- `B2a` 新增 8 项 Tetris GPU 纯逻辑测试。`cargo test --all-targets --locked` 当前通过 24 项 core、8 项 GPU 和 6 项 Terminal 测试；Clippy 以 `-D warnings` 通过。Tetris core 继续保持 328/328 regions、35/35 functions、257/257 lines。
- Tetris GPU 本机窗口已完成 adapter、surface、shader、pipeline、白色 atlas 和 submission 初始化。首帧使用完整 surface，后续帧使用 diff patch。窗口由用户手动关闭，进程退出码为 0，未出现 GPU 初始化或提交错误。
- Ramus Python verifier 的 format、test、Clippy 和 typed LCOV branch coverage 全部通过。生产源码为 1529/1529 line entries、158/158 functions、148/148 branch entries，missed lines 和 missed branches 均为 0。旧 `Test-PureCoverage.ps1` 同时通过，覆盖 10 个生产文件；新验证器通过后仍保留旧脚本。
- Gen3 Game 和 TUI Chater 的 Python 注册门禁全部通过。因 Punctum path dependency 变化，Punctum、Tetris、Gen3 Game 和 TUI Chater lockfile 已刷新，并通过 `--locked` 复核。

仍有一个限制：

- `GPU-REF-v0.1` 固定环境 readback 尚未执行。本机 headless smoke 不能替代固定 adapter identity 和像素 readback。

## 执行 Wave

每个 task 使用 repo-absolute、task-unique target：

```text
<repo-absolute>/.target/tasks/<wave>/<task-id>/<workspace>
```

### `S0`：串行脚手架

Program Integration Agent 创建四个 workspace、四个 lockfile、canonical path 表、initial baseline、`BATTLE-RULES-v0.1` approval slot 和 `GPU-REF-v0.1` record。

`S0` 不并行。它完成并通过独立验证前，不启动 `F1`。

### `F1`：三个并行 lane

- Punctum lane：grid/input。已按 TDD 完成并通过独立 verifier；纯逻辑 line/function/region coverage 均为 100%。
- Ramus lane：`ramus-core`。已按 TDD 完成，通过安全复审与纯逻辑 100% 覆盖率门禁。
- Battle lane：`battle-domain` 和 `battle-application`。已完成。

### `B1`：串行 barrier

先确认 F1 各 lane 的 public contract 已通过验证。Battle 必须先完成侧别 observation story、隐藏信息测试、pending command 防泄漏测试和纯逻辑 100% 覆盖率门禁。随后接受 crate-local manifest delta，更新对应 lockfile，生成 Punctum、Ramus 和 Battle export hash。Program Integration Agent 同时创建 `apps/punctum/examples/tetris` 的最小 package 空壳，把 `punctum-tetris-demo` 加入 Punctum workspace，使其 grid/input 依赖使用 workspace contract，并更新 lockfile。Tetris writer 不直接修改 root manifest 或 lockfile。

`B1` 已于 2026-07-11 通过。Ramus 物理命名已冻结，Punctum、Ramus、Battle export hash 与四个 workspace baseline 记录在 [`punctum-ramus-b1/records.json`](./punctum-ramus-b1/records.json)。

### `PT1`：Punctum headless Tetris

`B1` 通过后启动单一 Tetris writer。先完成纯业务核心、`TetrisState -> Surface<TetrisCell>` 和 `KeyEvent -> TetrisCommand`。本阶段不接 Terminal/GPU IO，不修改 Punctum 共享内核。所有非副作用代码按 TDD 实现，line/function/region coverage 均为 100%。

`PT1` 已于 2026-07-12 通过。Tetris core 的 region/function/line coverage 为 328/328、35/35、257/257。Terminal view 当前对应结果为 150/150、11/11、105/105。本地门禁由标准库 Python 工程 `packages/arbor-projects` 执行，入口为 `python packages/arbor-projects/run.py verify tetris`。

### `F2`：adapter lane

- Terminal adapter。Unicode width、continuation、覆盖修复、裁剪、resize 清理和 cursor 合同已完成，28 项测试通过。
- GPU adapter。本地 logical planner、wgpu runtime、winit 键盘事件规范化和 headless pipeline smoke 已通过。
- Chater lane 已取消，不再占用 `F2` writer。

GPU adapter 的本地 logical oracle 和 smoke test 不受 `GPU-REF-v0.1` 阻塞。只有固定环境 readback 与 release gate 继续 `Blocked`。

`F2` 已于 2026-07-12 完成。结果如下：

1. `punctum-gpu` 使用可构造的 `WinitKeyEventSnapshot` 接收 winit 的 physical key、logical key、modifier snapshot、state 和 repeat，再转换为 `punctum-input::KeyEvent`。modifier snapshot 由 host 根据独立的 `ModifiersChanged` 事件维护。
2. physical key、logical key、modifier 和 press/repeat/release 均按来源映射。winit 已提供但无法映射的 physical key 为 `Some(PhysicalKeyCode::Unidentified)`；未知 logical key 为 `LogicalKey::Unidentified`。
3. 8 项输入合同测试覆盖方向键、Space、physical `KeyR`、普通字符、未知键、全部 modifier 和全部事件阶段。枚举矩阵同时覆盖 `punctum-input` 可表达的全部 physical key 与 named logical key。
4. 本阶段未实现 IME composition、`TextInput`、文本编辑、Tetris GPU host 或 UI framework，也未把 Tetris 类型放入 adapter。
5. Punctum workspace 的 109 项测试通过，Clippy 以 `-D warnings` 通过。GPU pure coverage 为 560/560 regions、37/37 functions、460/460 lines。单独运行的 ignored headless smoke 通过。

### `R2`：adapter crate boundary

`F2` 已完成，现有行为和测试作为 `R2` 的迁移基线。`R2` 位于 `F2` 与当前并行 wave 之间，只调整物理边界和验证政策，不增加功能，也不改变 Terminal/GPU 可见行为。`R2` 已于 2026-07-12 完成。

`R2` 必须形成以下结构：

1. 当前 `punctum-terminal` 保留 `TerminalCell`、Unicode 文本处理、resize、patch planning 和其他纯逻辑，不再依赖 Crossterm。
2. Crossterm raw event 转换、presenter、session 和真实终端 IO 位于 `punctum-crossterm`。
3. 当前 `punctum-gpu` 保留 atlas、viewport、cell、submission planning、instance encoding 和 uniform encoding，不再依赖 winit 或 wgpu。
4. winit event 转换、wgpu runtime、shader、pipeline、surface 和 device 操作位于 `punctum-wgpu`。
5. 现有行为测试在迁移后继续作为合同，不因移动文件重复编写同类测试。只有新增的 crate 依赖边界先写失败测试，再通过拆分使其变绿。
6. 纯逻辑 crate 保留 line、function 和 region 100% coverage，且 coverage 命令不使用文件名排除。平台 crate 只运行相关合同测试、format、Clippy 和 smoke，不设置 `fail-under`。
7. Punctum workspace、Tetris Terminal example、Terminal golden、GPU logical oracle 和本机 headless smoke 在拆分前后结果一致。

`R2` 结果如下：

1. `punctum-terminal` 不再依赖 Crossterm；`punctum-gpu` 只依赖 `punctum-grid`，不再依赖 `punctum-input`、winit 或 wgpu。
2. Tetris Terminal view 继续使用 `punctum-terminal`，main 改用 `punctum-crossterm`。本阶段没有新增 GPU example。
3. instance/uniform encoding 与现有两个合同测试移入 `punctum-gpu`。Crossterm 与 winit 合同测试随平台代码迁移，没有复制同类测试。
4. pure coverage 命令不再使用 `ignore-filename-regex`。平台 crate 只执行合同测试、Clippy 和 ignored headless smoke。
5. `R2` 没有实现 UI framework，也没有进入 `F3a` 或 `B2a`。

### `F3a`：Terminal text content boundary

`F3a` 是一个原子修复任务。它与 `B2a` 都只依赖已经通过的 `R2`，两者可以并行。

1. `punctum-terminal::write_text` 改为接收 `&str`。
2. 固定标题、业务状态和提示文字直接传字符串。`TextEvent` 只留在键盘、粘贴、IME 和文本编辑输入链路。
3. `punctum-terminal` 删除对 `punctum-input` 的依赖。
4. 空字符串是无操作并返回原位置。现有 grapheme、Unicode width、continuation、裁剪和 resize 行为保持不变。
5. 更新 `punctum-terminal` 合同、受影响的 `punctum-crossterm` 合同和 Tetris Terminal view。
6. 本任务不创建 `punctum-ui`，不实现字体、GPU 文字、公共 `Text` Primitive 或 `TextInput`。

### `B2a`：Tetris GPU implementation

`B2a` 已于 2026-07-12 完成。它只实现 Tetris GPU 可运行入口，没有等待 `F3a`，也没有修改 Terminal 文本实现。

1. 新增 `apps/tetris/examples/gpu/main.rs` 和纯 GPU view/projection 模块。projection 调用现有 `paint`，再把 `TetrisCell` 转为 `GpuCell`。
2. 使用白色单像素 atlas 和 tint 表达边框与七种方块颜色。atlas、resource ID 和颜色属于 Tetris GPU view，不进入业务核心。
3. host 只负责 winit event loop、tick、resize、redraw 和 GPU submission。Terminal/GPU 必须复用 `TetrisState`、`transition`、`paint` 和 `command_for_key`。
4. viewport 使用整数 cell size 并居中。窗口小于棋盘时允许裁剪；resize、minimize 和 cell pixel size 变化不能修改业务状态。
5. 首帧提交完整 surface，后续帧使用 grid diff 和 GPU patch。backend submission 细节不能进入 Tetris core。
6. 为 GPU projection、颜色映射、viewport、resize、game over、restart 和 input chain 增加 pure fixture 或 golden 测试。
7. 本任务不实现 GPU 标题、系统字体、文字 shaping、glyph atlas、布局、焦点或 widget。

实现结果：

1. `apps/tetris/examples/gpu/main.rs` 接通 winit event loop、tick、resize、scale、redraw 和 `punctum-wgpu` submission。
2. `apps/tetris/examples/gpu/view.rs` 调用现有 `paint`，再逐格投影为 `GpuCell`。边框和七种方块共用白色单像素 atlas，通过 tint 区分颜色。
3. viewport 使用最大可用整数 cell size 并居中。小窗口使用一像素 cell 和负 origin 裁剪；minimize、resize 和 scale 不修改 `TetrisState`。
4. 首帧提交完整 `Surface<GpuCell>`。后续帧对上一帧执行 `diff`，并提交 `Patch<GpuCell>`。
5. 键盘事件先由 `punctum-wgpu` 规范化，再复用 `command_for_key` 和 `transition`。tick 复用 `transition(TetrisCommand::Tick)`。
6. 8 项纯逻辑测试覆盖 atlas、颜色、projection、viewport、裁剪、resize 状态不变式、game over、restart、input chain 和 full/diff 选择。
7. Tetris manifest 和 lockfile 已加入 `punctum-gpu`、`punctum-wgpu`、winit、wgpu 和 host-local async executor。最终依赖状态仍由 `B2b` 接收。
8. 本任务没有实现 GPU 文字、系统字体、`punctum-ui`、焦点或 widget。

### `B2b`：composition barrier

`B2b` 只依赖 `F3a` 和 `B2a`。它是唯一汇合任务，不增加功能，不重构两个上游任务的实现。

1. 接受 Tetris 对 `punctum-gpu`、`punctum-wgpu`、winit 和 host-local async executor 的 dev-dependency。
2. 统一刷新最终 manifest、lockfile 和 `packages/arbor-projects` 验证注册。
3. 确认 Terminal/GPU 继续共享同一 Tetris core 和 input contract。
4. 更新 Punctum 与 Tetris baseline，冻结 Terminal 和 GPU export hash，并在 Windows 11 本地分别运行 Terminal 与 GPU smoke。
5. 更新管理文档中的完成状态、残余风险和下一节点。

Tetris 双后端本地 smoke test 通过后，Punctum adapter 才交给 Poke Game 消费。Tetris 不能替代后续 Game E2E。

### `F3b`：stateless provisional UI foundation

`B2b` 通过后启动 `F3b`。先新增 Punctum `PEP 0002`，再由一个 UI foundation writer 实现。PEP 必须冻结以下 provisional 合同：

- 整数 constraints、measure、layout、paint、clip stack、resize、identity、state ownership 和结构化错误。
- 一个 backend-neutral UI 边界。UI core 只产生布局和绘制意图；Terminal/GPU 负责把相同内容投影为各自 cell 或 glyph resource。
- `Text` 接收只读字符串或独立文本模型，不能接收输入侧 `TextEvent`。measure 和 paint 必须消费同一个 backend text-layout 结果。
- `WidgetId` 和 focus scope 使用类型安全 ID。ID 由调用方稳定提供，不能从临时树位置推导。
- framework 只持有 focus、pressed、disabled 等通用 UI 状态。Tetris、Battle 等业务状态仍由各自 application/core 持有。

`F3b` 只实现无状态布局和绘制：

1. 在 Punctum workspace 新增单一 `punctum-ui` crate，内部使用 `primitives` 和 `widgets` 模块，不提前拆 crate。
2. 先实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
3. 用 exact layout fixture、surface golden、零尺寸、空间不足、嵌套 clip 和 resize 测试验证 stateless layout/paint。
4. 使用这些 primitive 重组 Tetris 的标题、棋盘、累计消行数、快捷键提示和 game-over 页面，并让 Terminal/GPU 继续使用各自 projection。
5. GPU 文字先使用确定的字体 fixture 或由 host 提供的字体数据。任意系统字体发现属于后续平台 adapter 能力，不进入 UI core，也不阻塞本阶段。

### `F3c`：provisional interaction foundation

`F3b` 通过后才启动 `F3c`：

1. 实现 focus registry、scope、Tab/Shift+Tab、event consumption 和 `Button`。
2. widget 只返回 action。Tetris restart 仍由 `transition` 执行，UI 不能直接清空棋盘。
3. 没有 Game 的真实需求和 oracle 时，不实现 `ChoiceList`、`List`、`ScrollView` 或其他集合 widget。

### `B3`：串行 barrier

`B3` 是独立 composition task。它接受 Punctum 与 Tetris 的 UI foundation 依赖变化，更新对应 lockfile、baseline 和 export hash。独立 verifier 必须确认：

- `punctum-ui` 不依赖 Terminal、GPU、winit、wgpu、Tetris、Battle 或 Ramus 类型。
- pure layout、paint、focus 和 dispatch 的 line、function 和 region coverage 全部为 100%。
- Tetris Terminal/GPU smoke 均通过，且两端继续共享同一业务核心和 input contract。
- `B3` 通过前不启动 Game UI 实现。`B3` 通过后只把 `F3b/F3c` 的 provisional API 交给 Game 消费，不把它标记为稳定公共合同。

### `F4`：Game 并行 lane 与串行 integration

可以并行实现 game UI library，以及 battle-Ramus、console 和 Agent harness library。两条 lane 通过各自验证后，创建新的原子 composition task 完成 Game composition 和 `game-e2e`。

跨域 E2E 归消费方 workspace：

- `apps/gen3-game/crates/game-e2e/` 验证 Human/Ramus 等价、GPU logical output 和 battle closure。
- Tetris 的 adapter/UI proof 留在 `apps/tetris`，不反向依赖 Battle 或 Game。

### `F5`：只读验证

逐一运行 Punctum、Tetris、Ramus 和 Game 的完整模板，再运行 GPU reference、Game E2E、path canonicalization 和 upstream hash 检查。Chater 只检查暂停状态没有被当前 wave 意外写入。

## 原子任务 DAG

Agent 是执行者，task 是 DAG 节点。每个 task 必须只有一个目标、一个写入范围和一个完成条件。一个 Prompt 不能要求 Agent 等待其他 Agent 后继续，也不能把实现、汇合和下一阶段设计塞进同一任务。

当前 DAG：

```text
              +-- F3a Terminal text boundary --+
R2 completed -+                                +-- B2b composition -- F3b -- F3c -- B3
              +-- B2a Tetris GPU -------------+
```

`F3a` 与 `B2a` 同时启动，互不通信。`B2b` 是唯一汇合节点。执行者可以复用，但必须收到新的独立 Prompt，不能把 `B2b` 预埋在 `F3a` 或 `B2a` Prompt 中。

### `TextBoundary` Prompt

```text
你叫 TextBoundary。

阅读 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-program.md 和 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-architecture-plan.md。执行原子任务 F3a：修复 Terminal 文本输出与输入事件耦合。

把 punctum-terminal::write_text 改为接收 &str，删除 punctum-terminal 对 punctum-input 的依赖。更新 punctum-terminal 合同、受影响的 punctum-crossterm 合同和 apps/tetris/examples/terminal/view.rs。空字符串是无操作；现有 Unicode、continuation、裁剪和 resize 行为不变。

只修改 apps/punctum/crates/punctum-terminal/**、apps/punctum/crates/punctum-crossterm/tests/**、apps/punctum/Cargo.lock 和 apps/tetris/examples/terminal/view.rs。

不要修改 Tetris manifest、Tetris lockfile、GPU example、项目注册或管理文档。不要实现字体、GPU 文字、punctum-ui 或 TextInput。按 TDD 实现，只验证本任务行为。完成后停止。
```

### `TetrisGpu` Prompt

```text
你叫 TetrisGpu。

阅读 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-program.md 和 C:\Users\nyml\code\arbor\workspace\manage\punctum-ramus-architecture-plan.md。执行原子任务 B2a：实现 Tetris GPU 版本。

新增 apps/tetris/examples/gpu 下的 host、纯 projection 和行为测试。复用现有 TetrisState、transition、paint 和 command_for_key。使用白色单像素 atlas 与 tint 绘制棋盘。实现整数 cell size、居中、裁剪、resize、minimize、首帧完整提交和后续 patch。

只修改 apps/tetris/examples/gpu/**、apps/tetris/Cargo.toml、apps/tetris/Cargo.lock，以及 GPU projection 直接需要的 Tetris fixture。

不要修改 Terminal example、Punctum、项目注册或管理文档。不要实现 GPU 文字、系统字体、punctum-ui、焦点或 widget。按 TDD 实现；平台 IO 不追求 coverage 百分比。完成后停止。
```

### `B2b` Prompt

`F3a` 与 `B2a` 都完成后，把以下新任务交给任一空闲 Agent：

```text
执行原子任务 B2b：接收 F3a 和 B2a。

不要增加功能，不重构上游实现。统一最终 manifest 和 lockfile，更新 packages/arbor-projects 验证注册，确认 Terminal/GPU 共享同一 Tetris core 和 input contract，记录双后端本地验收结果，并更新两份管理文档。

同时修正五个 workspace 对应五个 lockfile 和五个验证 manifest。完成后停止。
```

## Agent Staffing

| 阶段 | Writer | Reviewer | 并行规则 |
| --- | --- | --- | --- |
| `S0` | 一个 Program Integration Agent | 一个只读 `verifier` | 串行，不启动其他 writer |
| `F1` | 最多三个 `executor` | `verifier`；Ramus 追加 `security-reviewer` | 三个 lane 可并行，Battle 受规则门禁限制 |
| `B1` | Program Integration Agent | `verifier` | 串行 barrier |
| `PT1` | 一个 Tetris `executor` | `verifier` | 只写 `apps/tetris`；core 不接 IO |
| `F2` | 一个 GPU `executor`；Terminal 已完成 | `verifier` | 主线只补 GPU 输入合同；Ramus 验证器可作为独立后台 lane |
| `R2` | `r2_adapter_split` | 当前任务验证 | 已串行完成 |
| `F3a` | `TextBoundary` | `B2b` 接收 | 与 `B2a` 并行；完成后停止 |
| `B2a` | `TetrisGpu` | `B2b` 接收 | 与 `F3a` 并行；完成后停止 |
| `B2b` | 两个现有 Agent 中任一空闲者接受新 Prompt | 本轮不新增 Agent | 只依赖 `F3a` 与 `B2a`；唯一汇合任务 |
| `F3b` | 一个 UI foundation `executor` | `architect`、`verifier` | 先冻结 provisional `PEP 0002`，只实现无状态布局和绘制 |
| `F3c` | 一个 interaction `executor` | `architect`、`verifier` | 依赖 `F3b`；只实现焦点、事件分发和 `Button` |
| `B3` | 任一空闲 Agent 接受独立 composition Prompt | `verifier` | 只接收 `F3b/F3c`，不增加功能 |
| `F4` | 最多两个 lane writer，随后创建独立 composition task | `verifier`；Ramus 追加 `security-reviewer` | Game UI 与 Agent/Ramus 可并行，composition 是单独 DAG 节点 |
| `F5` | 无 writer | 独立 `verifier`；Ramus 追加 `security-reviewer` | 只读验证 |

每个 writer handoff 必须报告：

- 修改路径和写入所有权。
- 使用的合同版本与 upstream export hash。
- absolute `CARGO_TARGET_DIR`。
- 实际执行的验证命令与 exit code。
- 未通过的 gate、残余风险和 change request。

leader 收到 handoff 后先检查 write scope、baseline 和 upstream hash，再让 verifier 使用新的 task-unique target 独立重跑。barrier 通过前不得启动下一 wave。

## Workspace 验证模板

每组命令使用该 task 独占的 absolute `CARGO_TARGET_DIR`。

```powershell
cargo metadata --locked --manifest-path <manifest> --format-version 1
cargo check --workspace --all-targets --locked --manifest-path <manifest>
cargo fmt --all --manifest-path <manifest> -- --check
cargo clippy --workspace --all-targets --locked --manifest-path <manifest> -- -D warnings
cargo test --workspace --all-targets --locked --manifest-path <manifest>
cargo llvm-cov -p <pure-package> --all-targets --locked --manifest-path <manifest> --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
```

五个 `<manifest>`：

```text
<repo>/apps/punctum/Cargo.toml
<repo>/apps/tetris/Cargo.toml
<repo>/packages/ramus/Cargo.toml
<repo>/apps/gen3-game/Cargo.toml
<repo>/apps/tui-chater/Cargo.toml
```

对每个 pure package 分别执行 `cargo llvm-cov`。只有 workspace 全部由纯逻辑组成时才允许使用 `--workspace`。副作用外壳仍执行 fixture、logical oracle 和 smoke test，不为了覆盖率把 IO 逻辑搬回纯核心。

`R2` 通过后，pure package 的 coverage 命令不得包含 `ignore-filename-regex`。平台 package 不执行 `cargo llvm-cov --fail-under-*`，但仍执行相关 test、Clippy 和 smoke。

lane 验证把最终 test 和 coverage 收窄为 `-p <owned-package>`。wave barrier 和 `F5` 必须执行完整 workspace 模板，并检查 baseline、write scope 和反向依赖。

## 下一次实现 Session 的启动方式

下一位 agent 必须按以下顺序取得上下文：

1. 遵守新 session 实际注入的 `AGENTS.md instructions`。仓库根当前没有持久化的 `AGENTS.md` 文件，不要把该路径当成启动依赖。
2. 读取[项目群总控](./punctum-ramus-program.md)。
3. 读取本架构计划。
`F2` adapter lane、`R2` crate 边界和 `B2a` Tetris GPU 已完成。等待 `F3a` 完成后创建新的 `B2b` 汇合任务。`B2a` 的 manifest 和 lockfile 变化等待 `B2b` 接收；验证注册、双后端基线和最终状态仍由 `B2b` 汇合。

## 当前门禁状态

| Gate | 状态 | 影响 |
| --- | --- | --- |
| `P0 Product Clarified` | 已通过 | 产品范围可作为实现依据 |
| `A0 Architecture Approved` | 已通过 | 架构可作为实现依据 |
| `S0 Workspace Ready` | 已通过 | 四个 workspace 和 baseline 已建立 |
| `Punctum F1 grid/input` | 已通过 | export hash 已由 `B1` 接受 |
| `B1 F1 Accepted` | 已通过 | 可启动 `PT1`，不可跨 wave 启动 adapter |
| `PT1 Headless Tetris` | 已通过 | core、paint、input mapping 和 pure coverage 已验收 |
| `Terminal Unicode Text` | 已通过 | 28 项合同测试覆盖 grapheme、width、continuation、裁剪、resize 和 cursor；共享 `Text` Primitive 仍未实现 |
| `Punctum GPU Local` | 已通过 | pure planner 与输入转换 100%；本机 headless device、shader、pipeline 和 texture smoke 已通过 |
| `F2 Adapter Completion` | 已通过 | winit 输入规范化、8 项输入合同测试和 GPU pure coverage 已通过 |
| `Ramus Python Branch Coverage` | 已通过 | typed LCOV parser 检查逐条 `BRDA`；148/148 branch entries，旧脚本交叉验证通过 |
| `R2 Adapter Crate Boundary` | 已通过 | `punctum-crossterm` 与 `punctum-wgpu` 已隔离平台副作用，纯 coverage 不再排除文件名 |
| `F3a Terminal Text Boundary` | 未开始 | 可以与 `B2a` 并行；修复输出 API 对 `TextEvent` 的依赖 |
| `B2a Tetris GPU Implementation` | 已通过 | GPU 棋盘入口、纯 projection、full/diff submission 和本机 smoke 已完成 |
| `B2b Dual Backend Composition` | 等待 `F3a` | `B2a` 已就绪；待 `F3a` 完成后接收配置并运行双后端 smoke |
| `F3b Stateless UI Foundation` | 未开始 | 等待 `B2b`；先批准 `PEP 0002` |
| `F3c Interaction Foundation` | 未开始 | 等待 `F3b`；只实现焦点、事件分发和 `Button` |
| `BATTLE-RULES-v0.1` | 已批准 | Battle 规则核心与侧别观察合同已由 `B1` 接受 |
| `GPU-REF-v0.1` | 已建立，未通过 | 固定环境 readback 和 release 被阻塞；本地 GPU smoke 不阻塞 |

## 风险与约束

| 风险 | 约束 |
| --- | --- |
| grid/input 过小，产品重复 interaction | 允许短期重复，达到 extraction gate 后再立 ADR |
| Tetris 规则污染共享内核 | 业务类型只留在 `apps/tetris` 的 `punctum-tetris`；稳定公共 API 仍需 Game 实际消费和第二消费者评审 |
| Tetris 两个入口复制规则 | Terminal/GPU host 共享同一 core、paint 和 command mapping |
| Prompt 包含等待和多阶段职责 | task 保持原子；并行结果只由显式 DAG 汇合节点接收 |
| 覆盖率驱动 IO 回流 | `R2` 用 crate 隔离副作用；纯逻辑 100%，平台 crate 不设百分比并使用 fixture、logical oracle 和 smoke |
| path dependency 漂移 | canonical path、export hash、consumer pin 和 barrier 复核 |
| Ramus seal 后撤权失效 | per-effect `EffectPermit`、linearization test、100% capability matrix |
| Battle rule 范围漂移 | `BATTLE-RULES-v0.1` 之外的特性、道具和复杂状态继续留在后续版本 |
| GPU golden 受硬件影响 | 当前本机 hardware 只做 smoke；正式 release 仍等待 pinned llvmpipe identity |
| Ramus LCOV 分支语义漂移 | Python verifier 校验 typed LCOV 和逐条 `BRDA`，branch 不由 line、function 或 region coverage 替代；旧脚本继续作为交叉验证 |
| Agent 争抢 manifest 或 lockfile | 文件由 task 单 owner；所有权只沿 DAG 依赖边传给汇合节点 |
| 五 workspace 依赖版本漂移 | 各自 lockfile 和 export hash；有真实成本后再评估 umbrella workspace |

## 计划变更规则

- 产品事实变化时，先更新项目群总控，再评估本文。
- shared core 扩张必须经过 extraction gate 和新 ADR。
- root manifest、lockfile、path dependency 或 composition ownership 变化必须写入 task 的显式写入范围；跨 lane 接收必须建立独立汇合节点。
- 任何放宽 authorization、Battle fixture 或 GPU release oracle 的变化都需要独立评审。
- 每个 wave 完成后更新总控状态，不在临时聊天中维护唯一事实。
