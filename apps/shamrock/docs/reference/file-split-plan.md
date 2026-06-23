# 第一轮拆分计划

这份计划只覆盖当前最明显的工程化问题：超长文件和职责混杂。

## 1. 当前目标文件

- `crates/battle-core/src/lib.rs`
  - 当前约 1400+ 行
- `crates/battle-data/src/lib.rs`
  - 当前约 500+ 行
- `crates/battle-cli/src/main.rs`
  - 当前约 1000+ 行

## 2. 拆分原则

- 不为了目录好看而拆
- 每次拆分只做职责切割，不改行为
- 能先搬纯函数，就先搬纯函数
- 能先搬壳层渲染，就先搬壳层渲染

## 3. 建议顺序

### Step 1：拆 `battle-cli/src/main.rs`

优先拆出：

- `rendering.rs`
  - 领域事件 / trace 事件文本渲染
- `demo_loop.rs`
  - plain / scripted battle loop 编排
- `replay_io.rs`
  - replay 保存
- `ai.rs`
  - demo AI 选择逻辑

理由：

- 这是最低风险拆分
- 不会碰核心规则
- 能最快降低壳层复杂度

当前状态：

- 已完成
- 当前结果：
  - `main.rs` 已降到约 365 行
  - 仍需继续观察 `demo_loop.rs` / `rendering.rs` 是否还要二次拆分

### Step 2：拆 `battle-core/src/lib.rs`

优先拆出：

- `rng.rs`
- `log.rs`
- `state.rs`
- `ops.rs`
- `turn.rs`
- `move_resolution.rs`

理由：

- 先按职责拆，不先追求“完美模块化”
- 让回合流程、状态结构、随机和 op 应用分开

当前状态：

- 已完成
- 当前结果：
  - `lib.rs` 已降到约 100 行
  - 逻辑已拆入 `log.rs` / `rng.rs` / `state.rs` / `ops.rs` / `turn.rs` / `move_resolution.rs`
  - 但后续仍需要继续观察流程协调复杂度，尤其是 `turn.rs` / `move_resolution.rs` / `ops.rs`

### Step 3：拆 `battle-data/src/lib.rs`

优先拆出：

- `ids.rs`
- `defs.rs`
- `bundle.rs`
- `type_chart.rs`

理由：

- 数据定义、demo 数据、类型表本来就不是同一层

当前状态：

- 已完成
- 当前结果：
  - `lib.rs` 已降到约 15 行
  - 逻辑已拆入 `ids.rs` / `defs.rs` / `pack.rs` / `bundle.rs` / `type_chart.rs` / `tests.rs`
  - demo 内容已经改为 `packs/gen1-demo/*.json` 表驱动

## 4. 当前不做

- 不在第一轮拆分里继续大改对外 API
- 不在第一轮拆分里顺手重写规则系统
- 不把拆分和功能扩展绑在同一提交
