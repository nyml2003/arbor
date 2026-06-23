# Shamrock 当前状态

## 当前结构

- workspace 当前有 7 个 crate：
  - `battle-core`
  - `battle-data`
  - `battle-format`
  - `battle-mechanics`
  - `battle-replay`
  - `battle-view`
  - `battle-cli`

## 当前能力

- 1v1 单打可运行
- plain CLI 和 TUI 都可用
- demo 数据现在已经移到 `packs/gen1-demo/*.json`
- 合法动作枚举已经独立在 `battle-format`
- `battle-format` 现在已经有显式 `FormatPhase` / `FormatContext` 入口
- 纯规则计算已经独立在 `battle-mechanics`
- 公开 view-model 已独立在 `battle-view`
- replay 已支持：
  - JSON 导入导出
  - 完整重放
  - 事件校验
  - `restore_to_turn`
  - `restore_checkpoint`
- repo 内已有金样 replay：
  - `replays/first-playable-demo.json`
- demo 内容包现在走表驱动：
  - `packs/gen1-demo/data-pack.json`
  - `packs/gen1-demo/player-team.json`
  - `packs/gen1-demo/opponent-team.json`

## 当前规则面

- 常见基础招式类型已覆盖：
  - 伤害
  - 持续状态
  - 能力变化
  - 回复
  - 天气
  - 强制换人
- 属性克制当前支持 `200 / 100 / 50` 百分比倍率
- 低命中招式与 miss 回归测试已覆盖
- 当前已经有第一版“种族值 + 可选个体值 + 可选努力值 + 可选 Nature + 运行时能力值”链路
- 当前还没有：
  - 更完整的等级 / 最终能力值模型

## 当前验证基线

- `cargo test --workspace`
- replay 回归测试已经纳入 workspace 测试
- 最近一次记录覆盖率：
  - Regions `80.28%`
  - Lines `81.49%`

## 当前主要问题

- 当前 species stats 仍更像 demo 用战斗面板值，不是完整对战数据模型
- `battle-cli/src/main.rs` 已拆到 500 行以内，但 `demo_loop.rs` / `rendering.rs` 仍然偏大
- Phase 5 还没开始做：
  - 更完整的格式能力
  - 可替换 AI 策略
  - 更强的观战 / 脚本 / 外壳能力

## 当前判断

当前仓库的核心、view、replay 边界已经落地。  
下一步直接推进格式、AI 和更强外壳。

当前工程规则已写入：

- `docs/reference/engineering-rules.md`
- `docs/reference/comment-rules.md`
- `docs/reference/file-split-plan.md`

最近一次工程化推进：

- `battle-cli/src/main.rs` 已拆成：
  - `ai.rs`
  - `demo_loop.rs`
  - `rendering.rs`
  - `replay_io.rs`
- 当前行数：
  - `main.rs` 约 365
  - `demo_loop.rs` 约 349
  - `rendering.rs` 约 319
  - `ai.rs` 约 49
  - `replay_io.rs` 约 28
- `battle-core/src/lib.rs` 已拆成：
  - `log.rs`
  - `rng.rs`
  - `state.rs`
  - `ops.rs`
  - `turn.rs`
  - `move_resolution.rs`
  - `tests.rs`
- 当前行数：
  - `lib.rs` 约 100
  - `state.rs` 约 124
  - `turn.rs` 约 148
  - `move_resolution.rs` 约 103
  - `ops.rs` 约 155
  - `log.rs` 约 130
  - `tests.rs` 约 456
- `battle-data/src/lib.rs` 已拆成：
  - `ids.rs`
  - `defs.rs`
  - `pack.rs`
  - `bundle.rs`
  - `type_chart.rs`
  - `tests.rs`
- 当前行数：
  - `lib.rs` 约 15
  - `defs.rs` 约 90
  - `bundle.rs` 约 17
  - `packs/gen1-demo/*.json`
  - `pack.rs` 约 23
  - `type_chart.rs` 约 14
  - `ids.rs` 约 5
  - `tests.rs` 约 25
- `demo_pack.rs` 已移除，demo 内容改为从表驱动 pack 加载
- `PokemonTemplate` 现在支持可选 `level` / `individual_values` / `effort_values` / `nature`
- `battle-core` 现在会把 species stats + optional IVs/EVs/Nature 解析成运行时 `BattleStats`
- `battle-view` 已拆成：
  - `snapshot.rs`
  - `public.rs`
  - `text.rs`
- 当前行数：
  - `lib.rs` 约 124
  - `snapshot.rs` 约 125
  - `public.rs` 约 140
  - `text.rs` 约 21
- `battle-view` 现在已有中立 `BattleSnapshot` / `ActionDescriptor`，当前 `PublicBattleView` 改为从 snapshot 派生
- `battle-view` 现在已经有显式 `ViewerProfile`
- `ViewerProfile` 现在已经开始驱动动作展示差异：
  - `LocalPlayer` 保留当前人类交互式展示
  - `Spectator / Agent / Debug` 不再复用同一套动作显示
