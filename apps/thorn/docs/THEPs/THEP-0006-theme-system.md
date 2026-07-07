---
id: THEP-0006
title: "主题系统"
status: Accepted
created: 2026-07-07
updated: 2026-07-08
area: theme
---

# THEP-0006: 主题系统

## Summary

Theme 是 Thorn 的一等系统。

组件默认使用语义 token，不直接写 palette index。Theme 由 runtime context 提供。Theme 可以静态设置，也可以由 signal 驱动切换。

## Decision

Theme 不是全局 singleton。

Runtime 持有当前 theme：

```rust
Theme
```

后续可以支持：

```rust
Signal<Theme>
```

颜色类型：

```rust
enum Color {
    Palette(u8),
    Rgb { r: u8, g: u8, b: u8, fallback: u8 },
}
```

主题 token：

```rust
enum Token {
    Surface,
    SurfaceAlt,
    Text,
    TextMuted,
    Border,
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Focus,
    Selection,
}
```

style color source：

```rust
enum ColorSource {
    Token(Token),
    Color(Color),
}
```

内置主题：

- `Theme::dark()`
- `Theme::light()`
- `Theme::high_contrast()`

Theme 覆盖：

```rust
Theme::dark()
    .with(Token::Accent, Color::Palette(73))
    .with(Token::Surface, Color::Palette(0))
```

组件默认 token：

| 组件 | fg | bg | border/focus |
| --- | --- | --- | --- |
| `Text` | `Text` | inherited or `Surface` |
| `Panel` | `Text` | `SurfaceAlt` | `Border` |
| `Input` | `Text` | `SurfaceAlt` | `Focus` cursor |

背景规则：

1. root 默认填充 `Surface`。
2. 可见容器默认填充自己的 bg。
3. transparent 必须显式声明。
4. light theme 不能漏默认黑底。
5. 文本 cell 必须同时写 fg 和 bg。

Theme 更新规则：

- color token 变化触发 render。
- MVP 不做 spacing token。
- 如果后续加入 spacing/gap/padding token，它们必须触发布局。

## Non-goals

- 不实现 CSS 变量系统。
- 不实现 cascade。
- 不实现复杂 selector。
- 不实现动态主题文件加载。
- 不在 MVP 中做 spacing token。
- 不自动做 RGB 到 256 色映射。fallback 由 theme 提供。

## API Impact

用户 API：

```rust
thorn::app(root)
    .theme(Theme::dark())
    .run()
```

组件样式：

```rust
text("Ready").fg(Token::Success);
panel(body).bg(Token::SurfaceAlt).border(Token::Border);
```

允许 concrete color：

```rust
text("debug").fg(Color::Palette(244));
```

但文档推荐默认使用 token。

## Test Requirements

必须测试：

- dark theme token 解析。
- light theme token 解析。
- high contrast token 解析。
- token override 生效。
- concrete color 不走 token。
- root 背景填满 screen。
- panel 背景填满 rect。
- text 写入 fg/bg。
- light theme 下没有可见黑底泄漏。
- theme signal 切换后触发 render 更新。
- unknown token 不允许出现；编译期 enum 应覆盖。
