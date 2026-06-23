# Shamrock AI Script 对战设计

## 1. 目标

这份文档只讨论 AI 如何作为 battle participant 接入，不讨论终端布局。  
AI 的核心形式是“外部脚本驱动的玩家”。

设计目标：

- AI 主输入是一段稳定、非结构化的局势 prompt text
- AI 也可以额外通过 CLI 查询命令获取更多公开信息
- AI 主输出是一个稳定的 `ActionToken`
- `battle-core` 保持纯函数，不感知脚本、prompt 和外部进程

非目标：

- 不把 AI 设计成内嵌规则引擎
- 不要求 AI 输出 JSON
- 不让 AI 直接接触内部 `TraceEvent` 或隐藏信息

## 2. 与 TUI 的边界

AI 和 TUI 是弱耦合关系。

共享的只有：

- 公开 battle 视图
- 统一动作标识 `ActionToken`
- battle session 的只读查询接口

不共享的东西：

- 不共享 TUI 布局
- 不共享 AI prompt renderer 和 TUI renderer
- 不要求 AI prompt 长得像 TUI 画面

因此：

- TUI 可以自由优化“人类阅读体验”
- AI 可以自由优化“脚本和 LLM 决策体验”

只要两者消费的是同一个公开事实源即可。

## 3. Agent 形态

建议把 AI 接入抽象成 battle agent：

```rust
trait BattleAgent {
    fn choose_action(&mut self, ctx: &AgentTurnContext) -> AgentDecision;
}
```

v1 提供三类实现：

- `HumanAgent`
- `HeuristicAgent`
- `PromptScriptAgent`

`PromptScriptAgent` 是重点。  
它通过执行一个外部脚本命令，在每回合拿到 prompt text，再返回一个动作 token。

## 4. AI 主输入

### 4.1 Prompt-first

每回合轮到 AI 时，系统生成一段自然文本 prompt，通过 `stdin` 喂给脚本。

prompt 的职责是：

- 先给 AI 一个足够决策的默认上下文
- 让简单脚本只靠这一段文本就能出招
- 保持文本短、稳定、非结构化

推荐段落顺序：

- `Battle`
- `Situation`
- `Recent Events`
- `Available Actions`
- `Decision Rule`

内容示意：

```text
Battle: shamrock-demo-001
Turn: 4
Side: Opponent
Request: choose one action now

Situation:
Your active: Blaze (Charmander) HP 24/39
Enemy active: Sparky (Pikachu) HP 17/35
Your remaining team: 2
Enemy remaining team: 2

Recent Events:
- Turn 3: Enemy used Thunder Shock
- Your active took 8 damage
- You used Ember
- Enemy active took 11 damage

Available Actions:
- move:0 Use Scratch
- move:1 Use Ember
- move:2 Use Growl
- switch:1 Switch to Shell

Decision Rule:
Return exactly one action token on the first non-empty line.
You may write a short reason after that.
```

### 4.2 Prompt 约束

必须保证：

- 段落顺序固定
- 只使用公开信息
- 不引入 trace 和隐藏状态
- 动作 token 和当前合法动作完全一致
- 默认长度受控，不做长篇分析报告

## 5. AI 补充输入

虽然主输入是 prompt，但也允许 AI 脚本通过 CLI 做额外查询。  
这是增强路径，不是主路径。

推荐只读命令：

- `shamrock battle show --battle <id> --side <side>`
- `shamrock battle actions --battle <id> --side <side>`
- `shamrock battle events --battle <id> --side <side> --recent <n>`
- `shamrock battle prompt --battle <id> --side <side>`

这些命令的用途：

- `show`：拿完整公开局势文本
- `actions`：只确认动作列表
- `events`：补看最近事件
- `prompt`：重新读取主 prompt

设计原则：

- 全部只读
- 全部稳定文本输出
- 全部只暴露公开信息

这样脚本可以有两种实现：

- 简单脚本：只读 stdin prompt
- 强脚本：读完 prompt 后再主动查询

## 6. AI 输出

脚本 stdout 采用最小协议：

- 第一非空行：动作 token
- 后续行：可选摘要

动作 token 统一格式：

- `move:<index>`
- `switch:<index>`

摘要的用途：

- 在 TUI 的 agent panel 展示
- 在 replay 里作为注释保存

摘要不参与规则。

stderr 只用于调试日志，不参与规则输入。

## 7. 运行时流程

单回合流程建议固定如下：

```text
BattleState
-> projection to public agent context
-> render prompt text
-> execute agent script
-> read action token
-> validate token
-> map to BattleAction
-> call battle-core step
```

关键约束：

- `battle-core` 不接触脚本执行
- 脚本执行属于 `battle-cli` 编排层
- token 校验失败时，不让错误输入进入核心

## 8. 失败与降级

必须支持这些失败情况：

- 脚本超时
- 脚本退出失败
- stdout 为空
- token 非法
- token 合法格式但不在当前合法动作里

默认降级策略：

- 记录一条 system event
- 标记 agent fallback
- 使用 `HeuristicAgent` 代打本回合

这样可以保证 battle 不因为脚本异常而中断。

## 9. Replay 和记录

Replay 权威仍然是：

- 初始状态
- seed
- 最终输入序列

AI 相关信息只作为附加注释保存。

建议补充记录：

- agent script 名称
- 本回合动作 token
- AI 摘要
- latency
- fallback 事件
- 可选的 prompt digest

注意：

- replay 不应依赖 AI 摘要才能重放
- 即使删掉 AI 注释，只要输入序列还在，battle 仍能复现

## 10. 工程边界

建议先放在 `battle-cli` 内实现。

推荐模块：

- `agent.rs`
- `agent/prompt_script.rs`
- `agent/heuristic.rs`
- `prompt.rs`
- `session.rs`

依赖方向：

```text
PromptScriptAgent -> session read APIs -> battle-core / battle-replay
```

而不是：

```text
PromptScriptAgent -> 直接访问内部状态细节
```

## 11. 测试要求

至少覆盖：

- 同一状态生成的 prompt 稳定
- prompt 不泄露隐藏信息
- `actions` 命令和 prompt 中动作列表一致
- 脚本正常返回 token 时能正确映射为 `BattleAction`
- 空输出、非法 token、超时都能 fallback
- Human vs AI 和 AI vs AI 两种模式可运行

## 12. 当前默认决策

这份文档锁定这些默认值：

- AI 主输入走 `stdin prompt`
- AI 可额外调用只读 CLI 查询命令
- 所有输入输出都用稳定文本，不用 JSON
- AI 只看到公开信息
- AI 返回的第一非空行必须是 `ActionToken`
- `battle-core` 保持纯函数，不感知 agent runtime
