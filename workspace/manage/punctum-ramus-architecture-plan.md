# Punctum / Ramus 第一期架构计划

- 状态：已批准
- 批准日期：2026-07-11
- 实现状态：`S0`、三个 `F1` lane、`B1` 与 `PT1` 已完成；Terminal Unicode 文本和 GPU 本地 planner/runtime/smoke 已通过；`F2` 仍缺 winit 输入适配，Tetris GPU 入口留待 `B2`；上层组件候选尚未进入实现 wave
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

上层 UI 候选不得插入当前 `F2` adapter lane。`B2` 完成 Terminal/GPU 基础验证后，才允许建立新的 ADR 和实现 task。

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

当前不创建 `punctum-scene` crate。`F2/B2` 只冻结 grid 到 GPU viewport 的转换边界，并用 Tetris 验证修改 cell pixel size 不改变业务状态。完整 `Camera`、`TileLayer` 和 `SpriteLayer` 在宝可梦地图垂直切片前实现；出现稳定概念后再决定留在 Game 还是提取为 Punctum scene crate。

## Workspace 拓扑

第一期当前使用五个独立 Cargo workspace。不在 Arbor 根创建 Cargo workspace，也不创建 umbrella workspace。

| Workspace | Root manifest | 计划成员 | Lockfile |
| --- | --- | --- | --- |
| Punctum | `apps/punctum/Cargo.toml` | `punctum-grid`、`punctum-input`、`punctum-terminal`、`punctum-gpu` | `apps/punctum/Cargo.lock` |
| Tetris | `apps/tetris/Cargo.toml` | `punctum-tetris` | `apps/tetris/Cargo.lock` |
| Ramus | `packages/ramus/Cargo.toml` | `ramus-core` | `packages/ramus/Cargo.lock` |
| Game | `apps/gen3-game/Cargo.toml` | battle、UI、Ramus adapter、host、E2E crates | `apps/gen3-game/Cargo.lock` |
| Chater | `apps/tui-chater/Cargo.toml` | 暂停；只保留既有 workspace 空壳和历史 lockfile | `apps/tui-chater/Cargo.lock` |

五 workspace 方案用于隔离项目所有权。当前没有足够证据支持根 workspace 或专用 umbrella workspace。只有实际出现不可接受的 dependency、lockfile 或验证成本时，才能通过新 ADR 重新评估。

Tetris 项目固定为 `apps/tetris`，package name 为 `punctum-tetris`。它拥有独立 manifest 和 lockfile，通过 canonical path dependency 使用 Punctum，只依赖 `punctum-grid` 和 `punctum-input` 作为正常依赖。

两个副作用入口归 Tetris 项目所有，不放进 adapter crate：

- `apps/tetris/examples/terminal/main.rs`。
- `apps/tetris/examples/gpu/main.rs`。

`punctum-tetris` 的正常依赖仍只有 grid/input。可执行 example 通过 dev-dependency 消费 adapter。adapter crate 不依赖 Tetris 业务代码。Program Integration Agent 在 barrier 接受 canonical path dependency、Tetris dev-dependency 和 lockfile 变化。

2026-07-12 用户决定把 Tetris 从 Punctum 内部 example package 迁为 `apps/tetris` 独立项目。本决定覆盖后续拓扑和写入所有权，但不改写 `S0/B1` 对迁移前四 workspace 状态的历史记录。

## 写入所有权

设置唯一的 Program Integration Agent。它是 leader 或一个现有 `executor` 单人承担的任务身份，不是新增 agent role。

Program Integration Agent 独占：

- 五个 workspace root manifest。
- member list 和 `[workspace.dependencies]`。
- 四个 `Cargo.lock`。
- canonical path dependency。
- Game 和 Chater composition root。
- Game 和 Chater 跨域 E2E 接线。

lane writer 只能修改自己 crate 的 `Cargo.toml`、`src`、`tests` 和 fixtures。dependency 或 path 变化必须提交 change request，由 Program Integration Agent 在 barrier 串行接受。

两个 writer 不得同时修改同一文件、root manifest、lockfile 或 composition root。

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

- raw Terminal event 转换为 `punctum-input`。
- `Surface<TerminalCell>` 和 `Patch` 转换为 ANSI 输出。
- Unicode width、continuation、cursor 和 terminal capability 留在 adapter。
- adapter 不持有 chat state。

Terminal adapter 已实现 grapheme `TerminalCell`、Unicode width、continuation、覆盖任一宽字符槽时清理配对槽、整 grapheme 裁剪、resize 后孤立槽清理、patch planner、Crossterm key normalization、presenter、raw-mode session 和 cursor 停放。`cargo test -p punctum-terminal` 的 28 项合同测试已于 2026-07-12 通过。

### GPU adapter

- window keyboard event 转换为 `punctum-input`。
- `Surface<SpriteCell>` 和 `Patch` 转换为 resource lookup 与 GPU submission。
- atlas、texture、alpha、shader、viewport 和 GPU resource 留在 adapter。
- adapter 不持有 game state。

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
game-host -> punctum-gpu -> punctum-grid + punctum-input
game-ui -> punctum-grid + punctum-input

battle-agent -> battle-ramus-adapter -> battle-application
game-console -> battle-ramus-adapter -> ramus-core
battle-ramus-adapter -> ramus-core
ramus-core -X-> battle-domain

# paused: tui-host -> chater-ui -> chat-application -> model-port
# paused: tui-host -> punctum-terminal -> punctum-grid + punctum-input
# paused: chater-ui -> punctum-grid + punctum-input

tetris-terminal-host -> punctum-terminal -> punctum-grid + punctum-input
tetris-gpu-host -> punctum-gpu -> punctum-grid + punctum-input
tetris-terminal-host + tetris-gpu-host -> tetris-core -> punctum-grid + punctum-input

punctum-grid/input -X-> game / chat / Ramus / Crossterm / wgpu
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

Punctum 的非副作用代码必须使用 TDD。每项行为先取得失败测试，再实现最小通过代码。纯逻辑使用 stable toolchain 下的 `cargo llvm-cov`，line、function 和 region coverage 必须全部达到 100%。实验性 branch coverage 只有在已批准的 nightly toolchain 可用时才作为附加证据，不能用它替代 stable coverage 门槛。

Terminal/GPU host 的事件循环、系统 IO、窗口和 GPU submission 不要求源码覆盖率 100%。这些副作用必须保持薄，并通过 pure planner、fixture、in-memory backend、logical oracle 和本地 smoke test 验证。能从 IO 中拆出的转换、布局、资源解析和提交规划都属于非副作用代码，仍要求 100% coverage。

第一期先在 Windows 11 本地跑通。Terminal 人工验证使用 Windows Terminal；GPU 人工验证使用本机可用的 wgpu adapter。当前不建设 CI。`GPU-REF-v0.1`、llvmpipe readback 和正式 release oracle 保持 `Blocked`，但不阻塞本地 GPU adapter、Tetris GPU 入口和 smoke test。Ollama 和 DeepSeek live smoke test 不作为 deterministic oracle。

## 2026-07-12 本地验证记录

本期项目的本地验证已统一进入标准库 Python 工程 `packages/arbor-projects`。入口为 `python packages/arbor-projects/run.py verify <project-id>`。注册表包含 `punctum`、`tetris`、`ramus`、`gen3-game` 和 `tui-chater`。

Python 工程使用不可变 dataclass 领域模型和 typing.Protocol 端口。文件、进程和 LLVM 工具调用只位于 adapter。生产函数体内不得出现字面常量；AST 门禁负责检查该约束。

本地结果：

- Python 单元测试 28 项全部通过。
- Python 纯逻辑覆盖率：domain 133/133、application 89/89、JSON registry parser 102/102、LLVM export parser 85/85。
- Punctum 的 format、workspace test、Clippy 和四个纯 crate 覆盖率门禁全部通过。
- punctum-grid 为 450/450 regions、40/40 functions、325/325 lines。
- punctum-input 为 19/19 regions、3/3 functions、14/14 lines。
- punctum-terminal 为 457/457 regions、30/30 functions、347/347 lines。
- punctum-gpu pure planner 为 402/402 regions、30/30 functions、302/302 lines。
- Punctum GPU headless smoke 在本机 adapter 上通过。该测试实际创建 device、shader、pipeline 和 atlas texture。测试发现并修复 WGSL 保留字 `target` 导致的 shader parse failure。
- Tetris 的 format、29 项测试、Clippy、core 覆盖率和 Terminal view 单文件覆盖率全部通过。core 为 328/328 regions、35/35 functions、257/257 lines；Terminal view 为 150/150 regions、11/11 functions、105/105 lines。
- Ramus、Gen3 Game 和 TUI Chater 的 Python 注册门禁全部通过。因 Punctum path dependency 变化，Punctum、Tetris、Gen3 Game 和 TUI Chater lockfile 已刷新，并通过 `--locked` 复核。

仍有两个限制：

- `GPU-REF-v0.1` 固定环境 readback 尚未执行。本机 headless smoke 不能替代固定 adapter identity 和像素 readback。
- Ramus 的 Python 注册门禁当前只包含 format、test 和 Clippy。原 nightly branch coverage 规则依赖 LCOV 分支解析，尚未迁入 Python verifier。后续必须新增 typed LCOV adapter 后再移除旧门禁，不能用 line/function/region 阈值替代 branch 合同。

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
- GPU adapter。本地 logical planner、wgpu runtime 和 headless pipeline smoke 已通过。剩余实现只包含 winit 键盘事件规范化和对应合同测试。
- Chater lane 已取消，不再占用 `F2` writer。

GPU adapter 的本地 logical oracle 和 smoke test 不受 `GPU-REF-v0.1` 阻塞。只有固定环境 readback 与 release gate 继续 `Blocked`。

`F2` 剩余 task packet 固定为：

1. 在 `punctum-gpu` 内把 winit keyboard event 转为 `punctum-input::KeyEvent`。
2. 精确映射 physical key、logical key、modifier 和 press/repeat/release。未知键使用 `Unidentified`，不能伪造来源没有提供的身份。
3. 使用可构造的 raw key fixture 验证方向键、Space、physical `KeyR`、字符、modifier、repeat、release 和未知键。
4. 本阶段不实现 IME composition、`TextInput` 或文本编辑，也不把 Tetris 类型放入 adapter。
5. handoff 前运行 `punctum-gpu` 的 format、test、Clippy、pure coverage 和 ignored headless smoke。纯 planner 继续保持 line、function 和 region 100% coverage。

### `B2`：串行 barrier

`F2` handoff 通过后，只有 Program Integration Agent 可以写 root manifest、lockfile 和 Tetris GPU composition。它执行以下动作：

1. 接受 Tetris 对 `punctum-gpu`、winit 和 host-local async executor 的 dev-dependency，刷新相关 lockfile。
2. 新增 `apps/tetris/examples/gpu/main.rs` 和纯 GPU view/projection 模块。projection 调用现有 `paint`，再把 `TetrisCell` 转为 `GpuCell`。
3. 使用白色单像素 atlas 和 tint 表达边框与七种方块颜色。atlas、resource ID 和颜色属于 Tetris GPU view，不进入业务核心。
4. host 只负责 winit event loop、tick、resize、redraw 和 GPU submission。Terminal/GPU 必须复用 `TetrisState`、`transition`、`paint` 和 `command_for_key`。
5. viewport 使用整数 cell size 并居中。窗口小于棋盘时允许裁剪；resize、minimize 和 cell pixel size 变化不能修改业务状态。
6. 首帧提交完整 surface，后续帧使用 grid diff 和 GPU patch。backend submission 细节不能进入 Tetris core。
7. 为 GPU projection、颜色映射、viewport、resize、game over、restart 和 input chain 增加 pure fixture 或 golden 测试。
8. 更新 Punctum 与 Tetris baseline，冻结 Terminal 和 GPU export hash，并在 Windows 11 本地分别运行 Terminal 与 GPU smoke。

Tetris 双后端本地 smoke test 通过后，Punctum adapter 才交给 Poke Game 消费。Tetris 不能替代后续 Game E2E。

### `F3`：provisional UI foundation

`B2` 通过后才能启动 `F3`。先新增 Punctum `PEP 0002`，再由一个 UI foundation writer 实现。PEP 必须冻结以下 provisional 合同：

- 整数 constraints、measure、layout、paint、clip stack、resize、identity、state ownership 和结构化错误。
- 一个 backend-neutral UI 边界。UI core 只产生布局和绘制意图；Terminal/GPU 负责把相同内容投影为各自 cell 或 glyph resource。
- `Text` 接收只读字符串或独立文本模型，不能接收输入侧 `TextEvent`。measure 和 paint 必须消费同一个 backend text-layout 结果。
- `WidgetId` 和 focus scope 使用类型安全 ID。ID 由调用方稳定提供，不能从临时树位置推导。
- framework 只持有 focus、pressed、disabled 等通用 UI 状态。Tetris、Battle 等业务状态仍由各自 application/core 持有。

实现顺序固定为：

1. 在 Punctum workspace 新增单一 `punctum-ui` crate，内部使用 `primitives` 和 `widgets` 模块，不提前拆 crate。
2. 先实现 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。
3. 用 exact layout fixture、surface golden、零尺寸、空间不足、嵌套 clip 和 resize 测试验证 stateless layout/paint。
4. 再实现 focus registry、scope、Tab/Shift+Tab、event consumption、`Button` 和 `ChoiceList`。
5. widget 只返回 action。Tetris restart 仍由 `transition` 执行，UI 不能直接清空棋盘。
6. 使用这些 primitive 重组 Tetris 的标题、棋盘、累计消行数、快捷键提示和 game-over 页面，并让 Terminal/GPU 继续使用各自 projection。

### `B3`：串行 barrier

Program Integration Agent 接受 Punctum 与 Tetris 的 UI foundation 依赖变化，更新对应 lockfile、baseline 和 export hash。独立 verifier 必须确认：

- `punctum-ui` 不依赖 Terminal、GPU、winit、wgpu、Tetris、Battle 或 Ramus 类型。
- pure layout、paint、focus 和 dispatch 的 line、function 和 region coverage 全部为 100%。
- Tetris Terminal/GPU smoke 均通过，且两端继续共享同一业务核心和 input contract。
- `B3` 通过前不启动 Game UI 实现。`B3` 通过后只把 provisional API 交给 Game 消费，不把它标记为稳定公共合同。

### `F4`：Game 并行 lane 与串行 integration

可以并行实现 game UI library，以及 battle-Ramus、console 和 Agent harness library。两条 lane 通过各自验证后，唯一 Program Integration Agent 完成 Game composition 和 `game-e2e`。

跨域 E2E 归消费方 workspace：

- `apps/gen3-game/crates/game-e2e/` 验证 Human/Ramus 等价、GPU logical output 和 battle closure。
- Tetris 的 adapter/UI proof 留在 `apps/tetris`，不反向依赖 Battle 或 Game。

### `F5`：只读验证

逐一运行 Punctum、Tetris、Ramus 和 Game 的完整模板，再运行 GPU reference、Game E2E、path canonicalization 和 upstream hash 检查。Chater 只检查暂停状态没有被当前 wave 意外写入。

## Agent Staffing

| 阶段 | Writer | Reviewer | 并行规则 |
| --- | --- | --- | --- |
| `S0` | 一个 Program Integration Agent | 一个只读 `verifier` | 串行，不启动其他 writer |
| `F1` | 最多三个 `executor` | `verifier`；Ramus 追加 `security-reviewer` | 三个 lane 可并行，Battle 受规则门禁限制 |
| `B1` | Program Integration Agent | `verifier` | 串行 barrier |
| `PT1` | 一个 Tetris `executor` | `verifier` | 只写 `apps/tetris`；core 不接 IO |
| `F2` | 一个 GPU `executor`；Terminal 已完成 | `verifier` | 主线只补 GPU 输入合同；Ramus 验证器可作为独立后台 lane |
| `B2` | Program Integration Agent | `verifier` | 串行 barrier |
| `F3` | 一个 UI foundation `executor` | `architect`、`verifier` | 先冻结 provisional `PEP 0002`，再实现布局、焦点和 widget |
| `B3` | Program Integration Agent | `verifier` | 串行 barrier |
| `F4` | 最多两个 lane writer，随后 Program Integration Agent | `verifier`；Ramus 追加 `security-reviewer` | Game UI 与 Agent/Ramus 可并行，composition 串行 |
| `F5` | 无 writer | 独立 `verifier`；Ramus 追加 `security-reviewer` | 只读验证 |

### 四席并行安排

四个席位按下表使用。空闲席位不能通过跨 wave 写代码来提高表面并行度。

| 阶段 | 席位 1 | 席位 2 | 席位 3 | 席位 4 |
| --- | --- | --- | --- | --- |
| `F2` | GPU adapter writer | Ramus typed LCOV/branch verifier writer，非关键路径 | Punctum 只读 verifier | Integration lead，只准备 `B2` task packet |
| `B2` | Program Integration Agent，唯一主线 writer | handoff 后独立 verifier | GPU 只读 reviewer | Ramus verifier 可继续 |
| `F3` | UI foundation writer，唯一 UI writer | Ramus verifier 可继续 | `architect` 只读评审 PEP/API | 独立 verifier |
| `B3` | Program Integration Agent，唯一主线 writer | handoff 后独立 verifier | dependency 只读 reviewer | 准备 `F4` task packet，不实现 `F4` |

Ramus verifier lane 只能修改 `packages/arbor-projects` 中的 typed LCOV/branch coverage 能力和对应测试。它不能修改 Punctum、Tetris、root manifest 或 lockfile，也不构成 `B2/B3` 的产品完成条件。

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

四个 `<manifest>`：

```text
<repo>/apps/punctum/Cargo.toml
<repo>/packages/ramus/Cargo.toml
<repo>/apps/gen3-game/Cargo.toml
<repo>/apps/tui-chater/Cargo.toml
```

对每个 pure package 分别执行 `cargo llvm-cov`。只有 workspace 全部由纯逻辑组成时才允许使用 `--workspace`。副作用外壳仍执行 fixture、logical oracle 和 smoke test，不为了覆盖率把 IO 逻辑搬回纯核心。

lane 验证把最终 test 和 coverage 收窄为 `-p <owned-package>`。wave barrier 和 `F5` 必须执行完整 workspace 模板，并检查 baseline、write scope 和反向依赖。

## 下一次实现 Session 的启动方式

下一位 agent 必须按以下顺序取得上下文：

1. 遵守新 session 实际注入的 `AGENTS.md instructions`。仓库根当前没有持久化的 `AGENTS.md` 文件，不要把该路径当成启动依赖。
2. 读取[项目群总控](./punctum-ramus-program.md)。
3. 读取本架构计划。
4. 读取[Punctum PEP 0001](../../apps/punctum/peps/0001-punctum-technical-direction.md)，只作为次级来源；冲突时以前两份文档为准。

下一轮继续 `F2`。Terminal Unicode 文本、GPU logical planner、wgpu runtime 和本机 headless smoke 已通过，不再派发这些已完成能力。主线 writer 只补 winit 键盘事件规范化和合同测试；`F2` handoff 通过后，再由 Program Integration Agent 在 `B2` 接通 Tetris GPU 入口。

可在新的 Codex session 中直接发送：

```text
继续实现 Punctum / Ramus 项目群的第一期。

遵守当前 session 注入的 AGENTS.md instructions。先读取 workspace/manage/punctum-ramus-program.md 和 workspace/manage/punctum-ramus-architecture-plan.md。产品事实以总控文档为准，架构、所有权、门禁和 wave 以架构计划为准。只关注 Punctum、Tetris、Ramus、gen3-game 和游戏控制台；tui-chater 当前暂停。

`B1` 与 `PT1` 已通过。Tetris Terminal example 已可玩，Terminal Unicode 文本的 28 项合同测试已通过，GPU logical planner、wgpu runtime 和本机 headless smoke 已通过。业务代码位于 `apps/tetris` 独立 workspace。

现在继续 `F2`：只在 `punctum-gpu` 内按 TDD 完成 winit 键盘事件规范化和合同测试，不实现 Tetris GPU host，不修改 root manifest 或 lockfile。保持逻辑 grid、scene viewport 和像素投影分层，不要把产品类型移入 Punctum 共享内核，也不要建立 Arbor 根 Cargo workspace。

完成后独立验证 owned package、write scope、pure coverage、input fixture、adapter logical oracle 与本地 smoke。handoff 通过后停止，不跨 wave；由 Program Integration Agent 在 `B2` 接受 Tetris GPU dev-dependency 和运行入口。GPU 固定环境 readback 继续受 `GPU-REF-v0.1` gate 约束。
```

当前处于 `F2` adapter lane。Terminal/GPU adapter 完成后，由 Program Integration Agent 在 `B2` 接受两个本地 Tetris 入口。不得一次跨 wave 派发。

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
| `Punctum GPU Local` | 已通过 | pure planner 100%；本机 headless device、shader、pipeline 和 texture smoke 已通过 |
| `F2 Adapter Completion` | 进行中 | 等待 winit 键盘事件规范化、合同测试和独立 handoff |
| `B2 Dual Backend Tetris` | 未开始 | 等待 `F2` handoff；随后串行接通 GPU 入口并运行双后端 smoke |
| `BATTLE-RULES-v0.1` | 已批准 | Battle 规则核心与侧别观察合同已由 `B1` 接受 |
| `GPU-REF-v0.1` | 已建立，未通过 | 固定环境 readback 和 release 被阻塞；本地 GPU smoke 不阻塞 |

## 风险与约束

| 风险 | 约束 |
| --- | --- |
| grid/input 过小，产品重复 interaction | 允许短期重复，达到 extraction gate 后再立 ADR |
| Tetris 规则污染共享内核 | 业务类型只留在 `apps/tetris` 的 `punctum-tetris`；稳定公共 API 仍需 Game 实际消费和第二消费者评审 |
| Tetris 两个入口复制规则 | Terminal/GPU host 共享同一 core、paint 和 command mapping |
| 覆盖率驱动 IO 回流 | 纯逻辑 100%；IO 外壳用 fixture、logical oracle 和本地 smoke |
| path dependency 漂移 | canonical path、export hash、consumer pin 和 barrier 复核 |
| Ramus seal 后撤权失效 | per-effect `EffectPermit`、linearization test、100% capability matrix |
| Battle rule 范围漂移 | `BATTLE-RULES-v0.1` 之外的特性、道具和复杂状态继续留在后续版本 |
| GPU golden 受硬件影响 | 当前本机 hardware 只做 smoke；正式 release 仍等待 pinned llvmpipe identity |
| Ramus branch 门禁未进入 Python verifier | 保留原 branch 合同；新增 typed LCOV adapter 后再统一入口，不用较弱阈值替代 |
| Agent 争抢 manifest 或 lockfile | Program Integration Agent 单 owner，barrier 串行接受 |
| 五 workspace 依赖版本漂移 | 各自 lockfile 和 export hash；有真实成本后再评估 umbrella workspace |

## 计划变更规则

- 产品事实变化时，先更新项目群总控，再评估本文。
- shared core 扩张必须经过 extraction gate 和新 ADR。
- root manifest、lockfile、path dependency 或 composition ownership 变化必须由 Program Integration Agent 接受。
- 任何放宽 authorization、Battle fixture 或 GPU release oracle 的变化都需要独立评审。
- 每个 wave 完成后更新总控状态，不在临时聊天中维护唯一事实。
