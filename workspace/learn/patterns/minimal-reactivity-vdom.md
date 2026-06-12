# 模式：最小化响应式 + VDOM 系统（jue）

## 一句话

一个完整的响应式依赖追踪 + Virtual DOM diff/patch 系统，用 ~800 行 TypeScript 实现。读懂它，就懂了 Vue/SolidJS 的底层原理。

## 为什么这个模式值得留

jue 是一个学习性质的实验项目，但它做到了工业级框架的核心功能：

1. **看懂它 = 看懂所有响应式框架的骨架**
2. 代码质量高——属性处理、事件管理、边界条件都有考虑
3. 800 行 TS 是最小的可理解单元，比读 Vue/React 源码快 100 倍

## 核心架构

```
Reactivity（单例）
  │
  ├── reactive(obj)        Proxy 包装，get 时自动收集依赖
  ├── effect(fn)           副作用函数，run 时自动追踪
  ├── Dep 类               依赖容器（Set of effects）
  │   ├── depend()         将当前 activeEffect 加入 subscribers
  │   ├── notify()         触发所有 subscribers 重新运行
  │   └── remove()         清理
  └── targetMap            WeakMap<obj, Map<key, Dep>>
                              ↑ 全局依赖图


VDOM
  │
  ├── createElement(vnode) 虚拟 DOM → 真实 DOM
  ├── patch(el, patches)   差异应用（6 种 patch 类型）
  └── patchChildren()      子节点批量更新
```

## 关键实现

### 1. 依赖追踪：WeakMap + Dep

```
WeakMap<target, Map<key, Dep>>
  │              │        │
  │              │        └── Dep { subscribers: Set<EffectFn> }
  │              │             - depend(): 当前 effect 加入 subscribers
  │              │             - notify(): 所有 subscriber 重新运行
  │              └── key → Dep 的映射
  └── 对象级别的 Map（键是对象引用，可 GC）
```

**读取时自动收集**：
```typescript
// Proxy get 陷阱
get(target, key, receiver) {
  const dep = self.getDep(target, key);    // ← 拿到 key 对应的 Dep
  dep.depend(self.activeEffect);          // ← 当前 activeEffect 加入 subscribers
  return Reflect.get(target, key, receiver);
}
```

**写入时自动通知**：
```typescript
// Proxy set 陷阱
set(target, key, value, receiver) {
  const result = Reflect.set(target, key, value, receiver);
  const dep = self.getDep(target, key);
  dep.notify();                           // ← 通知所有订阅者
  return result;
}
```

### 2. Effect 栈管理

```typescript
effect(fn) {
  const effectFn = () => {
    cleanupEffect(effectFn);             // ← 先清理上一次的依赖
    this.effectStack.push(effectFn);     // ← 压栈（支持嵌套 effect）
    this.activeEffect = effectFn;         // ← 设为当前活跃
    const result = fn();                  // ← 运行 → get 时自动收集
    // ... pop stack, restore activeEffect
  };
  effectFn();                             // ← 立即运行一次
}
```

**关键**：`effectStack` 支持嵌套 effect（一个 effect 内部运行另一个 effect）。当前活跃的始终是栈顶。

### 3. Proxy 的数组方法拦截

```typescript
if (Array.isArray(target) && ["push","pop","splice"].includes(key)) {
  const originalMethod = Reflect.get(target, key, receiver);
  return function(...args) {
    const result = originalMethod.apply(target, args);
    const dep = self.getDep(target, "length");   // ← 数组突变 = length 变化
    dep.notify();
    return result;
  };
}
```

不拦截 `push` 本身会怎样？push 内部触发 set，但数组索引 (0, 1, 2...) 的 Dep 可能还没被收集。用 `length` 做数组级通知更可靠。

### 4. VDOM Diff：6 种 Patch

```
ADD      → 插入新节点
REMOVE   → 删除节点（含事件清理）
REPLACE  → 替换节点（cleanup + create + replaceChild）
TEXT     → 更新文本节点
MOVE     → 移动节点（insertBefore）
UPDATE   → 更新属性 + 递归 patch 子节点
```

**子节点 Patch 的执行顺序**：
1. REMOVE（索引从大到小，避免偏移）
2. MOVE（从后往前，避免索引问题）
3. ADD（索引从小到大，确保插入顺序）
4. 其他 UPDATE

这个顺序是实战中踩出来的——乱序执行会导致索引偏移，DOM 操作错位。

### 5. 属性处理：工业级的边界情况

```typescript
// 布尔属性：存在即 true
BOOLEAN_ATTRIBUTES.has("disabled")   →  el.setAttribute("disabled", "")
!value                                →  el.removeAttribute("disabled")

// DOM 属性（必须用 property 而非 attribute）
PROPERTY_ATTRIBUTES.has("value")     →  (el as HTMLInputElement).value = value

// style：支持 string 或 object
typeof value === "string"            →  el.style.cssText = value
typeof value === "object"            →  Object.entries(value).forEach(...)

// className：支持 string | array | object
Array.isArray(value)                 → value.filter(Boolean).join(" ")
typeof value === "object"            → Object.entries(value).filter(([_,active]) => active).map(...)
typeof value === "string"            → el.setAttribute("class", value)

// 事件：on* → addEventListener，切换 handler 时先 remove 旧的
key.startsWith("on")                 → el.addEventListener(eventName, handler)
oldHandler                           → el.removeEventListener(eventName, oldHandler)
```

## 和现代框架的映射

| jue 的概念 | Vue 3 对应 | SolidJS 对应 |
|-----------|-----------|-------------|
| `Dep` | `Dep` (effect scope) | `Signal` 的订阅者列表 |
| `reactive()` | `reactive()` | `createMutable()` / `createStore()` |
| `effect()` | `watchEffect()` | `createEffect()` |
| `targetMap` (WeakMap) | `targetMap` (同) | 无（编译期静态分析） |
| `VNode + patch()` | Virtual DOM + reconciler | 无 VDOM（编译为真实 DOM 操作） |
| `activeEffect` | `activeEffect` (同) | `Listener` / `Owner` |

核心差异：Vue 和 jue 用同一种模式（运行时 Proxy + WeakMap）；SolidJS 走编译期静态分析路线，没有 VDOM。

## 为什么知道这些有用

- **不会自己搞一个响应式框架**——Vue/SolidJS 已经做了，jue 的价值是帮助理解框架底层原理
- **属性处理逻辑是通用知识**——setAttribute 里的 style/class/event/boolean 分支，任何 UI 组件都需要
- **Patch 的顺序设计**（先删再移再加）是 DOM 批量更新的基础模式

## 来源

- jue 源码（`src/reactive/`、`src/vdom/`）
- 2026-06-07 agent 阅读后提炼
