# Gen3 游戏状态与副作用隔离迁移方案

状态：待执行

更新日期：2026-07-14

适用项目：`apps/gen3-game`

关联文档：[Gen3 游戏呈现架构现状与迭代约束](presentation-architecture.md)

## 结论

本项目可以平滑迁移到下面的结构：

- 产品运行状态只有一条所有权链。
- 表现和交互状态只有一个 owner。
- 平台副作用只存在于少数 host/adapter crate。
- 其他 crate 只执行确定性转换或定义不可绕过的状态机。
- 每个迁移阶段都保持可编译、可测试、可运行。
- 临时兼容层最多保留一个阶段，迁移完成后立即删除。

目标不是消灭所有 `mut`。目标是消灭隐藏状态、重复状态、共享可变状态和跨层修改。

一个状态机可以使用 `&mut self`，只要它满足：

- 状态只有一个 owner。
- 修改只能通过公开 command/use case 进入。
- 结果只取决于当前状态和显式输入。
- 时间、随机数和外部数据由调用方传入。
- 相同输入产生相同状态、事件和快照。

这种状态机仍属于纯核心。

## 术语

### 外部副作用

调用结果依赖或修改进程外部环境：

- 文件系统。
- 窗口和平台事件循环。
- GPU surface、command encoder 和 present。
- 字体系统和 GPU 字形缓存。
- 系统时间。
- 系统随机源。
- 线程、锁和共享队列。

### 时序副作用

调用结果依赖没有出现在参数中的内部历史：

- 全局缓存。
- 单例。
- `Arc<Mutex<_>>` 共享状态。
- 多个模块分别保存同一个当前状态。
- renderer 根据上一帧调用推导当前状态。
- host 根据多个布尔值和 deadline 推导产品阶段。

本方案把时序副作用和外部副作用按同等严格程度处理。

### 纯状态机

纯状态机允许保存状态，但状态转换必须确定：

```text
CurrentState + Command + ExplicitContext
-> NextState + Events + Result
```

Rust 实现可以原地修改以减少复制。纯度由可观察行为决定，不由是否出现 `mut` 决定。

### 状态 owner

状态 owner 是唯一可以决定该状态生命周期和转换的 crate。其他 crate 只能：

- 提交 command。
- 读取只读 snapshot。
- 消费 event。
- 生成派生 view。

snapshot 是派生值，不是第二份权威状态。

## 当前状态分布

| 当前状态 | 当前位置 | 问题 | 目标位置 |
| --- | --- | --- | --- |
| 世界、战斗实例、当前场景、roster seed | `game-host::DemoGame` | 产品状态位于 host crate | `game-session` |
| 战斗 application、phase、回放步骤、Scene | `battle-session` 内部所有权链 | 边界基本正确 | 保留为 `game-session` 的战斗子会话 |
| Battle UI 页面、选择和提示 | `game-ui::BattleUiState`，实例由 `DemoBattle` 保存 | 产品会话与 UI 状态混在 host 组合对象 | `game-ui` 的表现状态 owner |
| Console 查询、候选、选择和诊断 | `game-host::ConsoleState` | 纯交互状态位于 effect host | `game-ui` 的表现状态 owner |
| WorldMotion、精灵帧和五组 deadline | `game-host` | 表现状态和系统时间混合 | `game-ui` 的确定性表现状态 |
| 系统时间随机 roster seed | `game-host` library | 纯产品构造读取系统环境 | host 生成 seed，再传给 `game-session` |
| 嵌入数据全局 `OnceLock` | `game-host` library | 隐藏的进程级状态 | composition root 显式持有并传入 |
| Ramus action queue | `battle-ramus-adapter` 的 `Arc<Mutex<_>>` | 共享可变队列隐藏 action 传递 | 继续隔离在 adapter；上层只接收执行结果 |
| atlas、窗口、GPU runtime、字体缓存 | `game-host`、`map-editor` | 合法平台状态，但实现重复 | native target/host adapter |
| EditorModel | `map-editor` binary crate | 纯编辑状态与 FS/GPU 在同一 crate | `map-editor-core` |
| 游戏数据导入文件读写 | `game-data-import` library | 纯解析和 FS 写入混合 | importer core + FS adapter |

## 目标所有权模型

### 1. 产品状态 owner：`game-session`

新增纯 crate `game-session`。它拥有一局游戏的产品生命周期：

- 当前 Game scene/mode。
- 世界状态。
- 可选的战斗子会话。
- 战斗进入和退出。
- 玩家 command 的合法路由。
- 构造时注入的 roster seed 和静态数据引用。

建议状态树：

```text
GameSession
|- WorldApplication
|- Option<BattleSession<OpponentPolicy>>
|- GameMode
`- SessionConfig
```

`BattleSession -> BattleCoordinator -> BattleApplication -> Battle` 可以先保留。这是一条嵌套所有权链，不是重复状态。

本轮不为了形式纯度把 `BattleApplication` 和 `WorldApplication` 强制改成无状态 service。只有出现第二个运行时 owner、跨会话共享或测试困难时，才继续收窄。

`game-session` 禁止依赖：

- `game-ui`
- `punctum-gpu`
- `punctum-wgpu`
- `wgpu`
- `winit`
- `glyphon`
- 文件系统
- `Instant`、`SystemTime`
- 系统随机源

它只接收显式 command、seed、数据和策略。

### 2. 表现状态 owner：`game-ui`

保留 `game-ui` 作为产品私有表现 crate，但收窄职责。它唯一持有：

- Battle 菜单页面和选择。
- Console 开关、查询、候选和选择。
- 世界移动的表现进度。
- 战斗 Cue 的播放进度。
- 精灵循环帧。
- 转向和跑步收尾。
- overlay 和输入焦点状态。

`game-ui` 接收：

- `GameSnapshot`。
- 规范化后的 UI input/intent。
- 逻辑时间增量 `Duration`。
- 稳定资源键和只读配置。

`game-ui` 输出：

- 需要提交给 `game-session` 的产品 command。
- 当前 `PresentationSnapshot`。
- 下一次需要推进的逻辑延迟。
- 纯产品 view/scene layer。

它不读取 `Instant::now()`，不访问文件，不创建窗口，不 present，不持有 GPU runtime。

第一阶段可以继续使用固定 grid view。不要为了迁移状态所有权同时引入通用 widget tree。

### 3. 平台状态 owner：host/native adapter

`game-host` 只持有平台对象和两个不透明 session：

```text
NativeGameHost
|- GameSession
|- PresentationSession
|- Window / EventLoop state
|- GpuRuntime / TextRenderer
|- Asset runtime
|- last_real_instant
`- platform input state
```

host 可以拥有 `GameSession` 和 `PresentationSession` 的实例，但不能直接修改其内部字段。它只能调用 command/update 并读取 snapshot。

host 负责：

- 把 winit 输入规范化。
- 读取一次系统单调时间并计算 elapsed `Duration`。
- 把 elapsed 传给 presentation。
- 把 presentation 输出的 command 提交给 game session。
- 加载文件和素材。
- 把纯 frame plan 提交给 GPU。
- 根据一个 next delay 设置唤醒。

host 禁止保存：

- 独立的 HP、天气、当前成员或战斗 phase。
- 独立的 UI 页面和选择。
- 效果专用 deadline。
- 由 GPU frame 反推的产品状态。

### 4. 工具状态 owner：`map-editor-core`

地图编辑器是独立产品流程。它需要自己的单一状态 owner。

`map-editor-core` 持有：

- `EditorModel`。
- 选择、工具、dirty、帮助开关和撤销历史。
- `EditorIntent -> EditorEffect` 状态机。

现有 `map-editor` binary 保留：

- 文件读取和保存。
- winit 输入。
- GPU 和字体提交。
- 鼠标坐标转换。

`EditorEffect::SaveRequested` 已经是合适的副作用边界。core 发出请求，host 执行保存，再把成功或失败结果作为 intent 传回。

## 目标依赖图

```text
                     +------------------+
                     | game-host        |
                     | winit/fs/time    |
                     +--------+---------+
                              |
              +---------------+----------------+
              |                                |
              v                                v
       +--------------+                 +--------------+
       | game-session |                 | game-ui      |
       | product owner|<-- command -----| presentation |
       +------+-------+                 +------+-------+
              |                                |
       +------+------+                  snapshot/view
       |             |                         |
       v             v                         v
+-------------+ +--------------+       +---------------+
| world core  | | battle-session|       | native planner|
+-------------+ +------+-------+       +-------+-------+
                      |                       |
                      v                       v
              +---------------+       +---------------+
              | battle core   |       | native adapter|
              +---------------+       | wgpu/glyphon  |
                                      +---------------+
```

所有箭头向内。core 和 session 不能反向依赖 host、native adapter 或文件系统。

## 状态转换合同

### Game session

概念合同：

```text
GameSession + GameCommand
-> Result<GameEvents, GameError>
```

每次提交必须原子完成：

- 校验当前 mode。
- 执行对应领域转换。
- 更新场景生命周期。
- 生成只读 snapshot/event。

UI 和 host 不能先修改一部分状态，再调用 session 补全。

### Presentation session

概念合同：

```text
PresentationState
+ GameSnapshot
+ PresentationInput
+ ElapsedDuration
-> PresentationOutcome
```

`PresentationOutcome` 包含：

- 新表现状态。
- 零个或一个产品 command。
- 当前 presentation snapshot/view。
- 下一次推进所需 delay。

第一版不要求把它实现成返回新值的函数。可以使用 `&mut self`，但测试必须证明确定性。

### Native planning

概念合同：

```text
ProductView + AssetCatalog + Viewport
-> Result<FramePlan, PlanError>
```

planner 可以生成 GPU 数据结构，但不能调用 wgpu。真正的 command encoding 和 present 只在 adapter 中发生。

## 时间策略

### 两种时间

系统区分：

- Real time：host 从 `Instant` 读取，只用于平台调度。
- Presentation time：presentation 持有的逻辑 elapsed time。

host 每轮只计算一次 real elapsed，然后提交给 presentation。其他 crate 不读取系统时间。

### Console 暂停决策

第一版明确选择：Console 打开时暂停 presentation time。

结果：

- 世界移动不会在 Console 后台完成。
- 战斗回放不会在 Console 后台推进。
- 关闭 Console 后从原进度继续。
- GPU surface 和输入事件循环继续工作。

如果未来产品要求后台继续演出，应修改 presentation policy 和测试，不能依赖 deadline 偶然过期。

### 单一唤醒

presentation 汇总所有当前表现状态，返回一个 next delay。host 只维护：

- 上一次真实 `Instant`。
- 下一次平台唤醒 `Instant`。

禁止为天气、镜头、角色、转场或单个 Cue 增加 host 字段。

## 副作用 crate 允许清单

Gen3 workspace 内允许出现外部副作用的边界：

| Crate | 允许副作用 |
| --- | --- |
| `game-host` | winit、系统时间、系统 seed、文件读取、进程日志 |
| app-private native target crate | wgpu、glyphon、GPU cache、surface frame encoding |
| `map-editor` host | winit、文件保存、鼠标平台事件 |
| `game-data-import` FS adapter/CLI | CSV/JSON 文件读取、原子写入 |
| `battle-ramus-adapter` | Ramus runtime/provider 和内部 action 交付 |

`punctum-wgpu` 是 workspace 外部已有平台 adapter，继续作为 GPU 副作用边界。

除允许清单外，其他 Gen3 crate 不得依赖 `winit`、`wgpu`、`glyphon` 或 `pollster`。

`std::fs`、`Instant`、`SystemTime`、`Mutex`、`RwLock`、`RefCell`、`Atomic*` 和 `OnceLock` 需要路径级审查。它们不是一律禁止，但必须有明确 adapter 或只读初始化理由。

## 平滑迁移原则

### 每个阶段采用四步

1. 用测试冻结现有行为。
2. 在新边界实现同一行为。
3. 迁移一个消费者。
4. 删除该消费者的旧路径。

任何阶段都不能以“新旧两条路都先留着”结束。

### 临时兼容层限制

允许：

- 旧类型到新 command/snapshot 的无状态转换。
- 旧公开函数暂时转发到新 owner。
- 为 E2E 保留一阶段 re-export。

禁止：

- 兼容层保存自己的状态。
- 新旧 owner 双向同步。
- 新旧路径分别处理一部分输入。
- 用 feature flag 长期保留两套实现。
- 在兼容层加入未来功能。

每个兼容层必须在同一阶段计划中写出删除条件。

## 迁移阶段

### 阶段 0：建立行为和架构门禁

动作：

- 保留当前 49 项聚焦测试和战斗核心测试。
- 增加 Console 暂停、输入消费顺序和关键 frame oracle。
- 记录当前 crate 依赖图。
- 建立副作用依赖 allowlist 检查。
- 标记当前全部 `Instant`、`SystemTime`、FS、wgpu、glyphon 和共享锁位置。

删除项：无。

完成标准：任何新增副作用依赖、状态镜像或 host deadline 都会在验证中失败。

### 阶段 1：统一 Native frame 副作用边界

动作：

- 合并 Game 和 Map Editor 重复的 glyphon renderer。
- 建立 app-private native target crate。
- 让一帧只在 native target 中 encode/present 一次。
- host 和 editor 只提交完整 frame plan。
- 保持现有 `GameView`、`EditorFrame` 视觉不变。

临时兼容：旧 frame 类型可以转换为新 frame plan，但转换不保存状态。

删除项：

- `game-host::text` 重复实现。
- `map-editor::text` 重复实现。
- 两套 `present_plan_with_overlay` 编排。

完成标准：只有 native target 和 `punctum-wgpu` 直接接触 wgpu/glyphon。

### 阶段 2：建立 `game-session` 产品 owner

动作：

- 新增纯 `game-session` crate。
- 从 `game-host` library 迁移 `DemoGame` 的世界、战斗和场景生命周期。
- 把随机 seed 改为构造参数。
- 把静态数据改为 composition root 显式传入。
- 输出 `GameSnapshot`、`GameCommand` 和结构化 `GameError`。
- 让 E2E 直接依赖 `game-session`。

临时兼容：`game-host` 可以 re-export 旧名称一个阶段。

删除项：

- `game-host` library 中的 `SystemTime` 和全局 seed counter。
- `DemoGame::view*` 等产品状态到 UI 的反向依赖。
- E2E 对 host 产品内部类型的依赖。

完成标准：`game-session` 的依赖树不包含 UI、GPU、平台、文件系统和系统时间。

### 阶段 3：把表现状态收口到 `game-ui`

动作：

- 从 `DemoBattle` 移出 `BattleUiState`。
- 从 host 移出 `ConsoleState`。
- 把 `WorldMotion` 改为使用逻辑 `Duration`。
- 把 battle playback、sprite cycle、turn hold 和 run stop 收口为一个 presentation state。
- presentation 输出产品 command 和 next delay。
- 明确 Console 暂停测试。

临时兼容：host 可以把现有 key event 转成新的 presentation input，但只能保留一条 action 路径。

删除项：

- `next_playback`。
- `next_sprite_frame`。
- `next_world_tick`。
- `turn_hold_ends`。
- `run_stop_ends`。
- 重复的世界方向键映射。

完成标准：host 只保存一个平台唤醒 deadline；presentation 测试全部使用人工时间。

### 阶段 4：清理画面和资源派生状态

动作：

- Product view 使用稳定 `AssetKey`，不使用裸 `ResourceId`。
- 资源 adapter 独占 `AssetKey -> ResourceId` 映射。
- World、Battle、Console 和 HUD 通过显式 layer 组合。
- sprite slot 查找成为无状态 catalog 查询。
- frame plan 完成后不可再由 feature 修改。

删除项：

- `replace_world_background`。
- feature 内的 atlas 编号区间。
- host 中的 layer push/坐标修补。
- `DemoBattle` 中仅用于贴图解析的 ID 镜像状态。

完成标准：feature presenter 不依赖 `punctum-gpu`；native planner 是唯一 GPU plan 生成者。

### 阶段 5：拆出地图编辑器和导入器纯核心

动作：

- 新增 `map-editor-core`，迁移 model/controller 和纯 view model。
- 保留 `map-editor` 作为文件、窗口和 GPU host。
- 把保存成功/失败作为 intent 返回 core。
- 把 `game-data-import` 的解析/校验/生成与 FS 读写分离。
- 保持现有 CLI 和地图文件格式不变。

删除项：

- editor core 对 winit/wgpu/glyphon/fs 的可见依赖。
- importer 纯转换函数内部的文件访问。

完成标准：两个 core 都可以只用内存 fixture 覆盖完整故事。

### 阶段 6：删除兼容路径并固定门禁

动作：

- 删除所有旧 re-export 和 adapter wrapper。
- 固定副作用 allowlist。
- 固定状态 owner 清单。
- 运行 workspace 测试、Clippy、依赖检查和真实 wgpu smoke。
- 用一个天气或剧情纵向切片验证边界。

完成标准：项目只有一条产品状态链、一条表现状态链和一条 native 提交链。

## 每阶段提交边界

一个阶段应拆成可独立回退的提交：

```text
1. characterization tests
2. new owner/adapter with no consumers
3. migrate one consumer
4. delete old path
5. architecture gate
```

每个提交都必须通过相关测试。不要在同一个提交中同时：

- 移动状态 owner。
- 改变用户行为。
- 更换视觉布局。
- 引入天气或剧情功能。

## 自动门禁

### 依赖门禁

使用 `cargo metadata` 检查：

- 只有 allowlist crate 依赖 `winit`、`wgpu`、`glyphon`、`pollster` 和 `punctum-wgpu`。
- domain/application/session 不依赖 UI 和 adapter。
- `game-session` 不依赖 `game-ui`。
- `game-ui` 不依赖 host。
- native target 不依赖 battle/world domain。

### 源码门禁

对纯 crate 检查禁止项：

- `std::fs`
- `std::time::Instant`
- `std::time::SystemTime`
- `winit`
- `wgpu`
- `glyphon`
- `pollster`
- 无批准的 `Mutex`、`RwLock`、`RefCell`、`Atomic*`、`OnceLock`

源码扫描只是快速反馈。最终以 Cargo 依赖、公开 API 和行为测试为准。

### 状态门禁

评审新增状态字段时必须回答：

1. 这是权威状态、表现状态、平台状态还是缓存？
2. owner 是哪个 crate？
3. 哪个 command 可以修改它？
4. 是否复制了另一层已经拥有的事实？
5. 是否可以从 snapshot/config 纯计算得到？
6. 生命周期结束时由谁清理？

不能回答时，不允许增加字段。

## 验证矩阵

| 合同 | 验证 |
| --- | --- |
| 产品确定性 | 相同 seed + command 序列产生相同 GameSnapshot/Event |
| 战斗确定性 | 保留 transition/reducer/phase 和最终 Scene 定律 |
| 表现确定性 | 相同 snapshot + input + elapsed 产生相同 PresentationSnapshot |
| 暂停语义 | Console 打开期间 logical time 不推进 |
| 单一输入路径 | 同一规范化输入最多产生一个产品 command |
| 单一 frame | 每次 redraw 最多 acquire/present 一次 surface frame |
| 资源隔离 | Product view 只出现 AssetKey，不出现 ResourceId |
| 副作用隔离 | 非 allowlist crate 无平台/FS/系统时间依赖 |
| 平滑迁移 | 每阶段旧路径删除后全部故事测试仍通过 |

## 推荐验证命令

迁移期间按阶段运行相关包：

```powershell
cargo test -p battle-domain -p battle-application -p battle-session
cargo test -p world-domain -p world-application -p game-session
cargo test -p game-ui -p game-host -p game-e2e
cargo test -p map-project -p map-render -p map-editor-core -p map-editor
cargo clippy --workspace --all-targets -- -D warnings
```

新 crate 尚未建立前，从命令中移除对应 `-p`。跨边界阶段完成后运行：

```powershell
cargo test --workspace
```

平台边界还要保留真实 Game 和 Map Editor wgpu smoke。

## 禁止项

- 不建立全局 `GameState` 单例。
- 不使用 `Arc<Mutex<GameState>>` 在 UI、host 和 adapter 之间共享产品状态。
- 不让 renderer 保存上一帧产品 snapshot。
- 不让 UI 修改 domain 或 application 内部对象。
- 不让 host 保存独立业务状态镜像。
- 不把系统 `Instant` 传入 domain/session。
- 不让 application 自行读取随机源。
- 不为迁移长期保留两套 command 或 view API。
- 不借迁移建立通用 ECS、事件总线、动画 DSL 或 service locator。
- 不把所有状态压入一个无法分辨生命周期的巨型结构。

## 最终完成标准

- `game-session` 是游戏产品状态的根 owner。
- `battle-session` 是唯一战斗子会话 owner。
- `game-ui` 是唯一表现和交互状态 owner。
- `map-editor-core` 是地图编辑状态 owner。
- host 只持有不透明 session 和平台状态。
- 所有系统时间、文件、GPU、字体和共享队列都位于 allowlist adapter。
- 相同状态、输入、seed 和逻辑时间产生相同结果。
- host 只有一个平台唤醒 deadline。
- Product view 不包含 GPU 资源编号。
- frame 完成后不再被 feature 修改。
- 旧兼容 API 和重复状态已删除。
- 新天气或剧情切片不需要新增 host 状态或副作用边界。
