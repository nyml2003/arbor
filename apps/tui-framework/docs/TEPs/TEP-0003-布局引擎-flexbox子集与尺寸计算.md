---
id: TEP-0003
title: "布局引擎——flexbox 子集与尺寸计算"
status: Review
created: 2026-07-04
updated: 2026-07-04
author: nyml
area: layout
affects: []
related: [TEP-0001, TEP-0002]
---

# TEP-0003: 布局引擎——flexbox 子集与尺寸计算

## 摘要

手写 flexbox 子集布局引擎。两遍算法：bottom-up measure（叶子报约束）→ top-down layout（父级分配空间）。全部纯函数，放 domain/。不引入 taffy 或其他外部布局库。

## 目标

给定一棵 Widget 树和终端尺寸，计算出每个 Widget 在字符网格上的确切坐标和两个矩形（外层占位 + 内部内容），交给渲染层的 `render(content_rect)` 填入内容。

## 坐标系统

```rust
// domain/layout.rs

/// 字符网格上的矩形区域
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

/// 尺寸
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Size {
    pub w: u16,
    pub h: u16,
}

/// 四边偏移（用于 padding 和 margin）
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct RectOffset {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

/// Widget 唯一标识
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct WidgetId(pub u64);

/// 布局遍历用节点包装
pub struct WidgetNode {
    pub id: WidgetId,
    pub widget: Box<dyn Widget>,
}
```

终端坐标原点 (0, 0) 是左上角。x 向右递增（列），y 向下递增（行）。与 ANSI `CSI n;m H` 的光标定位一致（框架内部用 x=col, y=row）。

## Flexbox 子集

支持六个属性。不支持 wrap、gap、align-self、order、min/max-width/height、百分比尺寸。

```rust
pub enum Direction {
    Row,
    Column,
}

pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

pub struct LayoutProps {
    pub direction: Direction,
    pub justify: Justify,
    pub align: Align,
    pub flex: f32,                     // 弹性比例，0.0 = 不弹性
    pub width: Option<u16>,            // 固定内容区宽度
    pub height: Option<u16>,           // 固定内容区高度
    pub padding: RectOffset,
    pub margin: RectOffset,
}
```

### 裁剪理由

- **wrap**：终端屏幕小，单行组件 3-5 个。换行后交叉轴计算复杂度加倍，可用嵌套 Box 替代多行排列
- **gap**：padding + margin 组合已覆盖间距需求
- **order / align-self**：终端 UI 组件按逻辑顺序排列，重排场景极少。align-self 可通过嵌套 Box + Align 实现
- **min/max-width/height**：终端网格上组件的尺寸约束空间远小于 Web，flex 弹性分配已足够处理大多数场景。v1 不做，后续 TEP 按需扩展
- **百分比**：v1 不做。弹性区域用 `flex: 1`，窗口缩放由 SIGWINCH 全树重算

## 标准盒模型

终端字符网格上的盒模型规则——强制写入，避免实现偏差：

```
┌─────────────────────────────────────┐
│            margin (外间距)           │
│  ┌───────────────────────────────┐  │
│  │        padding (内间距)        │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │    content (内容区)      │  │  │
│  │  │    render 填充区域       │  │  │
│  │  └─────────────────────────┘  │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

规则：
1. **padding** 占用组件内部空间，子组件排列在 padding 内侧
2. **margin** 用于兄弟组件间距，不侵占自身内容区
3. 子组件可用尺寸 = 父内容区尺寸 - 子 margin
4. 固定 `width`/`height` 定义的是**内容区尺寸**，不含 padding/margin
5. 组件外部占位 = 内容区 + padding + margin

## SizeConstraint

```rust
/// 主轴或交叉轴的单轴约束
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum AxisConstraint {
    Fixed(u16),      // 有确定上限
    Unbounded,       // 无上限（如 Text 在未分配宽度前自由测量）
}

#[derive(Copy, Clone)]
pub struct SizeConstraint {
    pub min_w: u16,
    pub min_h: u16,
    pub max_w: AxisConstraint,
    pub max_h: AxisConstraint,
}

impl SizeConstraint {
    /// 全有界约束——父级已分配空间后使用
    pub fn bounded(available: Size) -> Self {
        Self {
            min_w: 0,
            min_h: 0,
            max_w: AxisConstraint::Fixed(available.w),
            max_h: AxisConstraint::Fixed(available.h),
        }
    }

    /// 无上限约束——叶子组件自由测量固有尺寸
    pub fn unbounded() -> Self {
        Self {
            min_w: 0,
            min_h: 0,
            max_w: AxisConstraint::Unbounded,
            max_h: AxisConstraint::Unbounded,
        }
    }
}
```

用 `AxisConstraint` 替代魔术值 `u16::MAX`——避免加减溢出，语义上明确区分"有界"和"无界"。

## 两遍布局算法

### Widget trait（布局引擎可见的子集）

```rust
// domain/widget.rs —— 布局引擎只依赖这几个方法

pub trait Widget {
    fn id(&self) -> WidgetId;
    fn layout_props(&self) -> &LayoutProps;
    fn children(&self) -> &[WidgetNode];

    /// measure：报告尺寸约束
    /// available = 父扣除 padding 后，当前组件包含自身 margin 在内的最大可用区域
    /// 实现内部应先减去自身 margin 得到内容区可用尺寸，再计算固有尺寸
    fn measure(&self, available: Size) -> SizeConstraint;
}
```

布局引擎不关心 Widget 的内部实现。只遍历它，不修改它。

### Pass 1: Bottom-up measure

从叶子到根，每个 widget 报告 `SizeConstraint`：

```
measure(widget, available: Size) → SizeConstraint
```

典型组件的 measure 行为：

| 组件 | min | max |
|------|-----|-----|
| Text("hello", Wrap::None) | (5, 1) | (5, 1) |
| Text(long_text, Wrap::Char) | (1, 1) | (available.w, ceil(chars/available.w)) |
| Input | (1, 1) | (available.w, 1) |
| Button | (len(label)+2, 1) | (available.w, 1) |
| Box(children) | 按 direction 累加 children constraints | 同 min（布局容器尺寸由子项决定） |
| List | (max_item_w, 1) | (max_item_w, available.h) |

Text measure 特殊情况——换行模式：
- `Wrap::None`：`min = max = (text_width, 1)`
- `Wrap::Char`：`min = (1, 1)`，`max = (available.w, ceil(chars / available.w))`
- `Wrap::Word`：西文按空格分词后同上计算；CJK 自动 fallback 到 Char

所有组件 min 强制下限 1——不允许零尺寸不可见组件。

### Pass 2: Top-down layout

从根到叶子，父级根据 flex 规则分配空间。

**公开 API**：

```rust
/// Pass 1：自底向上测量整树约束
pub fn measure_tree(root: &dyn Widget, screen_size: Size) -> HashMap<WidgetId, SizeConstraint>;

/// Pass 2：自顶向下分配坐标，输出所有组件布局信息
pub fn layout_tree(
    root_rect: Rect,
    root: &dyn Widget,
    constraints: &HashMap<WidgetId, SizeConstraint>,
) -> LayoutResult;
```

### Flex 主轴分配算法（完整）

以 `Direction::Column`（主轴=高度，交叉轴=宽度）为例。Row 仅交换 w ↔ h。

**Step 1：扣除内边距**

```
main_available = rect.h - padding.top - padding.bottom
cross_available = rect.w - padding.left - padding.right
```

**Step 2：遍历子项，分类收集**

```
每个子项：
  props = child.layout_props()
  子项主轴 margin = props.margin.top + props.margin.bottom
  子项固有主轴尺寸 = constraints[child.id()].min_h + 子项主轴 margin

fixed_total   = sum(所有 flex=0 子项的固有主轴尺寸)
flex_sum      = sum(所有 flex>0 子项的 flex 值)
```

**Step 3：计算可分配弹性空间**

```
free_space = main_available - fixed_total
```

**Step 4：空间分配分支**

```
if free_space >= 0:
    // 空间充足：弹性子项按 flex 比例瓜分
    每个弹性子项 base = floor(free_space * flex_i / flex_sum)
    // 余数分配：按 flex 权重依次补给靠前的弹性子项，填满可用空间
    remainder = free_space - sum(base)
    按 flex 降序排列弹性子项，前 remainder 个子项各 +1

if free_space < 0:
    // 空间不足：压缩弹性子项，不低 min 约束
    每个弹性子项压缩 = floor(|free_space| * flex_i / flex_sum)
    弹性子项最终主轴 = max(min约束主轴, 固有主轴 - 压缩)
```

**Step 5：主轴排布（Justify）**

```
children_total = sum(所有子项最终主轴尺寸)
main_gap = main_available - children_total

Justify::Start   → offset = 0，依次向下排列
Justify::Center  → offset = main_gap / 2
Justify::End     → offset = main_gap
Justify::SpaceBetween:
    子项数 ≥ 2  → 间隙 = main_gap / (子项数 - 1)，依次排布
    子项数 < 2  → 退化为 Start
```

**Step 6：交叉轴尺寸与对齐（Align）**

```
每个子项：
  cross_margin = props.margin.left + props.margin.right
  avail = cross_available - cross_margin

  Align::Stretch → 子项交叉轴 = avail
  Align::Start   → 子项交叉轴 = constraints.min_w，靠左排列
  Align::Center  → 子项交叉轴 = constraints.min_w，居中排列（offset = (avail - min_w) / 2）
  Align::End     → 子项交叉轴 = constraints.min_w，靠右排列
```

## LayoutResult

渲染层需要两套坐标——外层占位用于兄弟间距，内层内容区用于 `render()`：

```rust
pub struct WidgetLayoutInfo {
    pub id: WidgetId,
    /// 外层占位矩形（含 margin）
    pub outer_rect: Rect,
    /// 内部内容矩形（扣除 padding，render 的绘制区域）
    pub content_rect: Rect,
}

pub struct LayoutResult {
    pub widgets: Vec<WidgetLayoutInfo>,
}
```

## SIGWINCH 重排

终端 resize → 全树重新 measure + layout，不做增量。

```
SIGWINCH → crossterm 的 Resize(w, h) event
  → measure_tree(root, Size(w, h))
  → layout_tree(Rect(0,0,w,h), root, &constraints)
  → 全屏 diff + emit
```

## ScrollView 视口裁切

ScrollView 的子组件可能远大于视口（如 10,000 行 List）。布局与渲染的裁切规则：

### measure 阶段

ScrollView 将视口尺寸作为 `available` 传给子组件的 `measure()`。子组件报告的自然尺寸可能大于视口——这不影响布局，只影响后续的滚动范围。

```
scroll_view.measure(available):
  child_available = available - scroll_view.padding  // 视口大小
  child_constraint = child.measure(child_available)   // 子组件自然尺寸
  返回 child_constraint（不做裁剪——布局引擎需要完整尺寸来算滚动范围）
```

### layout 阶段

子组件按自然尺寸接收 `content_rect`——不裁切坐标。`scroll_offset` signal 控制视口内的可见区域。

### render 阶段

ScrollView 只把视口内的 Cell 拷贝到全局 VirtualScreen。视口外的子组件区域不参与全局渲染，不产生 diff 脏区。

```
scroll_view.render(content_rect):
  child_screen = child.render(child_content_rect)    // 子组件全量渲染
  visible = child_screen.clip(scroll_offset, viewport_size)  // 拷贝视口区域
  return visible
```

List/Table 利用此规则做虚拟滚动：`render_item` 只渲染 `scroll_offset..scroll_offset+viewport_height` 范围内的可见行。

理由：终端 resize 是窗口拖拽——全屏尺寸都变了，增量重排节省不了什么，但引入脏区管理复杂度远超收益。

## SizeCalc 统一尺寸计算工具

所有尺寸运算（margin 扣除、padding 扣除、可用空间计算）集中到 `SizeCalc` 工具，避免分散实现漏写饱和减法：

```rust
// domain/layout.rs

/// 统一尺寸计算——所有布局运算的唯一入口
pub struct SizeCalc;

impl SizeCalc {
    /// 计算内容区可用尺寸：容器 - padding - margin
    pub fn content_available(container: Size, padding: RectOffset, margin: RectOffset) -> Size {
        Size {
            w: sat_sub(container.w, padding.left + padding.right + margin.left + margin.right),
            h: sat_sub(container.h, padding.top + padding.bottom + margin.top + margin.bottom),
        }
    }

    /// 子组件外层占位尺寸：内容 + padding + margin
    pub fn outer_size(content: Size, padding: RectOffset, margin: RectOffset) -> Size {
        Size {
            w: content.w + padding.left + padding.right + margin.left + margin.right,
            h: content.h + padding.top + padding.bottom + margin.top + margin.bottom,
        }
    }

    /// 饱和减法——不允许下溢为超大值
    fn sat_sub(a: u16, b: u16) -> u16 {
        if a > b { a - b } else { 0 }
    }
}
```

布局引擎内部所有尺寸计算必须走 `SizeCalc`，禁止手写 `w - padding.left - padding.right`。编译期通过 code review 强制执行。

弹性分配后尺寸不能低于自身 min 约束。压缩空间时以 min 为底线。

## 已决议的开放问题

- [x] **SpaceBetween 单子项行为**：子项数 < 2 时退化为 `Justify::Start`
- [x] **百分比尺寸**：v1 不做，全用绝对值 + flex。后续独立 TEP 扩展
- [x] **CJK 换行**：`Wrap::Char` 严格按字符断开；`Wrap::Word` 对 CJK 自动 fallback 到 `Wrap::Char`
- [x] **所有组件 min 强制 ≥ 1**：不允许零尺寸不可见组件

## 附录 A：测试用例清单（验收标准）

1. 单列固定高度 + flex 弹性子组件均分剩余高度（free_space ≥ 0）
2. 子项固有总和超父可用空间，验证 flex 压缩至 min 约束（free_space < 0）
3. Justify::SpaceBetween 多组件间隙均匀 / 单组件退化为 Start
4. Align::Stretch / Center / End 交叉轴对比
5. Text 无换行、Word 换行、CJK Char 换行，三种 measure 结果校验
6. 窗口 SIGWINCH 缩小，子组件压缩至 min 约束，不下溢
7. 三层嵌套 Box，每层不同 padding + margin，盒模型坐标校验
8. 零弹性子项（全部 flex=0），验证 fixed_total = 总占用

## 附录 B：工程建议

1. 全部纯函数，无全局状态、无缓存。每个 `layout_tree()` 调用从头计算
2. `SizeConstraint`、`Rect`、`Size` 全部 `Copy`，栈分配，零堆开销
3. 实现顺序：固定尺寸 → flex 弹性 → 文本换行 measure → SIGWINCH 联动
4. 后续特性（百分比、gap、min/max 约束）通过独立 TEP 追加，不阻塞当前引擎

## 参考

- [Yoga Layout](https://yogalayout.com/) — Facebook flexbox 实现
- [taffy](https://github.com/DioxusLabs/taffy) — Rust flexbox 库
- [CSS Flexbox Spec](https://www.w3.org/TR/css-flexbox-1/) — W3C 标准
