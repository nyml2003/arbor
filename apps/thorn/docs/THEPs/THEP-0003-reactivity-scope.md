---
id: THEP-0003
title: "响应式与 Scope"
status: Accepted
created: 2026-07-07
updated: 2026-07-08
area: reactivity
---

# THEP-0003: 响应式与 Scope

## Summary

Thorn 采用 Solid 风格的细粒度响应式。

组件函数只执行一次。组件函数负责创建信号、派生值、副作用和 view 结构。后续状态变化不重新执行组件函数，只更新依赖该状态的 primitive slot。

## Decision

核心原语：

- `Signal<T>`
- `ReadSignal<T>`
- `Memo<T>`
- `Effect`
- `Scope`

第一阶段使用单线程模型：

- `Rc`
- `RefCell`
- thread-local active observer stack

不使用跨线程 atomic signal。

Signal 行为：

1. `get()` 读取当前值。
2. `get()` 如果发生在 active effect 中，自动记录依赖。
3. `set()` 或 `update()` 只通知依赖它的 effect。
4. 同值写入不触发 effect，前提是 `T: PartialEq`。
5. signal 不直接 render。

Effect 行为：

1. effect 首次创建时立即执行。
2. effect 执行前清除上一轮依赖。
3. effect 执行期间读取的 signal 会成为新依赖。
4. signal 变化后 effect 同步重新执行。
5. effect 可以注册 cleanup。

Memo 行为：

1. memo 是派生值。
2. memo 依赖 signal 或其他 memo。
3. 依赖变化时 memo 标记 stale。
4. 下次读取时重新计算。
5. 输出值变化时通知下游依赖。

Scope 行为：

1. 每个组件 mount 时创建一个 scope。
2. `Show` 的每个活动分支有独立 scope。
3. `For` 的每个 keyed item 有独立 scope。
4. scope dispose 时清理内部 effect、memo、子 scope 和 cleanup。
5. scope dispose 后，内部 effect 不应再响应 signal。

组件生命周期映射：

| Solid 概念 | Thorn 对应 |
| --- | --- |
| 组件函数执行一次 | mount component scope |
| signal 更新 | effect 更新 primitive slot |
| cleanup | scope dispose |
| conditional branch unmount | branch scope dispose |
| keyed list item unmount | item scope dispose |

## Non-goals

- 不做 React 式组件重渲染。
- 不做虚拟 DOM diff。
- 不在第一阶段做异步调度。
- 不做 priority、transition、concurrent rendering。
- 不允许 render 阶段创建终端副作用。
- 不要求 Signal 跨线程可写。

## API Impact

建议 API：

```rust
let count = cx.create_signal(0usize);

let doubled = cx.create_memo(move || count.get() * 2);

cx.create_effect(move || {
    let value = doubled.get();
    // 更新 primitive slot，或执行受控副作用。
});

count.update(|value| *value += 1);
```

组件 API：

```rust
fn counter(cx: &Scope) -> View<Action> {
    let count = cx.create_signal(0usize);

    col((
        text("Counter"),
        text(move || format!("count: {}", count.get())),
    ))
}
```

约束：

- 组件只能通过 `Scope` 创建响应式资源。
- `ReadSignal<T>` 可以传给子组件。
- 子组件不能写父组件的 read signal。
- 可观察 UI 状态优先使用 signal。

## Test Requirements

必须测试：

- signal 初始读取。
- signal set 触发依赖 effect。
- 同值 set 不触发 effect。
- effect rerun 会清理旧依赖。
- effect 嵌套时 active stack 恢复正确。
- scope dispose 后 effect 不再执行。
- scope cleanup 按创建逆序执行。
- memo 首次读取计算。
- memo 依赖不变时复用缓存。
- memo 依赖变化后惰性重算。
- `Show` 分支切换会 dispose 旧 scope。
- `For` 删除 item 会 dispose 对应 scope。
