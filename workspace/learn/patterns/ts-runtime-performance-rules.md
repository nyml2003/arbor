# 模式：TypeScript 非 UI 热路径规则

## 一句话

先把热路径缩短，再做局部优化。非 UI 层最值得借鉴的方向有五个：编译期显式化依赖、slot/索引驱动、稳定数据布局、可预期失败不用异常、解析路径 single pass。

## 这篇只讲什么

这篇只保留非 UI 层能迁移到别的 TypeScript 项目的部分：

- 运行时内核
- 编译器 / parser / scheduler
- 数据处理和状态流转

明确不收录这些内容：

- UI 组件渲染技巧
- 虚拟列表、文件树这类界面结构优化
- 还没有 benchmark 支撑的实验性猜想

## 1. 先缩短热路径，再谈语法糖

热路径里最贵的不是某个 `if`，而是整条更新链太长。

如果一个状态更新要经过这些步骤：

1. 重跑组件
2. 重新收集依赖
3. 重建中间树
4. 再做一轮通用 diff
5. 最后才落到真实副作用

那后面的微优化价值不大。

更稳的写法是把依赖边提前写出来，让运行时只做命中和分发：

```ts
signal slot -> dependency table -> dirty mark -> queued work -> commit
```

这条链短，行为也更可预测。出问题时，你能直接看 slot、队列和提交顺序，不需要回溯“这次为什么又跑了一遍整个系统”。

适用场景：

- 自己维护 runtime-core
- 编译器生成运行时结构
- 高更新频率的状态系统

不适用场景：

- 普通 CRUD 页面
- 业务逻辑稀疏、更新频率低的脚本或后台工具

## 2. 热数据优先平铺，不要建对象图

热路径优先用这些结构：

- `Uint8Array`
- `Uint32Array`
- 固定字段对象
- 元素类型稳定的普通数组

少用这些结构：

- 大量互相引用的对象
- 热循环里的 `Map` / `Set`
- 同一个数组里混 `number` / `object` / `function`

原因很简单。平铺结构更适合顺序访问，也更容易保持对象 shape 稳定。

典型写法：

```ts
type Blueprint = Readonly<{
  bindingOpcode: Uint8Array;
  bindingNodeIndex: Uint32Array;
  bindingDataIndex: Uint32Array;
  signalToBindingStart: Uint32Array;
  signalToBindingCount: Uint32Array;
  signalToBindings: Uint32Array;
}>;
```

这类结构的好处不是“看起来底层”，而是：

- 数据连续
- 扫描顺序稳定
- 调试时能直接看索引
- 可以把“读表”和“做副作用”拆开

## 3. 用 slot、opcode、bitset 表达状态

高频路径要优先选择整数索引，而不是名字查找和散装对象访问。

常见做法：

- signal 用 `slot`
- binding 用 `slot`
- 生命周期状态用稳定常量或 opcode
- dirty 标记用 bitset

bitset 是很实用的一条。它同时解决两件事：

1. 判断某个 slot 这批次是否已经脏了
2. 给调度器做去重

```ts
const wordIndex = slot >>> 5;
const bitMask = 1 << (slot & 31);
const previous = words[wordIndex] ?? 0;
words[wordIndex] = previous | bitMask;
const firstDirty = (previous & bitMask) === 0;
```

这比“用一个 `Set<number>` 存所有 dirty slot”更硬，也更适合热路径。

## 4. 可预期失败不要走异常

文件不存在、权限不足、输入不合法，这些都不是异常情况。它们是正常失败。

高频代码里，正常失败优先返回 `Result`：

```ts
type Result<T, E> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: E };
```

这么做的价值不只是代码风格：

- 失败路径可见
- 调用方被类型系统强制处理
- 不会把正常失败混进异常控制流

真正该抛异常的，是这些情况：

- 不变量被破坏
- 内部状态损坏
- 不可能分支被命中

一句话：不要让热路径靠 `throw` 走业务分支。

## 5. 运行时常量优先 `const object`，少用 TS `enum`

如果一组值只是运行时常量，不需要 `enum`。

```ts
export const FsErrorCode = {
  ENOENT: "ENOENT",
  EACCES: "EACCES"
} as const;
```

这样做更直接。运行时代码也更轻。

`enum` 不是不能用。问题在于很多场景里，它带来的运行时代码和语义负担都没有必要。特别是在热路径周围，大量 `enum` 往往只是历史习惯。

## 6. parser / scanner 尽量 single pass

能单次扫描完成的事情，就不要先 `split()`，再正则切，再回头拼。

更稳的写法是：

- 一个 `position` 指针
- 从左到右扫一遍
- 一边读，一边产出 token 或状态

```ts
while (position < source.length) {
  const char = source[position];
  // 按当前字符决定状态转移
  position += 1;
}
```

这样做的好处：

- 少中间数组
- 少临时字符串
- 行列号和错误位置更容易带出来
- 性能更稳定

如果输入数据有明显特征，还可以继续做：

- 零拷贝
- 延迟解码
- 只在必要时分配新字符串

核心原则不是“自己手写 parser 更酷”，而是不要让通用工具在窄问题上付出太多额外成本。

## 7. 调度统一收口，不要旁路更新

一旦系统里同时有这些东西：

- 数据更新
- 结构更新
- 异步返回
- 消息分发

它们就该进同一个 scheduler。

不要让系统变成这样：

- signal 自己 flush
- region 自己切换
- channel 自己派发
- async resource 自己提交

旁路一多，顺序就不稳定。顺序不稳定，性能问题和一致性问题会一起出现。

一个可借鉴的最小模型是：

- 按 lane 分桶
- 同 slot 去重
- 统一 batch flush
- 先结构，再数据，再副作用提交

这类调度模型的价值不只是“快”，而是稳定。稳定之后，性能才有优化空间。

## 8. 先判断值不值，再决定 TS 还是 Rust

不是所有性能问题都该在 TS 里硬抠。

继续留在 TS 的情况：

- 瓶颈主要在 IO，不在计算
- 逻辑复杂度高，数据量不大
- 需要快速改动和高频迭代
- 主要收益来自结构优化，不是纯算力

该考虑 Rust 或其他更低层方案的情况：

- 热循环时间占比已经很高
- 解析、编译、文件处理是主业务
- 需要长期跑大规模数据
- 已经做完结构优化，CPU 仍是明确瓶颈

不要一上来就迁语言。也不要明明已经是算力问题，还继续在 TS 里绕。

## 反模式警示

### 1. 用全局重算掩盖依赖建模失败

“先全量跑一遍，后面再优化”很容易变成永远不优化。

### 2. 热数组里混多种 shape

这会让代码既难推断，也难稳定。

### 3. 正常失败靠异常分支

调用方迟早漏接。性能和行为都不稳。

### 4. 明明是窄问题，却先上大而全抽象

比如窄场景 parser 先套多层通用框架，或者固定结构数据先建对象森林。很多时候这不是扩展性，是提前付税。

### 5. 只会测 UI，不测 runtime / compiler

性能判断如果没有 benchmark，最后就会退回“感觉应该更快”。

## 最小清单

- 更新链能不能缩成“命中表 -> 标脏 -> 调度 -> 提交”
- 热数据能不能平铺成数组或 `TypedArray`
- 高频状态能不能改成 `slot + bitset`
- 正常失败能不能从异常改成 `Result`
- scanner / parser 能不能改成 single pass
- 这个问题到底该不该继续留在 TS

## 来源

- jue 文档与源码（运行时热路径、slot/TypedArray、dirty bitset、scheduler）
- ObolosFS 文档与源码（Result-based error、const object 代替 enum、single pass shell parser）
- url-parser-bench 模式文档（零拷贝、按数据特征做解析优化）
- 2026-06-07 agent 阅读后提炼
