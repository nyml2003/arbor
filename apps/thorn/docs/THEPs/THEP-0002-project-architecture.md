---
id: THEP-0002
title: "项目架构"
status: Accepted
created: 2026-07-07
updated: 2026-07-08
area: architecture
---

# THEP-0002: 项目架构

## Summary

Thorn 是新的 Rust TUI 框架实验。

Thorn 不兼容 `arbor-tui`。它不迁移旧 widget，不提供 legacy adapter。它只参考 `arbor-tui` 中已经证明有效的底层经验，例如 cell grid、row diff、宽字符处理、模拟后端和布局测试。

## Decision

Thorn 放在：

```text
apps/thorn/
```

第一阶段使用独立 Rust workspace。默认 crate：

```text
apps/thorn/
  Cargo.toml
  crates/
    thorn-core/
    thorn-terminal/
    thorn/
  docs/
    THEPs/
```

crate 责任：

| Crate | 责任 |
| --- | --- |
| `thorn-core` | 纯核心 crate。内部按领域模块组织响应式、view、布局、主题、渲染、基础组件和测试 harness |
| `thorn-terminal` | crossterm 终端适配、raw mode、alternate screen、输入事件、resize |
| `thorn` | 用户入口、`prelude`、高层运行 API |

`thorn-core` 不是一个平铺大模块。它必须按领域分组：

```text
thorn-core/src/
  reactive/    Signal、Memo、Effect、Scope
  view/        View、PrimitiveNode、NodeId、NodeKey、动态文本和样式
  layout/      Rect、Size、Edge、FlexBox 子集、layout tree
  theme/       Theme、Token、Color、style resolution
  render/      Cell、Screen、DirtyRegion、diff、screen compose
  widgets/     Text、Row、Col、Panel，后续再按 THEP 增加 Input、Show、For
  testing/     TestApp、screen assertions、simulated backend helpers
  lib.rs       public exports and prelude-facing surface
```

领域依赖方向：

```text
widgets -> view -> reactive
widgets -> layout
widgets -> theme
render  -> layout
render  -> theme
testing -> all core domains
```

约束：

- `reactive` 不依赖 view、layout、theme、render 或 widgets。
- `view` 可以依赖 reactive，但不依赖 terminal。
- `layout` 只处理几何和 FlexBox 子集，不读取 theme。
- `theme` 不依赖 layout 和 render。
- `render` 可以解析 theme 后写 cell，但不处理输入事件。
- `widgets` 只能组合 core 领域能力，不拥有 terminal backend。
- `testing` 可以组合 core 领域，但不能成为生产代码依赖。

依赖规则：

- `thorn-core` 不能依赖 crossterm。
- `thorn-core` 不能访问真实终端。
- `thorn-core` 不能持有平台资源。
- `thorn-terminal` 只能消费 core 的输出和端口。
- `thorn` 只做 facade，不写核心协议。

Thorn 直接重做这些层：

- 组件模型。
- 响应式模型。
- view/primitive tree。
- 布局协议。
- 主题系统。
- 终端 runtime。

Thorn 不继承这些 `arbor-tui` 概念：

- `Widget trait`
- `WidgetNode`
- `WidgetFactory`
- `PropsRevision`
- `SignalDeps` 手写协议
- legacy adapter
- 每个 widget 自己实现 mount/update/render 全套协议

交互协议由 THEP-0010 约束。当前阶段不把 `Button`、鼠标事件或 click handler 放进 core。

可以参考并重写这些能力：

- `Cell`
- `VirtualScreen`
- row diff
- `Rect` / `Size` / `RectOffset`
- 宽字符处理
- light theme 默认背景测试
- simulated backend

## Non-goals

- 不做 `arbor-tui` 的下一版。
- 不保留 `arbor-tui` API。
- 不让旧项目无改动迁移。
- 不在第一阶段拆出过多 crate。
- 不为了未来发布提前做通用 crate 包装。

## API Impact

用户只从 facade 入口导入：

```rust
use thorn::prelude::*;
```

普通用户不直接感知：

- terminal backend。
- raw mode。
- diff regions。
- retained primitive slot。
- effect graph 内部结构。
- core 内部领域模块边界。

高层入口形态：

```rust
fn main() -> thorn::Result<()> {
    thorn::app(root)
        .theme(Theme::dark())
        .run()
}
```

具体签名在实现阶段确定，但必须保持这条边界：应用写 view，runtime 处理终端。

## Test Requirements

架构层测试要求：

- `thorn-core` 单独 `cargo test` 不需要真实终端。
- `thorn-core` 每个领域模块要有自己的单元测试。
- `thorn-core::testing` 只做测试入口，生产模块不能依赖它。
- `thorn-terminal` 测试可以使用 simulated backend 或 crossterm adapter 的小范围单测。
- facade 测试只验证导出 API 和 smoke path。
- CI 或本地验证优先运行：

```powershell
cargo test --manifest-path apps/thorn/Cargo.toml --workspace
cargo check --manifest-path apps/thorn/Cargo.toml --workspace
```
