# Punctum / VSH 项目群总控

- 状态：产品需求、第一期架构和 `S0` workspace 已批准
- 当前阶段：`S0` 已完成，`F1` 未开始
- 本轮记录：四个独立 Cargo workspace、门禁记录和初始 baseline 已通过独立只读验证
- 更新日期：2026-07-11

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
- [第一期架构计划](./punctum-vsh-architecture-plan.md)
- [Punctum 当前方向](../../apps/punctum/peps/0001-punctum-technical-direction.md)

`.omx/` 下的访谈规格、摘要和上下文只用于当前 session 审计。该目录被 Git 忽略，长期决策不能只写在那里。

发生冲突时，本文件中的产品事实优先于访谈前形成的技术草案。Punctum PEP 中与本文件冲突的部分需要在架构阶段重新评审，不能静默沿用。

## 产品北极星

Punctum 是一套可复用 UI 框架。它不是只为一个游戏写的渲染器。

框架必须通过真实产品证明复用，而不是通过假想平台证明“通用”。第一期使用两个产品验证：

1. Native GPU 宝可梦游戏界面。
2. Terminal TUI AI Chater。

两个产品必须复用 Punctum 的二维离散空间和键盘输入基础。更高层的 component state、lifecycle、focus、widget、layout tree 和 event routing 是否共享，要由真实重复证明，不能先写成产品要求。

Punctum 可以是一组分层能力，不要求所有产品使用同一套上层 UI runtime。平台渲染、产品状态和交互组合可以不同，但不能复制已经确定共享的 grid/input 基础。

## 项目群

| 项目 | 产品职责 | 当前事实 |
| --- | --- | --- |
| Punctum | 可复用 UI 核心及平台适配 | 有 README/PEP 和 `S0` crate 空壳，无业务实现 |
| 游戏 | 最终成为完整第三世代风格 RPG | 已建立 `S0` workspace 空壳；第一期只验证对战 |
| VSH | 把结构化 application API 投影成类型安全 shell 命令 | 已建立 `S0` workspace 空壳，无业务实现 |
| 游戏控制台 | 玩家可选的 VSH 入口 | 属于游戏，不是独立产品 |
| TUI AI Chater | Terminal TUI 大模型对话产品，也是第二个 UI 框架消费者 | 已建立 `S0` workspace 空壳，无业务实现 |

工作目录名、crate 名和内部模块名尚未批准。架构规划可以提出方案，但不能把技术工作名当成产品事实。

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
5. Agent 通过注册的 VSH 命令选择合法动作。
6. 两条输入路径调用同一个结构化对战 application API。
7. 对战引擎结算回合，直到一方获胜。
8. 显示胜负并结束本局。

第一期借用“对战工厂”的随机租借队伍思路，不做连续挑战、战后交换或长期奖励。

## 人类与 Agent 的公平边界

```text
Human keyboard UI ───────────────> structured battle application API
Agent -> registered VSH command ─> structured battle application API
```

- 玩家键盘操作直接调用结构化接口，不经过 VSH parser。
- Agent 不直接访问对战引擎内部状态，只能调用获准的 VSH 能力。
- 两条路径最终进入同一套对战规则和状态转换。
- 同一个合法动作不能因为来源不同而拥有不同语义。

## VSH 产品定义

VSH 是结构化 application API 的类型安全 shell 投影。

- 玩家可以在游戏内打开控制台并使用 VSH。
- Agent 使用注册的 VSH 命令观察和操作获准能力。
- 开发者可以注册内部、调试和作弊能力。
- “已注册”不代表任何主体都能发现或调用。

VSH 至少区分三类 principal：

| Principal | 典型能力 |
| --- | --- |
| Player | 玩家被允许观察和执行的正式能力 |
| Agent | Agent 被允许观察和执行的对战能力 |
| Developer | 显式授予的内部、调试和作弊能力 |

命令发现、补全、读取和调用都必须按 capability 过滤。权限不能只在执行失败时检查。

## TUI AI Chater 产品定义

TUI AI Chater 是一个 Terminal TUI 产品。

第一期最低结果：

1. 用户在终端输入消息。
2. 应用把消息发送给选定模型后端。
3. 应用显示模型回复。
4. 模型后端可以是本地 Ollama 或 DeepSeek。

它首先用于证明 Punctum UI 核心可以离开游戏宿主，在 Terminal TUI 中复用。更复杂的 Agent 工具、工作流和自主执行不属于当前最低产品承诺。

## 第一期 UI 框架边界

第一期必须证明：

- Native GPU 游戏界面和 Terminal TUI 对话界面复用同一套二维离散空间基础。
- 两个产品复用同一套键盘输入基础合同。
- 两个产品都以键盘为主要输入。
- 两个产品都以离散布局为当前布局范围。
- 游戏和 TUI 的平台适配不把宿主差异反向塞进共享产品状态。
- 接入第二个产品时，不复制 grid geometry、surface/diff 或 keyboard event 基础。

component tree、状态模型、lifecycle、focus、widget、layout tree 和 event routing 不属于已确认硬交集。架构阶段可以提出可选共享模块，但必须分别证明，不得绑定到最底层 grid/input core。

## 第一期验收

### 游戏

- 玩家可以用键盘完成一场随机租借队伍的 6v6 对战。
- Agent 可以通过 VSH 完成同一场对战。
- 对战可以正常结束并显示结果。
- 玩家和 Agent 的合法动作进入同一结构化对战接口。

### VSH

- 玩家、Agent、开发者看到的命令集合符合各自 capability。
- Agent 不能发现或调用未授权的开发者/作弊能力。
- 开发者可以在不修改对战引擎公共规则的前提下获得显式调试能力。
- 玩家键盘主路径不依赖 VSH parser。

### TUI AI Chater

- 用户可以在 Terminal TUI 中输入消息。
- 用户可以选择 Ollama 或 DeepSeek 后端。
- 应用可以显示模型回复。

### UI 框架

- Native GPU 游戏 UI 与 Terminal TUI Chater 都实际依赖同一 grid/input 基础。
- 两个产品没有复制 grid geometry、surface/diff 或 keyboard event 合同。
- 可选上层模块没有向 grid/input 基础加入游戏或 chat 专属概念。
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
- TUI Chater 的复杂 Agent 工具、工作流和自主长任务。
- 让普通玩家或 Agent 自动获得内部、调试或作弊权限。

## 决策边界

用户决定：

- 产品范围和 milestone 范围。
- 玩家可见行为。
- 第三世代规则的取舍。
- VSH principal 和 capability 政策。
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
| `G1 Battle Proof` | Native GPU 下的人类对 Agent 随机 6v6 跑通 | 等待 `BATTLE-RULES-v0.1` 和后续实现 wave |
| `T1 TUI Reuse Proof` | Terminal TUI Chater 复用 grid/input 基础完成对话 | 等待后续实现 wave |
| `U1 Phase-one UI Accepted` | 两个消费者证明分层复用成立 | 等待 `G1` 和 `T1` |

## 当前规划状态

第一期架构计划已经通过 Planner、Architect 和 Critic 评审。最终 Critic 结论为 `APPROVE`。

正式结论记录在[第一期架构计划](./punctum-vsh-architecture-plan.md)。主要决定如下：

- mandatory shared foundation 只有 grid/input。
- 上层 interaction 能力必须由游戏和 Chater 的真实同构重复触发新 ADR。
- Punctum、VSH、Game 和 Chater 使用四个独立 Cargo workspace。
- 唯一 Program Integration Agent 管理 root manifest、lockfile、path dependency、composition 和跨域 E2E。
- 实现按 `S0 -> F1 -> B1 -> F2 -> B2 -> F3 -> B3 -> F4 -> F5` 推进。

`S0` 已由唯一 Program Integration Agent 完成，并通过独立只读 verifier 复核。记录、baseline 算法、canonical path 表和验证证据位于 [`punctum-vsh-s0`](./punctum-vsh-s0/verification.md)。本轮没有进入 `F1`，没有实现 grid、input、VSH、Battle、GPU、Terminal 或 Chater 业务逻辑。

下一轮可以并行启动 `F1` 的 Punctum grid/input lane 和 VSH lane。Battle lane 在 `BATTLE-RULES-v0.1` 获得用户或 Product Owner 批准前继续阻塞。

`BATTLE-RULES-v0.1` 尚未获得用户批准。它只阻塞 Battle lane 和 game downstream，不阻塞 `S0`、Punctum、VSH 与 Chater 的准备工作。

`GPU-REF-v0.1` record 已建立，但未能固定的必填字段都保持 `Blocked`。因此 GPU readback 和 release gate 尚未通过。

## 待后续产品确认

以下问题不阻塞当前产品形状，但在对应实现前需要用户决定：

- 第三世代对战规则精确复刻到什么程度。
- 随机租借队伍的数据范围和平衡规则。
- 对战 Agent 的模型来源和最低决策水平。
- TUI Chater 是否需要流式输出、取消、历史记录和会话持久化。

## 变更规则

- 产品事实变化时，先更新产品规格，再更新本文件。
- 架构决策不能反向改写产品需求。
- Agent handoff 只记录结论、证据、风险和 change request，不粘贴完整聊天或调试过程。
- 本文件只在 milestone、范围、门禁、决策或状态变化时更新。
