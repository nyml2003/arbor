# 模式：Vue3 水平虚拟滚动（visual-frame）

## 一句话

用"前后占位空白 + 可见窗口 + buffer"实现水平虚拟滚动——不是库，是 120 行的 Vue3 组件。核心原理：scroll 位置 / itemWidth = startIndex，只渲染窗口内的元素。

## 为什么需要自己实现

虚拟滚动库（如 `@tanstack/virtual`）做了一堆通用抽象——水平/垂直、动态高度、overscan、滚动到指定项。但如果你只需要固定宽度的水平虚拟滚动（如文件浏览器、图片画廊、时间轴），120 行 Vue3 组件比一个库更可控、更可调试。

## 核心原理

```
scrollLeft / itemWidth = startIndex

总宽度：itemCount * itemWidth

┌──────────────────────────────────────────────┐
│ [前占位] │ item0 │ item1 │ item2 │ [后占位] │  ← 可见窗口（buffer 各加 2）
│  start*W  │  W   │  W   │  W   │ 剩余*W    │
└──────────────────────────────────────────────┘
```

只有窗口内的 item 在 DOM 里——占位用 flexShrink: 0 的固定宽度 div。

## 关键实现

### 1. 滑动 → startIndex

```typescript
const handleScroll = () => {
  const scrollLeft = containerRef.value.scrollLeft;
  const newIndex = Math.round(scrollLeft / props.itemWidth);
  if (newIndex !== startIndex.value) {
    startIndex.value = newIndex;
  }
};
```

`Math.round` 不是 `Math.floor`——避免在 item 边界附近频繁切换。

### 2. Buffer 渲染

```typescript
const bufferCount = 2;
const calculateVisibleItems = () => {
  const start = Math.max(0, startIndex.value - bufferCount);
  const end = Math.min(itemSize.value, startIndex.value + visibleCount + bufferCount);
  return { visibleItems: indexedItems.value.slice(start, end), start, end };
};
```

Buffer 防止快速滑动时出现空白闪烁。bufferCount = 2 意味着左右各多渲染 2 个 item。

### 3. 占位空白

```tsx
{/* 前占位 */}
<div style={{ width: `${start * itemWidth}px`, flexShrink: 0 }} />

{/* 可见项 */}
{visibleItems.map(item => <div style={{ width: `${itemWidth}px`, flexShrink: 0 }}>
  {props.renderItem(item.item)}
</div>)}

{/* 后占位 */}
<div style={{ width: `${Math.max(0, itemCount - end) * itemWidth}px`, flexShrink: 0 }} />
```

前后占位保证容器总宽度 = `itemCount * itemWidth`，滚动条位置始终正确。`flexShrink: 0` 防止占位 div 被 flex 压缩。

### 4. renderItem prop：控制反转

```typescript
renderItem: {
  type: Function as PropType<(item: any) => JSX.Element>,
  required: true,
}
```

组件不关心 item 渲染成什么——调用方通过 `renderItem` 注入渲染逻辑。这和 React 的 `render prop` 模式一样。

### 5. 生命周期管理

```typescript
onMounted(() => container.addEventListener("scroll", handleScroll));
onBeforeUnmount(() => container.removeEventListener("scroll", handleScroll));
```

只做 scroll 监听——没有 ResizeObserver、没有 IntersectionObserver、没有 requestAnimationFrame 节流。对固定宽度水平滚动，这就够了。

## 性能特征

| 场景 | 实际渲染 DOM 数 |
|------|---------------|
| 10000 项，可见 10 项，buffer 2 | ~14 项 |
| 不滚动 | 不触发 re-render |

不滚动时不触发 re-render——`startIndex` 不变，`calculateVisibleItems` 返回相同引用。

## 反模式警示

### ❌ 用 `position: absolute` + `left` 定位

`left` 频繁变更会触发布局（layout）。flex + 占位 div 只触发 paint，比 absolute 定位快。

### ❌ 动态高度

固定 item 宽度是这个实现的前提。如果需要动态宽度，需要预先测量或用 ResizeObserver，复杂度翻倍。但如果你不需要——别做。

## 来源

- visual-frame 源码（`src/HorizontalVirtualScrollList.tsx`）
- 2026-06-07 agent 阅读后提炼
