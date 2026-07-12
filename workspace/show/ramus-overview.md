# Ramus 临时概览

> 本文是便于快速了解 Ramus 的临时说明，不是架构事实来源。正式定义以 `ramus-core` README 和 Punctum / Ramus 管理文档为准。

## 一句话说明

Ramus 把应用已经定义好的结构化 API 投影成类型安全、可发现、可授权的命令接口。

它可以作为玩家控制台、Agent 和开发工具访问应用能力的统一入口，但不是操作系统 Shell。

## 解决的问题

应用通常已经有明确的命令、查询和状态转换。玩家控制台、Agent 和开发工具如果分别直接接入业务内部，会产生三类问题：

- 每个入口重复实现参数解析和类型检查。
- 权限容易只在执行阶段检查，导致未授权能力通过补全、错误信息或 Schema 泄漏。
- 不同入口可能绕过正式 application API，产生不同的业务语义。

Ramus 在调用方与 application API 之间提供统一边界：

```text
ShellText / Agent PlanDraft
            ↓
解析、Schema 与参数类型检查
            ↓
Principal 和 capability 检查
            ↓
密封为 TypedPlan
            ↓
执行前重新校验授权和版本
            ↓
Provider
            ↓
应用原有的命令、查询和状态转换
```

## 命令形式

Ramus v1 使用“节点路径 + 方法 + 参数”的形式：

```text
/<node-path> <method> [name=value | positional]...
```

例如：

```text
/battle/turn submit move=thunderbolt target=opponent:1
/battle/state get
/tetris/piece rotate
```

路径表示虚拟能力树中的地址，不是文件系统路径。

## 核心能力

### 类型安全

命令按照注册的 Schema 检查参数名称、参数数量和参数类型。当前支持字符串、`i64`、布尔值和枚举等类型。

外部输入先形成不可信的 `PlanDraft`。只有经过解析、能力过滤和 Schema 校验后，才能密封为可执行的 `TypedPlan`。

### 能力发现

Ramus 可以从同一份命令目录生成命令发现和补全结果。调用方不需要手工维护另一份命令列表。

发现和补全同样受权限控制。未授权命令不会出现在候选中。

### 权限隔离

Ramus 使用 principal 表示调用主体。当前产品设计至少区分：

- `Player`：正式玩家可以观察和执行的能力。
- `Agent`：Agent 可以观察和执行的能力。
- `Developer`：显式授予的内部、调试和作弊能力。

权限采用 default-deny。`discover`、`complete`、`read`、`write` 和 `invoke` 分别授权。

“命令已经注册”不代表所有主体都能发现或调用它。

### 安全执行

密封计划不代表永久授权。每个副作用执行前，Runtime 都会重新检查：

- principal 和 capability 是否仍然有效。
- catalog 和 Schema 版本是否改变。
- capability generation 是否改变。
- Provider、路径、方法和 effect 是否匹配。

检查通过后签发一次性 `EffectPermit`。Provider 必须消费 permit 才能执行操作。

这种设计可以处理计划生成后撤权、命令更新以及并发授权变化。

## Ramus 不负责什么

Ramus 不负责应用业务规则。它不实现对战规则、俄罗斯方块状态转换或游戏 UI。

它也不提供操作系统 Shell 的能力。v1 不支持：

- 管道和重定向。
- 变量和命令替换。
- 分号语句。
- 任意文件 IO。

复杂业务编排应由 application API 或显式 plan 节点提供，不能借用宿主 Shell 权限。

Ramus 也不替代玩家的主要键盘路径。例如玩家按键和 Ramus 命令都应该产生同一个业务 `Command`，再进入同一套状态转换。

## 当前落地情况

### ramus-core

`packages/ramus/crates/ramus-core` 已经实现：

- parser 和 AST。
- typed value 与 Schema。
- catalog 和 capability-filtered view。
- discover 和 complete。
- compiler、`PlanDraft` 和 `TypedPlan`。
- principal、授权、撤权和一次性 permit。
- Provider 绑定与 Runtime 执行。
- 输入、调用数量、参数和值深度等资源限制。

### Tetris

Tetris GPU 示例已经接入 Ramus command palette。玩家可以通过授权命令完成移动、旋转、软降、硬降和重新开始。

命令面板负责 UI 状态和模糊匹配。Ramus 负责授权候选、解析、密封和执行。Provider 最终只产生现有的 `TetrisCommand`，由 Host 进入原有 `transition`。

### 第三世代对战游戏

目标是让 Agent 通过 Ramus 读取自己被允许看到的对战 observation，并提交合法动作。

玩家键盘和 Agent Ramus 命令最终进入同一套 `battle-application` API。Agent 不能直接读取对战引擎内部完整状态。

目前 `battle-ramus-adapter` 仍是占位模块，真实对战集成尚未完成。

## 总结

Ramus 是应用能力与玩家控制台、Agent、开发工具之间的安全命令桥梁。

它统一命令描述、类型检查、能力发现、权限控制和执行入口，同时把业务规则继续留在 application/domain 内部。
