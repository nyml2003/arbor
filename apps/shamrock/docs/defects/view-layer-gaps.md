# 视图层缺陷

## 1. `battle-view` 仍然不是完全中立的多终端视图层

现状：

- `battle-view` 现在已经有中立的 `BattleSnapshot` / `ActionDescriptor`，这是改进。[lib.rs](../../crates/battle-view/src/lib.rs)
- 但 `PublicBattleView` 里仍有大量最终展示文本。[public.rs](../../crates/battle-view/src/public.rs)

问题：

- 当前已经有 snapshot / public 两层，这是进步
- 但 `PublicBattleView` 里仍然混了结构化状态和渲染后字符串
- `ActionView` 混了动作语义和人类交互信息
- `UiEventLog` 直接存字符串
- `agent_summary` / `recent_events` 这种字段明显偏 TUI

风险：

- TUI 够用
- GUI 勉强能复用
- AI 支持最弱

## 2. `battle-view` 仍然把本地玩家视角写死进动作视图

现状：

- 当前 `action_descriptor` 仍然按 `SideId::Player` 取动作上下文。[snapshot.rs](../../crates/battle-view/src/snapshot.rs)

问题：

- 这让视图层天然偏“本地玩家操作面板”
- 不适合作为 spectator / AI / remote player 的共用动作协议

风险：

- GUI、观战和 AI 继续接入时，会被迫绕开当前 view 层

## 3. viewer profile 已落地，但还不够细

现状：

- 当前已经有 snapshot 和 public view 两层，也已经引入 `ViewerProfile`。[lib.rs](../../crates/battle-view/src/lib.rs) [snapshot.rs](../../crates/battle-view/src/snapshot.rs)

问题：

- 现在虽然有 `LocalPlayer / Spectator / Agent / Debug`
- 并且已经开始驱动动作展示差异
- 但还没有驱动真正不同的裁剪、可见性和更深的动作策略

风险：

- 未来支持 GUI / AI / 观战时，仍然可能继续在同一套 public view 上打补丁

## 4. `battle-cli` 的 TUI 仍然直接消费单一 `PublicBattleView`

现状：

- `battle-cli/src/tui.rs` 直接把 `PublicBattleView<Locale>` 当成 UI 协议。[tui.rs](../../crates/battle-cli/src/tui.rs)

问题：

- 这让 `battle-view` 更像“为 TUI 服务的公共模型”
- 不像“真正多终端共享的中立状态层”

风险：

- 未来再接 GUI / AI 时，会重新遇到协议不够中立的问题
