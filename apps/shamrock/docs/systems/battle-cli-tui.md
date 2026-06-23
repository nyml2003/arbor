# Shamrock TUI 设计

## 1. 目标

这份文档只讨论人类用户使用的终端界面，不讨论 AI 如何决策。  
TUI 的职责是把当前 battle 的公开状态、合法操作、事件流和回放信息，以高密度但易读的方式展示出来。

设计目标：

- 人类玩家能快速看清局势并操作
- 调试时能方便观察 `DomainEvent` 和必要的 `TraceEvent`
- 支持单场 battle、回放和 AI 对战时的观战模式
- 不把 TUI 和 AI prompt 绑定成一套渲染器

非目标：

- 不在 TUI 里直接实现对战规则
- 不让 TUI 直接操作 `BattleState`
- 不让 TUI 决定 AI 输入格式

## 2. 与 AI 机制的边界

TUI 和 AI 是弱耦合关系。

它们共享的只有三类东西：

- 公开视图投影，例如 `PublicBattleView`
- 统一动作标识，例如 `ActionToken`
- battle session 查询边界

它们不共享的东西：

- 不共享具体渲染器
- 不共享 prompt 文案
- 不共享布局逻辑
- 不共享 agent 执行流程

换句话说：

- TUI 从公开视图渲染成人看的多面板界面
- AI 层从公开视图渲染成给脚本的 prompt text

两者可以使用同一个“数据源”，但不能耦合成同一个“展示实现”。

## 3. 视图模型

TUI 只读取 battle 编排层提供的只读视图，不直接接触权威状态。

建议准备一个专门给终端界面用的读模型：

```rust
struct PublicBattleView {
    battle_id: String,
    turn: u16,
    mode: BattleMode,
    request: ViewRequest,
    player: SidePanelView,
    opponent: SidePanelView,
    legal_actions: Vec<ActionView>,
    recent_events: Vec<EventLine>,
    recent_trace: Vec<EventLine>,
    replay_status: Option<ReplayStatusView>,
}
```

关键约束：

- 所有字段都来自公开信息
- `legal_actions` 的顺序稳定
- 事件文本不要求和 AI prompt 一样
- 允许 TUI 增加纯展示字段，例如颜色、标签、焦点状态

## 4. 界面布局

推荐使用 `ratatui + crossterm`，采用 5 区固定布局。

### 4.1 顶栏 Status Bar

显示：

- 当前模式：`Human vs AI` / `AI vs AI` / `Replay`
- battle id
- turn
- 当前 request side
- seed
- 连接或脚本状态摘要

风格要求：

- 一眼能看清 battle 是否仍在进行
- 用少量 Nerd Font 图标增强扫描速度
- 任何图标都要有纯文本等价
- `Q` 退出和 `?` 帮助必须固定可见

### 4.2 左侧主战场 Battlefield

上下两块对称展示双方状态：

- active 昵称
- 物种名
- HP
- 公开状态
- 剩余可战斗数量
- 最近一次动作摘要

要求：

- 先看见 battle 的核心信息，再看细节
- 玩家侧和对手侧布局固定
- 不因为装饰影响信息密度

### 4.3 右上操作区 Action Panel

显示当前 side 的合法动作：

- 数字索引
- `ActionToken`
- 动作文案
- 动作类型标签，例如 `MOVE`、`SWITCH`

要求：

- 顺序必须稳定
- 热键和动作 token 一一对应
- 人类和日志里看到的动作标识一致

### 4.4 右下 AI/Session 区 Agent Panel

这一块只负责显示 battle 会话中的附加元信息：

- 当前 agent 名称
- 最近一次 agent 摘要
- latency
- 是否触发 fallback

注意：

- 这是观测区，不是 AI 控制区
- TUI 只读展示 agent 状态，不决定 agent 输入输出协议

### 4.5 底部事件区 Event Console

默认显示 `DomainEvent` 的人类可读文本。  
可切换到：

- `Trace`
- `Replay`
- `System`

要求：

- 默认视图优先显示玩家关心的 battle 语义
- trace 是调试工具，不应该淹没主界面
- 每条事件带稳定短前缀，例如 `TURN`、`MOVE`、`DMG`

## 5. 交互模型

TUI 是键盘优先界面。

建议快捷键：

- `1-9` 选择动作
- `Tab` 切换焦点面板
- `E` 切换事件视图
- `R` 进入回放面板
- `A` 查看 agent 摘要区
- `Q` 退出

交互原则：

- 不依赖鼠标
- 不让焦点状态影响规则结果
- battle 进行中和 replay 模式的快捷键语义尽量一致
- 帮助信息不能只在首次进入时出现
- 帮助弹层必须可重复打开和关闭

## 6. 视觉规范

允许使用 Nerd Font 和少量 emoji，但只服务扫描效率。

推荐：

- `󰓥` 标记 turn
- `󰊠` 标记 HP
- `󰆋` 标记 action
- `󰭹` 标记 agent
- `💥` 表示高伤害
- `☠️` 表示倒下

限制：

- 不要依赖颜色表达唯一语义
- 不要用大量装饰符破坏对齐
- 不要让事件区变成彩色跑马灯

### 6.1 颜色和字体差异

颜色和图标都要考虑系统差异。

建议：

- 颜色自动探测
- 如果存在 `NO_COLOR`，则禁用颜色
- 非交互终端不依赖颜色
- Nerd Font 不做盲猜，采用显式切换

建议支持这些图标模式：

- `SHAMROCK_TUI_ICONS=ascii`
- `SHAMROCK_TUI_ICONS=unicode`
- `SHAMROCK_TUI_ICONS=nerd`

推荐默认值：

- 交互终端默认 `unicode`
- 非交互终端默认 `ascii`

## 7. 运行模式

TUI 需要支持三种模式：

### 7.1 Battle 模式

人类直接操作，或人类观战 AI。

### 7.2 Spectator 模式

AI vs AI，TUI 只展示局势和 agent 摘要。

### 7.3 Replay 模式

从 replay 文件恢复展示：

- 单步前进
- 按回合跳转
- 查看当时的事件流和动作

### 7.4 非 TTY 回退

如果当前环境不是交互终端，例如：

- 管道输入
- CI
- 脚本批量执行

则不要强行进入 `ratatui` 界面。  
应自动回退到纯文本 CLI，保证 battle 仍然可运行、可回放、可导出 replay。

## 8. 工程边界

建议在 `battle-cli` 内先实现，不急着拆独立 crate。  
等交互稳定后再评估是否独立成 `battle-tui`。

推荐模块边界：

- `tui.rs`
- `tui/layout.rs`
- `tui/widgets.rs`
- `tui/input.rs`
- `view.rs`

依赖方向：

```text
TUI -> battle session read model -> battle-core / battle-replay
```

而不是：

```text
TUI -> 直接修改 battle-core state
```

## 9. 测试要求

至少覆盖这些：

- 宽终端和窄终端下布局可读
- `Action Panel` 顺序稳定
- 关键视图可以做 snapshot test
- 同一 `PublicBattleView` 渲染结果稳定
- Battle、Spectator、Replay 三种模式都能进界面

## 10. 当前默认决策

这份文档锁定这些默认值：

- TUI 用 `ratatui + crossterm`
- 界面是 IDE 控制台风格，不走极简纯日志界面
- TUI 只依赖公开视图，不依赖 AI prompt renderer
- `ActionToken` 是 TUI 的稳定动作标识
- AI 信息只作为附加面板展示，不主导主界面结构
- 非 TTY 环境自动回退纯文本 CLI
