# Punctum / Ramus 项目群总控

- 状态：产品需求和第一期架构已批准；`S0`、三个 `F1` lane、`B1`、`PT1`、`F2` 与 `R2` 已完成
- 当前阶段：等待 `B2`；Terminal 与 GPU 的纯逻辑和平台副作用已按 crate 隔离
- 本轮记录：`R2` 新增 `punctum-crossterm` 和 `punctum-wgpu`，下一阶段按 `B2 -> F3 -> B3` 推进
- 更新日期：2026-07-12

## 文档职责

这份文档只记录跨项目的产品事实、阶段目标、验收、非目标、决策边界和规划状态。

架构、crate、接口、依赖和 Agent wave 必须从产品规格推导。在架构评审完成前，它们都不是既定答案。

总控 Agent 是本文件的唯一写入者。项目 Agent 不直接修改本文件。

仓库中其他已有项目不在范围内，不作为实现来源，也不构成兼容约束。

## 文档语言

- Markdown 的标题、正文、状态、结论和操作说明使用中文。
- 技术术语、代码、命令、类型名、标识符和路径使用英文原文。
- 不用英文句子承载普通说明，也不把已有技术术语强行翻译成中文造词。
- 总控 Agent 在接收其他 Agent 的文档后负责检查这条规则。

## 事实来源

- 本文件是项目群长期产品事实来源。
- [第一期架构计划](./punctum-ramus-architecture-plan.md)

`.omx/` 下的访谈规格、摘要和上下文只用于当前 session 审计。该目录被 Git 忽略，长期决策不能只写在那里。

发生冲突时，本文件中的产品事实优先于访谈前形成的技术草案。Punctum PEP 中与本文件冲突的部分需要在架构阶段重新评审，不能静默沿用。

## 产品北极星

Punctum 是一套可复用 UI 框架。它不是只为一个游戏写的渲染器。

框架必须通过可运行程序证明边界，而不是通过假想平台证明“通用”。当前先使用 Terminal/GPU 双后端 Tetris 打磨 grid、input、text、adapter、layout、focus 和 widget 基座，再进入 Native GPU 宝可梦游戏界面。TUI AI Chater 暂停，不再阻塞第一期，也不再作为当前抽取公共 UI API 的必要消费者。

Tetris 只能证明基座 API 可运行，不能独自证明公共 API 已经跨产品稳定。上层 component state、lifecycle、focus、widget、layout tree 和 event routing 先保持 provisional；宝可梦游戏实际使用并收窄 API 后，才评估稳定合同。

Punctum 可以是一组分层能力，不要求所有产品使用同一套上层 UI runtime。平台渲染、产品状态和交互组合可以不同，但不能复制已经确定共享的 grid/input 基础。

## 开发与验证理念

项目使用 TDD。测试先表达玩家可见行为、领域不变量、边界合同或失败场景，再实现最小通过代码，最后重构。测试数量和 coverage 数字不是开发目标。不得只为了执行某一行、getter、平台错误分支或 IO 路径而增加没有行为价值的单元测试。

纯逻辑 crate 使用 line、function 和 region 100% coverage 作为收尾门禁。coverage 用于发现 TDD 遗漏，不能反向充当测试需求清单。平台和 IO crate 不设置 coverage 百分比，改用合同测试、fixture、错误场景、smoke 和 E2E 验证。

纯逻辑与副作用的边界由 crate 表达，不使用文件名或 `runtime.rs` 排除规则决定 coverage 范围。`R2` 已在不改变产品行为的前提下完成该边界。

## 项目群

| 项目 | 产品职责 | 当前事实 |
| --- | --- | --- |
| Punctum | 可复用 UI 核心及平台适配 | grid/input、Terminal、GPU 与平台 adapter crate 拆分已完成；Tetris GPU 入口和上层 UI 尚未完成 |
| 游戏 | 最终成为完整第三世代风格 RPG | 已完成纯 Battle domain/application 与侧别观察合同；UI、Ramus 集成、host 与租借数据仍待后续 wave |
| Ramus | 把结构化 application API 投影成类型安全 shell 命令 | F1 核心与 Python typed LCOV branch verifier 已完成，并通过安全复审和新旧覆盖率门禁 |
| 游戏控制台 | 玩家可选的 Ramus 入口 | 属于游戏，不是独立产品 |
| TUI AI Chater | 备选 Terminal TUI 产品 | 已建立 `S0` workspace 空壳；当前暂停，不进入第一期关键路径 |

产品名 `Ramus` 及其 workspace/crate 路径已在 `B1` 冻结。其他未实现模块仍以架构计划中的工作名为准，不能把工作名反向写成产品事实。

## 最终游戏产品

终态是完整 RPG，包括：

- 地图探索。
- 捕捉。
- 养成。
- 剧情。
- 对战。

第一期不做缩小版开放世界。第一期只做核心玩法验证：人类与 Agent 完成一场随机租借队伍的 6v6 对战。

## 第一期游戏闭环

1. 启动对战。
2. 为玩家随机生成 6 只租借宝可梦。
3. 为 Agent 随机生成 6 只租借宝可梦。
4. 玩家通过键盘界面选择合法动作。
5. Agent 通过注册的 Ramus 命令选择合法动作。
6. 两条输入路径调用同一个结构化对战 application API。
7. 对战引擎结算回合，直到一方获胜。
8. 显示胜负并结束本局。

第一期借用“对战工厂”的随机租借队伍思路，不做连续挑战、战后交换或长期奖励。

## 地图与场景边界

地图逻辑单位与渲染单位必须隔离。游戏逻辑使用 `TilePos`、地图尺寸、`TileId` 和碰撞规则，不读取像素、DPI、纹理或终端 Cell。

`Camera`、`Viewport`、`TileLayer` 和 `SpriteLayer` 属于游戏业务与 backend adapter 之间的场景层。场景层决定看见哪些逻辑对象；Terminal adapter 把可见对象投影为 `TerminalCell`，GPU adapter 把它们投影为 atlas region、instance 和 pixel rect。一格最终显示多少像素只由 GPU 投影决定。

当前只要求 GPU adapter 保持逻辑 grid 到像素 viewport 的明确转换边界，并由 Tetris 验证缩放不会改变业务状态。完整地图、相机、tile layer 和 sprite layer 在宝可梦地图垂直切片前实现。没有稳定消费者前不创建 `punctum-scene` crate。

## 人类与 Agent 的公平边界

```text
Human keyboard UI ───────────────> structured battle application API
Agent -> registered Ramus command ─> structured battle application API
```

- 玩家键盘操作直接调用结构化接口，不经过 Ramus parser。
- Agent 不直接访问对战引擎内部状态，只能调用获准的 Ramus 能力。
- 两条路径最终进入同一套对战规则和状态转换。
- 同一个合法动作不能因为来源不同而拥有不同语义。

## Ramus 产品定义

Ramus 是结构化 application API 的类型安全 shell 投影。

- 玩家可以在游戏内打开控制台并使用 Ramus。
- Agent 使用注册的 Ramus 命令观察和操作获准能力。
- 开发者可以注册内部、调试和作弊能力。
- “已注册”不代表任何主体都能发现或调用。

Ramus 至少区分三类 principal：

| Principal | 典型能力 |
| --- | --- |
| Player | 玩家被允许观察和执行的正式能力 |
| Agent | Agent 被允许观察和执行的对战能力 |
| Developer | 显式授予的内部、调试和作弊能力 |

命令发现、补全、读取和调用都必须按 capability 过滤。权限不能只在执行失败时检查。

## TUI AI Chater 暂停状态

TUI AI Chater 的 workspace 与既有记录保留，但业务实现、model adapter、UI 和 E2E 全部暂停。恢复时重新确认产品范围和它对 Punctum 的真实需求，不沿用“必须作为第二消费者”的旧前提。

## 第一期 UI 框架边界

第一期必须证明：

- Tetris 的 Terminal/GPU 入口复用同一套二维离散空间和键盘输入合同。
- Tetris 业务状态不读取 Terminal Cell、像素、DPI、atlas 或 GPU resource。
- 游戏 UI 继续以键盘为主要输入，以离散布局为当前布局范围。
- 游戏和 TUI 的平台适配不把宿主差异反向塞进共享产品状态。
- 接入第二个产品时，不复制 grid geometry、surface/diff 或 keyboard event 基础。

component tree、状态模型、lifecycle、focus、widget、layout tree 和 event routing 不属于已确认硬交集。架构阶段可以提出可选共享模块，但必须分别证明，不得绑定到最底层 grid/input core。

### 上层组件产品词汇

Punctum 后续使用四层词汇描述 UI 能力。层级按业务语义和依赖方向划分，不按代码量或视觉复杂度划分。

| 层级 | 业务知识 | 职责 | 典型例子 |
| --- | --- | --- | --- |
| UI Primitive | 无 | 提供布局、绘制、裁剪和样式基础 | `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer` |
| Framework Widget | 无 | 在 Primitive 上提供焦点、状态和通用交互 | `Button`、`TextInput`、`Checkbox`、`List`、`ScrollView` |
| Shared Pattern | 少量或无具体业务规则 | 组合 Widget 与 Primitive，表达重复的产品界面模式 | `FormField`、`StatusBar`、`ShortcutHint`、`ConfirmationDialog`、`LabeledValue` |
| Feature Component | 有 | 显示业务状态并产生业务意图 | `TetrisBoard`、`ScorePanel`、`ChatMessage`、`ModelSelector` |

这四层必须遵守以下产品边界：

- UI Primitive 和 Framework Widget 不包含 game、battle、chat 或 backend 专属概念。
- Shared Pattern 默认归实际消费产品所有。只有多个消费者证明语义和行为相同后，才能提升为 Punctum 公共能力。
- Feature Component 归业务项目所有。它可以读取业务状态并产生业务 action，但不能直接操作 Terminal、GPU 或业务核心内部状态。
- Terminal 和 GPU 的提交模式不能暴露给组件。组件只处理约束、布局、绘制意图、通用交互和业务 action。

上层组件采用分阶段验证，不一次实现完整目录：

1. 第一组候选为 `Text`、`Row`、`Column`、`Border`、`Padding`、`Spacer`、`Align` 和 `SurfaceView`。它们先验证 measure、layout、paint、clip 和 resize。
2. 布局基础稳定后，再验证 `WidgetId`、事件分发、焦点系统和 `Button`。
3. `Checkbox`、`List` 和 `ScrollView` 由 Game 的真实需求触发。
4. Terminal 只读文本投影已经具备 grapheme、Unicode width 和 continuation。`TextInput` 仍须等待 IME composition、光标、选区、删除语义和滚动合同明确后再进入公共候选。

第一组候选可以用于给 Tetris 增加标题、消行数、快捷键提示和带边框的页面布局。Terminal 文本投影完成不等于 `Text` Primitive 已实现；后者仍需要共享的 measure、layout、paint 和 clip 合同。Tetris 只验证 API 可行性，不能单独证明公共 API 已经跨产品稳定。完整候选目录、crate 策略和提升门禁见[第一期架构计划](./punctum-ramus-architecture-plan.md)。

## 第一期验收

### 游戏

- 玩家可以用键盘完成一场随机租借队伍的 6v6 对战。
- Agent 可以通过 Ramus 完成同一场对战。
- 对战可以正常结束并显示结果。
- 玩家和 Agent 的合法动作进入同一结构化对战接口。

### Ramus

- 玩家、Agent、开发者看到的命令集合符合各自 capability。
- Agent 不能发现或调用未授权的开发者/作弊能力。
- 开发者可以在不修改对战引擎公共规则的前提下获得显式调试能力。
- 玩家键盘主路径不依赖 Ramus parser。

### Tetris 基座验证

- 同一份 Tetris 业务状态可以由 Terminal 与 GPU 两个后端呈现。
- backend 缩放、窗口 resize 和提交模式不改变 Tetris 业务状态。
- Terminal Unicode 文本可以显示标题、状态和操作提示，不留下孤立 continuation。

### UI 框架

- Tetris Terminal/GPU 与 Native GPU 游戏 UI 依赖同一 grid/input 基础。
- 消费方不复制 grid geometry、surface/diff 或 keyboard event 合同。
- 可选上层模块不向 grid/input 基础加入 Tetris 或 game 专属概念。
- 验收不依赖鼠标、触屏、DOM 或连续布局。

## 第一期非目标

- 地图探索、捕捉、养成和剧情。
- 自由组队、完整配招和养成配置。
- 对战工厂连续挑战、战后交换、奖励和长期状态。
- 鼠标输入。
- 触屏输入。
- Web DOM。
- 连续像素布局。
- 为没有真实消费者的场景提前建立通用抽象。
- TUI Chater 的业务实现、model adapter、UI 和 E2E。
- 让普通玩家或 Agent 自动获得内部、调试或作弊权限。

## 决策边界

用户决定：

- 产品范围和 milestone 范围。
- 玩家可见行为。
- 第三世代规则的取舍。
- Ramus principal 和 capability 政策。
- 任何扩大或削弱第一期产品承诺的变化。

总控 Agent 可以决定：

- 架构和模块边界。
- 内部 API 和数据结构。
- 技术实现。
- 测试与验证方式。
- Agent 任务拆分、写入所有权和并行调度。

任何技术决定只要改变玩家可见行为、游戏规则、权限暴露或产品范围，就必须返回用户确认。

## 产品里程碑

| Milestone | 结果 | 状态 |
| --- | --- | --- |
| `P0 Product Clarified` | 产品目标、第一期、非目标和决策边界明确 | 已完成 |
| `A0 Architecture Approved` | 从产品规格推导共享边界、合同和验证方案 | 已完成 |
| `S0 Workspace Ready` | 四个独立 workspace、门禁记录和 baseline 就绪 | 已完成 |
| `G1 Battle Proof` | Native GPU 下的人类对 Agent 随机 6v6 跑通 | 规则引擎已完成；等待 UI、Ramus adapter、host、租借数据与集成 |
| `T1 Tetris Foundation Proof` | Tetris 通过 Terminal/GPU 验证 adapter、文本与后续 UI 基座 | Terminal 已通过；等待 GPU 入口、双后端验收与上层 UI |
| `U1 Provisional UI Accepted` | Tetris 证明 provisional UI 可用，Game 完成消费评审 | 等待 `T1` 和 Game UI |

## 当前规划状态

第一期架构计划已经通过 Planner、Architect 和 Critic 评审。最终 Critic 结论为 `APPROVE`。

正式结论记录在[第一期架构计划](./punctum-ramus-architecture-plan.md)。主要决定如下：

- mandatory shared foundation 只有 grid/input。
- 上层 interaction 能力先由 Tetris 做 provisional 验证；稳定公共合同仍需 Game 实际消费或第二个真实消费者提供同构证据。
- Punctum、Ramus、Game、Chater 和 Tetris 使用五个独立 Cargo workspace。Tetris 是 proof project，不是第三个最终产品。
- 唯一 Program Integration Agent 管理 root manifest、lockfile、path dependency、composition 和跨域 E2E。
- 实现按 `S0 -> F1 -> B1 -> PT1 -> F2 -> R2 -> B2 -> F3 -> B3 -> F4 -> F5` 推进。

## 下一阶段执行顺序

当前详细规划做到 `B3`。执行顺序固定如下：

1. `F2` 已完成 GPU 键盘事件规范化，并冻结当前本地 GPU adapter 合同。
2. `R2` 已用 `punctum-crossterm` 和 `punctum-wgpu` 隔离平台副作用，不改变现有行为。
3. `B2` 接通 Tetris GPU 入口，完成 Terminal/GPU 双后端本地验收。
4. `F3` 先批准 provisional UI 方向，再实现无状态布局、只读文本、焦点和基础 widget。
5. `B3` 接受 Punctum 与 Tetris 的 UI 依赖变化，验证 UI crate 不依赖 backend 或产品类型。

`GPU-REF-v0.1` 继续阻塞固定环境 readback 和正式 release oracle，但不阻塞 `B2` 双后端 Tetris 或 `F3` UI foundation。

`S0` 已由唯一 Program Integration Agent 完成，并通过独立只读 verifier 复核。记录、baseline 算法、canonical path 表和验证证据位于 [`punctum-vsh-s0`](./punctum-vsh-s0/verification.md)。`S0` 交付轮本身没有进入 `F1`，当时没有实现 grid、input、Ramus、Battle、GPU、Terminal 或 Chater 业务逻辑。

`BATTLE-RULES-v0.1` 已于 2026-07-11 获用户批准：实现第三世代 6v6 单打核心规则，暂不实现特性、道具和复杂状态。canonical fixture 与 SHA-256 记录在 [`punctum-vsh-s0/records.json`](./punctum-vsh-s0/records.json)。

Battle F1 已完成 `battle-domain` 与 `battle-application`。核心使用显式 seed、结构化 command/event/error、侧别 observation、合法动作查询和事务式 submit；首个动作在双方提交前不会进入公开观察或事件流；核心不读取文件、系统随机源、时钟或平台 API。28 个 domain tests 与 15 个 application tests 本地通过，纯逻辑覆盖率门禁通过。

Ramus branch coverage 已接入 `packages/arbor-projects` Python verifier。verifier 使用 typed LCOV parser 读取生产源码记录，并逐条检查 `BRDA`，不能用 line、function 或 region coverage 替代 branch coverage。本地结果为 1529/1529 line entries、158/158 functions、148/148 branch entries，missed lines 和 missed branches 均为 0。旧 `Test-PureCoverage.ps1` 仍保留并通过交叉验证。

## `B1` 结果

- `BattleApplication` 只由 trusted host 持有。host 把两个 `BattlePerspective` 分别交给 Player 与 Agent；两条路径使用同一观察和操作合同，不能传入任意 `Side` 或读取完整快照。
- Ramus workspace/crate 已冻结为 `packages/ramus` / `ramus-core`，Rust crate 名为 `ramus_core`。
- Game adapter 已物理改名为 `battle-ramus-adapter`，并通过 canonical path 依赖 `ramus-core`。
- `apps/tetris` 已在 `PT1` 后成为独立 workspace。Terminal view 消费 `punctum-terminal`，Terminal main 消费 `punctum-crossterm`；adapter 不反向依赖 Tetris。
- Punctum、Ramus 与 Battle export hash、四个 workspace baseline 和验证规则记录在 [`punctum-ramus-b1`](./punctum-ramus-b1/records.json)。
- 本轮不建设 CI。`R2` 已把 19 项 Terminal 纯合同与 9 项 Crossterm 合同分开，把 GPU planner 和字节编码留在纯 crate，并把 8 项 winit 输入合同与 headless smoke 移入 `punctum-wgpu`。下一步在 `B2` 接通 Tetris GPU 入口。

`GPU-REF-v0.1` record 已建立，但未能固定的必填字段都保持 `Blocked`。因此 GPU readback 和 release gate 尚未通过。

## 待后续产品确认

以下问题不阻塞当前产品形状，但在对应实现前需要用户决定：

- `BATTLE-RULES-v0.1` 之外的第三世代规则精确复刻到什么程度。
- 随机租借队伍的数据范围和平衡规则。
- 对战 Agent 的模型来源和最低决策水平。
- TUI Chater 是否恢复，以及恢复后的最小产品范围。

## 变更规则

- 产品事实变化时，先更新产品规格，再更新本文件。
- 架构决策不能反向改写产品需求。
- Agent handoff 只记录结论、证据、风险和 change request，不粘贴完整聊天或调试过程。
- 本文件只在 milestone、范围、门禁、决策或状态变化时更新。
