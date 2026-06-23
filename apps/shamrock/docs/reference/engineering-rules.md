# 工程规则

这份文档定义当前仓库的工程化共识。  
目标不是“理想化流程”，而是阻止代码继续把现有问题放大。

## 1. 文档规则

- `docs/current/status.md` 只反映最新状态
- `docs/current/roadmap.md` 只写未完成主线
- `docs/defects/` 只记录当前仍成立的结构性问题
- 已经完成的任务，不在过程性文档里重复叙述

## 2. 文件规模规则

- 单文件默认目标：500 行以内
- 超过 500 行的文件：
  - 不要求立刻重写
  - 但新增改动优先拆小，不允许继续长胖
- 当前重点文件：
  - `crates/battle-core/src/lib.rs`
  - `crates/battle-data/src/lib.rs`
  - `crates/battle-cli/src/main.rs`

## 3. 边界规则

- `battle-core`
  - 只保留权威状态推进、流程编排、`BattleOp` 应用和权威日志
- `battle-mechanics`
  - 只保留纯规则计算
- `battle-format`
  - 继续承接格式规则，不把格式逻辑回流到 core
- `battle-view`
  - 只保留视图模型和投影，不放 TUI/CLI 专属逻辑
- `battle-replay`
  - 只保留 replay 记录、导入导出、重放、恢复、校验
- `battle-cli`
  - 只保留壳层、渲染适配和 demo 协调，不新增规则逻辑

## 4. 测试规则

- 规则改动必须带行为测试
- replay 改动必须带 replay 测试
- `cargo test --workspace` 是最低验证门槛
- 影响 goldens 的改动必须说明：
  - 是规则修正
  - 还是资产迁移

## 5. 缺陷文档规则

- 只记录：
  - 架构缺陷
  - 模型缺陷
  - 长期工程债
- 不记录：
  - 普通 bug
  - 一次性回归
  - 已经修完的问题

## 6. 提交流程规则

- 先更新实现，再同步文档
- 每轮工作结束前必须检查：
  - 当前状态文档是否过时
  - roadmap 是否仍只保留未完成主线
  - defects 是否还保留已解决问题

## 7. 当前优先级规则

当前工程化优先级：

1. 固定规则
2. 拆大文件
3. 收缩壳层职责
4. 再做更深层架构改造
