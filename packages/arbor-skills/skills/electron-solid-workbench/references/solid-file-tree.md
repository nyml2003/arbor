# Solid 文件树

## 意图

用 SolidJS 的细粒度响应和按需资源加载维护文件树，不提前引入复杂状态库和虚拟滚动库。

## 适用场景

- 递归文件树。
- 展开/折叠目录。
- 按工作区读取本地目录。
- Markdown 或作品集浏览侧边栏。

## 必须遵守的规则

- 展开目录时再加载子目录。
- 未展开节点不要挂载子树。
- 展开态用局部状态表示，不引入全局状态库。
- 文件列表排序保持目录优先、名称稳定。
- 组件不要绕过 Solid 响应式直接操作 DOM。

## 推荐模式

- `createResource` 的 key 为路径；未展开时 key 为 `null`。
- `Show` 控制子树是否存在。
- `For` 渲染条目。
- `Set<string>` 存展开路径。
- `depth` 递归传递缩进，不额外维护树形副本。

## 反模式

- 启动时读取完整目录树。
- 用 CSS 隐藏大量未展开节点。
- 用 effect 手动同步 fetch 和 setState。
- 在几百个节点阶段提前引入重型虚拟滚动。

## 证据

- `workspace/learn/patterns/solidjs-file-tree.md` 记录 Arbor Phase 1 文件树实现。
- `workspace/learn/patterns/ts-ui-performance-rules.md` 记录按需挂载、局部加载和窗口化策略。
