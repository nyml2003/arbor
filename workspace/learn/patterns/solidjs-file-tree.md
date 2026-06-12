# 模式：SolidJS 递归文件树组件

## 一句话

用 SolidJS 的 `createResource` + `createSignal` + `Show` + `For` 构建按需加载、细粒度响应的递归文件树，不需要虚拟滚动库，不需要状态管理库。

## 核心组件

### FileTree（顶层）

```
FileTree
├── Header（标题 + 切换工作区按钮）
└── Nodes（滚动容器）
    └── TreeNodeList（排序 + 遍历）
        └── TreeNode（递归节点）
            ├── Button（展开/折叠 + 选中）
            └── TreeNodeList（子节点，条件渲染）
```

### 组件职责

| 组件 | 职责 | 关键 API |
|------|------|---------|
| `FileTree` | 持有选中态 + 展开态，加载根目录 | `createSignal`, `createResource` |
| `TreeNodeList` | 排序（目录优先、字母序），遍历 entries | `For` |
| `TreeNode` | 单个节点渲染，按需加载子节点 | `createResource`（key 为 null 时不加载） |

## 关键实现

### 1. 按需加载子目录

```tsx
function TreeNode(props) {
  const [children] = createResource(
    () => props.entry.isDirectory && props.expanded.has(props.entry.path)
      ? props.entry.path    // ← 展开时才加载
      : null,                // ← null 表示不加载
    fetchDirectory,
  );

  return (
    <>
      <button onClick={/* toggle expand */}>...</button>
      <Show when={props.entry.isDirectory && props.expanded.has(props.entry.path)}>
        <TreeNodeList entries={children() ?? []} {...props} />
      </Show>
    </>
  );
}
```

**为什么不用 `createMemo` 或手动 fetch 再 `setState`？**
- `createResource` 内置了 loading/error 状态，`children()` 在加载中是 `undefined` → `??[]` 自然处理空态
- key 变化时自动重新请求，不需要手动 `useEffect`
- 同一个 path 多次展开不会重复请求（SolidJS 的资源缓存）

### 2. 展开态管理

```tsx
const [expanded, setExpanded] = createSignal<Set<string>>(new Set());

const toggleExpand = (path: string) => {
  setExpanded((prev) => {
    const next = new Set(prev);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    return next;
  });
};
```

- 用 `Set<string>` 而非 `Record<string, boolean>` —— 展开的节点通常远少于总数
- 用 `setExpanded(prev => ...)` 的函数式更新，保证并发安全

### 3. 排序：目录优先

```tsx
const sorted = [...props.entries].sort((a, b) => {
  if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
  return a.name.localeCompare(b.name);
});
```

### 4. 深度缩进

```tsx
<button style={{ "padding-left": `${0.75 + props.depth * 1.25}rem` }}>
```

- 用 `depth` prop 递归传递，不需要维护树形数据结构
- 每个节点只知道自己的深度

## 数据流

```
workspace/ 目录（文件系统）
    │  readdir + stat
    ▼
main/ipc/filesystem.ipc.ts
    │  ipcMain.handle
    ▼
preload/index.ts（contextBridge）
    │  window.appAPI.fs.listDirectory()
    ▼
FileTree → createResource(fetchDirectory)
    │  返回 FileEntry[]
    ▼
TreeNodeList → For each → TreeNode
    │  展开时 createResource(子路径)
    ▼
TreeNodeList → ... 递归
```

## 为什么不用虚拟滚动

- 文件树通常节点数在数百级别，DOM 节点数不会成为瓶颈
- 虚拟滚动和递归展开的交互很难做好（展开一个节点后滚动偏移量计算复杂）
- 保持简单——如果以后文件数量到万级别，再加 `@tanstack/virtual`

## 反模式警示

### ❌ 一次性加载整棵树

```tsx
// 不要这样做
const [tree, setTree] = createSignal(buildFullTree(rootPath));
```

### ❌ 用 Effect 同步展开态和子节点加载

```tsx
// 不要这样做
createEffect(() => {
  if (expanded().has(path)) {
    fetchDirectory(path).then(setChildren);
  }
});
```

`createResource` 已经是声明式的了——声明 key → 自动管理加载。

### ❌ 用 DOM 操作控制展开

```
// 不要这样做
document.getElementById("children-" + id).classList.toggle("hidden");
```

SolidJS 的 `Show` 是真条件渲染——不展开的节点根本不在 DOM 中。

## 来源

- Arbor Phase 1 文件树实现（2026-06-07）
- SolidJS 官方文档：createResource、Show、For
