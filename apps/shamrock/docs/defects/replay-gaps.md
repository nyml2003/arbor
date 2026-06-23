# 回放与确定性缺陷

## 1. replay 已可用，但还没有增量 checkpoint

现状：

- replay 已支持 JSON 导入导出、完整重放、校验和 checkpoint 恢复。[lib.rs](../../crates/battle-replay/src/lib.rs)

问题：

- `restore_checkpoint` 仍然是从开局重放到目标 turn
- 没有增量状态快照
- 没有差分 checkpoint

风险：

- 长局回放会越来越慢

## 2. 计算精度规范还没锁死

现状：

- 当前实现主要走整数路径，这是好事。[lib.rs](../../crates/battle-mechanics/src/lib.rs)

问题：

- 向下取整、倍率链顺序、未来更复杂伤害公式的固定精度规则还没写成硬规范

风险：

- 同规则不同写法会导致结果漂移

## 3. 金样 replay 的版本迁移策略还没有

现状：

- repo 内已有金样 replay，并进入测试。[status.md](../current/status.md)

问题：

- 旧金样和新规则的兼容策略还没定义

风险：

- 规则演进一多，golden 资产会越来越难维护

## 4. repo 金样 replay 现在只锁“可重放”，没有锁逐事件真值

现状：

- 当前 repo 内金样 replay 测试主要验证“可解析、可重放、可恢复”。[lib.rs](../../crates/battle-replay/src/lib.rs)

问题：

- 它没有对 repo 内大 replay 资产做严格逐事件一致性断言

风险：

- 一些语义漂移可能不会第一时间被 repo 内金样文件拦住
