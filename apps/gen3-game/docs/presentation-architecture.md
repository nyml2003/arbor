# Gen3 游戏呈现架构现状与迭代约束

状态：当前迭代依据

更新日期：2026-07-14

适用项目：`apps/gen3-game`

状态、副作用和迁移步骤见 [状态与副作用隔离迁移方案](state-and-effects-migration-plan.md)。

## 结论

当前代码不是从领域核心开始整体腐化。主要问题集中在呈现边界。

`battle-domain`、`battle-application` 和 `battle-session` 已经形成清楚的数据链路。战斗规则、玩家视角、语义事件、回放 Scene 和交互阶段都有明确所有者。这些边界应保留。

当前需要处理的是下面三件事：

1. GPU 绘制权限散落在 `game-ui`、`map-render`、`map-editor`、`game-host` 和资源加载代码中。
2. 动画和动效由 host 中多组 `Instant`、deadline 和布尔状态共同推进。
3. `game-ui` 同时承担输入、菜单状态、布局、资源编号、atlas 构建、地图合成和动画映射。

UI 抽象可以开始，但应先抽已经重复出现的 target 能力：

- 一帧画面的组合与提交。
- 文字、图片、矩形和 grid canvas 的目标侧绘制计划。
- 单一 GPU frame ownership。
- 确定性的呈现时间推进。

当前不应先做通用组件库、剧情 DSL、任意 tween 系统或通用效果插件。`punctum-ui` 也不应直接成为 Gen3 Native GUI 的既定基础。PEP 7 已把它标记为 provisional；当前代码还没有证明 `GridSize` 布局就是 Native GUI 的长期合同。

## 审查范围

本次审查覆盖：

- `game-ui`
- `game-host`
- `battle-session`
- `world-application`
- `map-render`
- `map-editor` 的 view、text 和 host 提交路径
- Punctum 当前 UI、GPU 和 wgpu 边界

本次不重新设计天气规则、特性、道具、剧情数据格式或地图事件模型。天气和剧情只用于检查当前边界是否会阻碍后续迭代。

## 当前数据链路

### 战斗

```text
battle-domain
-> battle-application: BattleTransition
-> battle-session: BattleScene + BattleCue + BattleInteraction
-> game-ui: GameView
-> game-host: GPU plan + glyphon text
-> punctum-wgpu: present
```

这条链路在 `battle-session` 之前是稳定的。问题从 `game-ui` 开始：`GameView` 已经包含 `Surface<GpuCell>`、`Vec<GpuImage>` 和带 `Rgba8` 的文字标签。

### 世界

```text
world-domain
-> world-application: WorldObservation
-> game-host::DemoGame
-> game-ui: 玩家图片和占位背景
-> map-render: MapScenePlan
-> game-host: 修改 GameView，替换背景并移动图片
-> GPU plan
```

地图和玩家不是由同一个场景组合器生成。host 先取得 `GameView`，再调用 `replace_world_background` 修改已经生成的画面。

### 地图编辑器

```text
EditorModel
-> map-render: MapScenePlan
-> map-editor::view: EditorFrame
-> map-editor::text: glyphon text
-> punctum-wgpu: present
```

`EditorFrame` 与 `GameView` 具有相同的基本结构。`map-editor::text` 与 `game-host::text` 也基本重复。这已经满足“第二个真实实现”的最低证据门槛。

## 已有稳定基础

### 战斗 session 边界应保留

`battle-session` 已经提供：

- 完整的 `BattleScene`。
- 语义化的 `BattleCue`。
- 显式的 `BattleInteraction`。
- 顺序回放步骤。
- 回放最终 Scene 与 transition.after 一致的检查。
- 回放期间输入锁定。

天气、异常状态或剧情触发如果属于战斗语义，应先通过 domain、application 和 session 产生语义状态或事件。它们不能直接产生 GPU 资源、颜色、中文文案或毫秒数。

### 世界规则与地图项目已经分离

`world-domain` 不依赖 GPU。`MapProject` 把视觉、碰撞和事件分层。这个方向正确。

`map-render` 当前是 GPU target planner，不是领域层。它依赖 `punctum-gpu` 本身不是问题，但它的输出不应继续被 feature UI 和 host 任意修改。

### Punctum 已经提供底层能力

当前可继续使用：

- `punctum-grid` 的离散 grid 和 `Surface`。
- `punctum-input` 的规范化输入。
- `punctum-gpu` 的 atlas、grid image 和 submission plan。
- `punctum-wgpu` 的 surface 生命周期和提交。

缺失的是 Gen3 当前真正需要的 Native GUI frame、文字/图片组合和单一提交边界。不要把这些缺口继续塞进 `GpuCell`。

## 主要问题

### 1. GPU 权限没有明确所有者

当前以下模块都直接接触 GPU 类型：

- `game-ui` 创建 `GpuCell`、`GpuImage`、`ResourceId`、`Rgba8` 和 `GpuAtlas`。
- `map-render` 创建地图 `GpuImage`。
- `map-editor::view` 创建并公开 `EditorFrame` 的 GPU 字段。
- `game-host` 直接调用 `plan_composite`。
- `game-host::text` 和 `map-editor::text` 直接编码 wgpu render pass。
- `game-assets` 直接构建 `GpuAtlas`。

结果是每个功能都可以通过“再 push 一个 `GpuImage`”实现视觉效果。这个做法会绕过统一的层级、clip、资源所有权、生命周期和测试合同。

天气常驻层、镜头遮罩、剧情角色、对话框和转场加入后，这个问题会快速放大。

### 2. host 已经成为隐式时间轴

`CreatureGameApp` 当前保存：

- `next_playback`
- `next_sprite_frame`
- `next_world_tick`
- `turn_hold_ends`
- `run_stop_ends`

`about_to_wait` 手工检查每个 deadline，再计算最早唤醒时间。

这种结构只能支撑少量全局唯一效果。每增加一种效果，都需要：

- 新增一个字段。
- 新增一段到期判断。
- 新增重绘规则。
- 修改 earliest deadline 列表。
- 决定与 Console、Battle、World 模式的暂停关系。

当前暂停语义已经不一致。Console 打开后 `about_to_wait` 直接返回，但动画仍使用真实 `Instant` 计算进度。关闭 Console 后，过期的世界移动或战斗回放会立即跳到后续状态。代码既不是明确暂停，也不是明确继续播放。

### 3. `game-ui` 的职责已经过载

`game-ui/src/lib.rs` 当前同时包含：

- 内嵌 PNG 和资源编号区间。
- atlas 构建。
- 战斗菜单状态和键盘处理。
- 世界按键到命令的映射。
- 战斗、队伍、世界和 Console 的投影。
- 颜色、布局和绘制 helper。
- 战斗 Cue 到动画枚举的映射。
- 世界精灵帧选择。
- 大量跨功能测试 fixture。

文件长度已经超过 1700 行。更重要的问题不是行数，而是修改任意一种表现能力都需要理解全部职责。

先按现有职责拆模块是低风险动作。不要先把这些内容塞进一个通用 Widget tree。

### 4. 已经出现三种相似但不兼容的 frame

当前存在：

- `game_ui::GameView`
- `map_render::MapScenePlan`
- `map_editor::view::EditorFrame`

它们都在表达“本帧要画什么”，但字段可见性、viewport 所有权、文字类型和组合方式不同。

`GameView` 通过方法暴露只读 GPU 数据，但提供专门的地图替换方法。`EditorFrame` 直接公开所有字段。`MapScenePlan` 只负责地图图片。没有统一位置负责完整画面的 layer order 和 clip。

### 5. 场景组合依赖事后修改

世界画面先由 `game-ui` 生成玩家图片，再由 host 投影地图，最后通过 `replace_world_background`：

- 替换 surface。
- 修改已有图片坐标。
- 把地图图片插到图片列表前面。

Console 也通过 `overlay_command_console(&mut GameView, ...)` 修改完成后的画面。

这种模式没有显式 layer 合同。后续天气覆盖、光照、转场和剧情遮罩只能继续依赖调用顺序。

### 6. 输入存在两条路径

真实 host 会先拦截世界方向键并直接调用 `step_world`。`DemoGame::handle_key` 又通过 `game_ui::world_command_for_key` 提供另一条世界输入路径。

战斗输入由 `BattleUiState` 解释，Console 输入由 host 和 `GameConsole` 解释。当前输入优先级主要由 `CreatureGameApp::handle_key` 的 if 顺序决定。

剧情锁定、对话框和天气演出加入后，如果继续增加分支，同一个按键会更难判断由谁消费。

### 7. 资源身份与 GPU 编号混在一起

`game-ui` 使用手工起始编号分配角色、战斗精灵、队伍图标、属性图标和招式分类图标。`game-host::map` 从 1000 开始分配地图资源。地图编辑器使用自己的范围和 `u32::MAX` overlay。

当前没有碰撞，是因为各调用方人为避开了范围。天气贴图、剧情角色和 UI 图标继续加入后，资源编号会成为跨模块隐式协议。

稳定的产品资源身份应与 atlas 内部 `ResourceId` 分开。GPU 编号只由资源/target adapter 分配。

## 目标边界

目标不是让业务代码完全不知道“画面”。目标是让只有一个 target 边界能把画面意图转换成 GPU 命令。

```text
Domain / Application
        |
        v
Product Session Snapshot
        |
        v
Presentation State
        |
        v
Product View / Scene Layers
        |
        v
Native GPU Target Planner
        |
        v
Single Frame Submission
```

### Domain / Application

负责规则、状态和语义事件。不依赖 UI、时间、GPU、文字和素材。

### Product Session

负责用户流程和可观察快照。`battle-session` 已经符合这个方向。世界与场景切换后续也应有同等清楚的 session 所有者。

### Presentation State

负责当前正在播放的表现状态：

- 战斗 Cue 的停留和完成。
- 世界移动的进度。
- 精灵循环帧。
- 转向和跑步收尾。
- 明确的暂停策略。

它读取单调时间并输出可测试的表现快照。它不创建 GPU 对象。

### Product View / Scene Layers

负责产品画面结构、文字内容、稳定资源键和 layer 意图。Battle、World、Console 和 Editor 可以保留各自 presenter，不要求共享同一组件树。

### Native GPU Target Planner

这是产品内唯一允许把 view 转成 `GpuCell`、`GpuImage`、glyphon buffer 和 submission plan 的边界。

`map-render` 可以作为 canvas/map planner 存在，但最终完整 frame 应由同一个 target composer 组合。feature presenter 和 host 不再修改 GPU 列表。

### Single Frame Submission

host 负责：

- window 和 surface 生命周期。
- 输入规范化。
- 调用 application/session/presentation。
- 根据一个 next deadline 调度唤醒。
- 提交一份已完成的 frame plan。

host 不决定动画帧、layer 顺序、资源编号或文字布局规则。

## GPU 权限规则

| 模块 | 允许能力 | 禁止能力 |
| --- | --- | --- |
| domain / application / session | 语义状态、事件、action | `punctum-gpu`、wgpu、资源编号、颜色、时长 |
| feature presenter | 产品 view、布局意图、稳定资源键 | wgpu、atlas 编号、surface present |
| presentation state | 时间进度、播放队列、表现快照 | GPU draw command、资源加载 |
| target planner | GPU draw plan、clip、layer、文字/图片转换 | 业务规则、输入决策 |
| resource adapter | 素材解码、稳定键到 GPU 资源映射 | 战斗和剧情状态转换 |
| host adapter | window、surface、deadline、单次提交 | feature 绘制分支、独立 HP/天气/剧情状态 |

近期可以用依赖检查保护这条边界。先限制 `wgpu`、`glyphon` 和 `punctum-wgpu`，再逐步限制 `punctum-gpu`。不要一次搬完所有类型后再验证。

## UI 抽象现在应做到哪一层

### 现在可以抽

#### 完整 frame 合同

Game 与 Map Editor 已经重复实现：

- surface/grid canvas。
- 图片列表。
- 文字标签。
- viewport。
- glyphon prepare/render。
- `present_plan_with_overlay`。

可以先在 Gen3 产品边界内合并这条 target 路径。第一步不要求发布为 Punctum 公共 API。

#### 文字 target

`game-host::text` 和 `map-editor::text` 只有标签类型、字号和错误文案等少量差异。应先收口相同的 glyphon 生命周期和 wgpu encode 逻辑，把字号和样式留给产品配置。

#### Layer 与 clip 合同

当前已有地图、角色、HUD、Console 和帮助浮层。它们需要稳定的 layer 顺序和 clip，而不是依赖 Vec push 顺序。

#### 小型绘制原语

现有重复已经证明需要矩形、图片、文字和 grid canvas。可以为 Gen3 Native target 建立这四种明确 primitive。不要把它们合并成万能 Cell。

### 现在不应抽

- 通用 Button、Menu、Dialog、Form 组件库。
- GUI/TUI 共用的最终 tree。
- 任意 tween/easing DSL。
- 任意剧情脚本语言。
- 任意效果插件和注册顺序。
- 自动反射业务 enum 的资源系统。
- 为每种 layer 单独创建 crate。

Battle 菜单和 Editor 工具栏虽然都可点击或选择，但它们的输入方式、状态、布局和输出 oracle 还不相同。先保留产品私有实现。

## 动画与动效的近期方向

### 先收口时间，不先设计动画框架

第一步只建立一个确定性的呈现推进器。它应满足：

- 输入是单调时间和语义状态/事件。
- 输出是当前表现快照和一个 next deadline。
- 同一输入产生同一输出。
- 测试可以使用人工时间，不依赖真实 sleep。
- 暂停、继续和跳过是显式策略。
- host 不再分别保存五组 deadline。

当前代码已经证明两种组合需求：

- 战斗 Cue 按顺序播放。
- 世界移动、相机位移和精灵帧需要并行推进。

实现只需要覆盖这两个已存在的需求。不要提前建立通用 timeline graph。

### 区分三个概念

#### 语义事件

例如 `DamageApplied`、`Fainted`、`Moved`。它们来自 domain/application/session。

#### 表现片段

例如受击闪烁、移动插值、精灵循环。它们决定持续时间、进度和完成条件，但不包含 GPU 对象。

#### 绘制结果

例如某个稳定资源键在某个位置、颜色和 layer 上显示。它由 target planner 生成。

禁止从 GPU frame 反推语义状态，也禁止用动画是否结束推导领域阶段。

### 天气如何接入

当前只保留边界，不定义天气规则：

```text
domain weather state/event
-> application observation
-> session scene/cue
-> presentation ambient/transient state
-> target layer
```

常驻天气和天气开始/结束演出是两种不同表现。它们都不应成为 host 的新 `Option<Instant>`。

### 剧情演出如何接入

当前不设计剧情脚本格式。第一个剧情切片到来时，先验证：

- 谁拥有演出期间的输入锁定。
- 角色移动是否提交世界命令，还是只改变表现位置。
- 对话、镜头和角色动作如何串行或并行。
- 中断、跳过和场景退出的语义。

这些问题应由一个真实剧情切片回答。不要先创建通用 Cutscene DSL。

## 分阶段迭代

### 阶段 0：冻结当前行为

动作：

- 保留 battle session 的 transition/reducer/phase 测试。
- 保留世界移动、按键优先级和像素位移测试。
- 为 Console 打开期间的时间行为补充明确 oracle：暂停或继续必须二选一。
- 记录 Game 与 Map Editor 当前 frame 输出和关键截图。

完成标准：后续只移动职责，不改变当前视觉和交互时，测试能发现回退。

### 阶段 1：建立单一 Native frame 提交边界

动作：

- 合并 Game 和 Map Editor 重复的 frame、text renderer 和 present 编排。
- 让一帧只通过一个 target composer 生成和提交。
- 明确 grid canvas、图片、文字、矩形和 clip 的顺序。
- 保持现有固定 grid 视觉不变。

完成标准：

- `game-host::text` 与 `map-editor::text` 不再各自维护一份 glyphon renderer。
- host 不再接收可任意修改的 GPU Vec。
- Game、Map Editor 各有一个完整 frame oracle。

### 阶段 2：缩小 `game-ui`

动作：

- 按 Battle、World、Console、资源/样式和 frame projection 拆模块。
- 把 atlas 构建移出 UI projection。
- 把世界输入映射收口为一条路径。
- 把 `BattleUiState` 继续作为产品私有的表现状态，不藏进通用框架或 `game-session`。

完成标准：

- 修改 Battle UI 不需要读取 World、Console 和 atlas 代码。
- `game-ui` 不再分配裸 `ResourceId`。
- 真实 host 和 E2E 使用相同输入 action 路径。

### 阶段 3：建立确定性呈现推进器

动作：

- 把 `WorldMotion`、battle playback interval、sprite cycle 和收尾计时移出 host 事件循环。
- 使用人工时间测试顺序、并行、完成和暂停。
- host 只保存一个 next deadline，并调用一次 advance/update。
- 统一 Battle、World 和 Console 模式切换时的时间策略。

完成标准：

- `CreatureGameApp` 不再新增或维护效果专用 `Option<Instant>`。
- 关闭 Console 不会产生未定义的动画跳帧。
- 既有移动和战斗回放行为保持不变。

### 阶段 4：显式场景与 layer 组合

动作：

- 让 World map、player、HUD 和 Console 作为显式 layer 组合。
- 删除 `replace_world_background` 这类完成后再修改 frame 的路径。
- 让 Battle、World、Editor 都通过同一 target composer，但保留各自 presenter。
- 集中稳定资源键到 GPU atlas ID 的映射。

完成标准：

- layer 顺序和 clip 有纯 plan 测试。
- feature 代码不直接 push `GpuImage`。
- 新增一个 overlay 不需要修改 host redraw 主流程。

### 阶段 5：用一个真实新切片验收

天气或剧情演出只选择一个作为纵向切片。不要同时建设两套系统。

验收重点：

- 领域语义没有进入 GPU 层。
- 新效果没有给 host 增加专用 deadline。
- 常驻效果和瞬时演出可以并存。
- 输入锁定由 session/演出状态明确决定。
- Game 与 Map Editor 的 target 基础没有分叉。

完成后再决定哪些能力应提升到 Punctum。提升依据是相同语义和相同测试 oracle，不是类型名称相似。

## 近期禁止项

出现下面任一情况，应停止加功能并先修边界：

- 在 `CreatureGameApp` 新增效果专用 `Option<Instant>`。
- 在 feature presenter 中新增 `wgpu` 或 `punctum-wgpu` 依赖。
- 在 domain、application 或 session 中出现 `ResourceId`、`Rgba8` 或动画时长。
- 为天气或剧情向 `BattleAnimation`、`WorldAnimation` 持续堆无关变体。
- 通过修改完成后的 `GameView` 叠加新 layer。
- 新模块自行选择未登记的裸 `ResourceId` 范围。
- 同一个规范化按键在 host 和 UI 各映射一次。
- 为单个新效果建立通用插件、注册表或脚本语言。
- 为了使用 `punctum-ui` 把当前产品状态强制转换成通用 ScreenModel。

## 验证方式

每阶段先运行相关 crate：

```powershell
cargo test -p battle-session -p game-ui -p game-host -p map-render -p map-editor
cargo clippy -p battle-session -p game-ui -p game-host -p map-render -p map-editor --all-targets -- -D warnings
```

跨 crate 边界稳定后运行：

```powershell
cargo test --workspace
```

呈现改动还应保留：

- 纯 frame/plan fixture。
- 人工时间测试。
- Game 和 Map Editor 的关键截图。
- 至少一个真实 wgpu smoke。

## 文档处置

本文件替代以下已失真的文档：

- `battle-session-design.md`：迁移已完成，稳定部分已由代码和测试表达；旧文件混入大量历史阶段和未来推测。
- `weather-system-design.md`：提前冻结了尚未验证的天气模型和实施顺序。

后续天气实现应在真实需求进入时写短期迭代文档。只有稳定规则经过实现和测试验证后，才更新长期架构文档。
