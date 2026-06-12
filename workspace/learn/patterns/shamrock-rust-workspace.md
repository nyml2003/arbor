# 模式：Rust 多 crate 游戏引擎 workspace（shamrock）

## 一句话

用 Rust workspace 把宝可梦对战引擎拆为 8 个独立 crate——data/core/mechanics/format/view/replay/cli——每层单向依赖，核心 crate 不依赖 UI。

## 核心架构

```
battle-data        ← 数据结构（宝可梦、招式、属性）
    │
battle-core        ← 对战核心逻辑
    │
battle-mechanics   ← 机制计算（伤害公式、类型克制、状态效果）
    │
battle-format      ← 队伍/招式/规则的序列化格式
    │
battle-view        ← 可视化渲染
    │
battle-replay      ← 对局回放记录
    │
battle-cli         ← CLI 入口
```

依赖方向：`data ← core ← mechanics ← format ← view/replay/cli`。data 是最底层，cli 是最外层。

## 关键设计

### 1. Cargo workspace：编译隔离

```toml
[workspace]
members = [
    "crates/battle-data",
    "crates/battle-core",
    "crates/battle-mechanics",
    "crates/battle-format",
    "crates/battle-view",
    "crates/battle-replay",
    "crates/battle-cli",
]
```

每个 crate 有自己的 `Cargo.toml`，只声明自己需要的依赖。`battle-data` 不依赖任何 battle 包，`battle-cli` 依赖全部。编译器并行编译，改了 `battle-data` 只重新编译依赖它的包。

### 2. 单向依赖

```
battle-data        ← 零 battle 依赖
battle-core        ← 只依赖 data
battle-mechanics   ← 依赖 data + core
battle-format      ← 依赖以上全部
battle-view        ← 依赖 format
battle-replay      ← 依赖 format
battle-cli         ← 依赖全部
```

没有循环依赖。`battle-data` 改了不影响 `battle-core` 的测试。和 ObolosFS 的 core → driver、workshop 的 domain → application 完全同构。

### 3. 关注点分离

| crate | 关注点 | 不做什么 |
|-------|--------|---------|
| data | 宝可梦/招式/属性定义 | 不计算伤害 |
| core | 对战流程控制 | 不处理 UI |
| mechanics | 伤害公式、类型克制 | 不渲染 |
| format | 序列化/反序列化 | 不做对战逻辑 |
| view | 可视化 | 不做计算 |
| replay | 回放记录 | 不做对战 |

每个 crate 只有一个职责。改 mechanics 不会意外影响 format 的序列化。

### 4. 共享依赖统一版本

```toml
[workspace.dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

workspace 级别的依赖声明——所有 crate 用同一个 `serde` 版本。没有"crate A 用 serde 1.5、crate B 用 serde 1.8"的不一致。

## 和 workshop 的对比

| 维度 | shamrock | workshop |
|------|---------|----------|
| 拆分方式 | 8 crate 按功能域 | 4 crate 按层 (domain/app/infra/cli) |
| 依赖规则 | data ← core ← mechanics ← ... | domain ← application ← infra ← cli |
| 共性 | 单向依赖，核心不含 UI | 单向依赖，核心不含 UI |

两种拆分策略：按功能域 vs 按架构层。都是有效的——选哪种取决于功能域之间是否独立。

## 来源

- shamrock 源码（`Cargo.toml`、`crates/` 目录结构）
- 2026-06-07 agent 阅读后提炼
