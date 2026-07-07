---
id: THEP-0005
title: "FlexBox 子集布局"
status: Accepted
created: 2026-07-07
updated: 2026-07-07
area: layout
---

# THEP-0005: FlexBox 子集布局

## Summary

Thorn 只实现 FlexBox 子集。

布局单位是终端 cell。布局目标是稳定、可测试、足够写 TUI，不追求完整 CSS。所有布局规则必须能用整数 cell 解释。

## Decision

布局输入：

```text
PrimitiveTree + root Rect -> LayoutTree
```

基础类型：

```rust
struct Rect { x: u16, y: u16, w: u16, h: u16 }
struct Size { w: u16, h: u16 }
struct Edge { top: u16, right: u16, bottom: u16, left: u16 }
```

支持属性：

| 属性 | 说明 |
| --- | --- |
| `direction` | `Row` 或 `Column` |
| `width` | 固定列数 |
| `height` | 固定行数 |
| `min_width` | 最小列数 |
| `min_height` | 最小行数 |
| `flex` | grow 权重 |
| `gap` | children 之间的固定间距 |
| `padding` | 容器内边距 |
| `margin` | child 外边距 |
| `justify` | 主轴分布 |
| `align` | 交叉轴分布 |

`justify` 支持：

- `Start`
- `Center`
- `End`
- `SpaceBetween`

`align` 支持：

- `Start`
- `Center`
- `End`
- `Stretch`

Flex 规则：

1. fixed size 先分配。
2. gap 和 margin 先扣除。
3. 剩余主轴空间按 `flex` 权重分配。
4. `flex == 0` 的 child 只拿 intrinsic/fixed size。
5. `flex > 0` 的 child 可以 grow。
6. MVP 不实现完整 shrink。空间不足时按最小尺寸裁切或压缩到 1 cell。
7. 余数按 child 顺序分配，保证确定性。

测量规则：

1. leaf 可以返回 intrinsic size。
2. text intrinsic width 使用显示宽度，不使用 byte length。
3. container 的 intrinsic size 来自 children。
4. padding、margin、gap 都参与 container intrinsic size。

`measure` 边界：

- MVP 有内部 measure 阶段。
- 内部 measure 由 primitive/layout domain 使用。
- `Text`、`Button`、`Panel`、`Row`、`Col` 等内置节点可以提供 intrinsic size。
- 应用层函数组件不实现 `measure`。
- 应用层函数组件只能组合公开 builder，让底层 primitive 参与测量。
- 应用层自定义 intrinsic measure 需要后续 custom primitive THEP。

渲染规则：

- 每个可见 node 拿到一个确定 rect。
- rect 可以是 0 宽或 0 高，但 renderer 必须安全跳过。
- 默认组件应填满自己的 rect 背景。

`on_layout` 边界：

- MVP 不开放同步 `on_layout`。
- 测试 harness 可以读取 layout info。
- 后续可以开放只读的 after-layout hook。
- after-layout hook 只能观察最终 rect。
- after-layout hook 不能修改当前 frame 的 layout。
- after-layout hook 如果写 signal，必须进入下一帧。

后续 hook 形态可以是：

```rust
view.on_layout(|info| {
    // info.rect 是本帧最终布局结果。
    // 这里不能重入 layout。
})
```

禁止语义：

```text
on_layout writes signal -> same frame re-layout
```

允许语义：

```text
on_layout writes signal -> schedule next frame
```

## Non-goals

不实现：

- flex wrap。
- 完整 flex shrink。
- flex basis 百分比。
- order。
- absolute position。
- z-index。
- baseline。
- CSS cascade。
- auto overflow layout。
- min-content/max-content 完整规则。
- gap 百分比。
- 像素布局。
- 应用层自定义 measure。
- 同步 on_layout。
- on_layout 触发当前帧重布局。

## API Impact

建议 API：

```rust
col((
    text("Header").height(1),
    row((
        panel(task_list()).width(24),
        panel(transcript()).flex(1),
    ))
    .gap(1)
    .flex(1),
    input().height(1),
))
.padding(1)
.bg(Token::Surface)
```

布局 builder 方法：

- `.row()`
- `.col()`
- `.width(u16)`
- `.height(u16)`
- `.min_width(u16)`
- `.min_height(u16)`
- `.flex(u16 | f32)`
- `.gap(u16)`
- `.padding(u16 | Edge)`
- `.margin(u16 | Edge)`
- `.justify(Justify)`
- `.align(Align)`

默认值：

- `direction = Column`
- `flex = 0`
- `gap = 0`
- `padding = 0`
- `margin = 0`
- `justify = Start`
- `align = Stretch`

MVP 不暴露这些 API：

```rust
fn measure(...)
fn on_layout(...)
```

测试 API 可以暴露 layout info：

```rust
let layout = app.layout_of(node_id);
assert_eq!(layout.rect.w, 24);
```

后续如果开放 `on_layout`，必须是 after-layout callback，不能是 layout 计算的一部分。

## Test Requirements

必须测试：

- row 按 x 轴排列 children。
- col 按 y 轴排列 children。
- fixed width/height 生效。
- `flex` child 获得剩余空间。
- 多个 flex child 按权重分配。
- 余数分配确定。
- gap 扣除空间。
- padding 缩小 content rect。
- margin 推开 child rect。
- `justify Center/End/SpaceBetween` 生效。
- `align Start/Center/End/Stretch` 生效。
- 空间不足不会 panic。
- 宽字符文本测量正确。
- light theme 下布局空白区域有背景填充。
- 测试 harness 可以读取节点 layout info。
- 应用层函数组件不能自定义当前帧 measure。
