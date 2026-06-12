# 模式：Astro 文件树多媒体作品集（Untitled）

## 一句话

用 Astro 的 content collections + SSG（`getStaticPaths`）把统一内容模型渲染为文件树形态的静态站点。这不只是一个博客——是 file-tree-based multimedia portfolio。

Untitled 的核心设计：

- **统一内容模型**：article / music / video / game / folder 五种类型，用同一个 `ContentNode` 类型承载
- **文件树 UI**：不是真的读文件系统——是从内容图（content graph）生成树
- **SSG（静态站点生成）**：`getStaticPaths()` 为每个 node 生成一页 HTML
- **多媒体支持**：audio、video、自研游戏（tetris）都可以嵌入

## 核心架构

```
Content Collections (Astro)
  │  posts/, music/, videos/, games/
  │  每个 collection 有 schema (zod)
  ▼
Content Query Layer (content-schema.ts, content-query.ts)
  │  getAllNodes(), getTree(), getNodeByPath()
  │  统一 ContentNode 类型
  ▼
SSG (getStaticPaths)
  │  为每个 node.path 生成一个静态页面
  │  /node/articles/welcome-to-the-shell/
  │  /node/games/prototype/
  │  /node/music/album/track/
  ▼
Astro Components
  │  TreePane → TreeBranch (递归)
  │  MediaDock, PreviewPane, MetadataPane
  ▼
Custom Elements (progressive enhancement)
     x-file-tree, x-media-dock, x-preview-pane
```

## 关键设计

### 1. 统一 ContentNode 类型

```typescript
export type ContentNode = {
  id: string;
  kind: 'article' | 'music' | 'video' | 'game' | 'folder';
  slug: string;
  title: string;
  summary: string;
  cover: string;
  path: string[];           // ← 树路径: ['articles', 'tech', 'rust']
  tags: string[];
  publishedAt: string;
  updatedAt: string;
  assets: string[];         // ← 关联的媒体文件
  entry: string;            // ← 入口 URL 或特殊标识 (如 'tetris')
  children: string[];       // ← 子节点 ID
  related: string[];        // ← 关联节点 ID
  draft: boolean;
  sourceCollection: string; // ← 来源 collection
  sourceId: string;
};
```

**folder 也是节点**——目录是合成的（synthetic），不是文件系统目录。这给了很大的灵活性：可以有空的、描述性的目录节点。

### 2. TreeItem 递归类型

```typescript
export type TreeItem = {
  id: string;
  title: string;
  kind: ContentKind;
  path: string[];
  url: string;
  children: TreeItem[];     // ← 递归
};
```

**TreePane.astro 渲染**：
```astro
<x-file-tree data-current={currentPath.join('/')}>
  <ul class="tree">
    {tree.map((item) => <TreeBranch item={item} currentPath={currentPath} />)}
  </ul>
</x-file-tree>
```

### 3. SSG：一个 node 一页

```typescript
export async function getStaticPaths() {
  const nodes = await getAllNodes();
  return nodes.map((node) => ({
    params: { slug: node.path.join('/') },
    props: { path: node.path },
  }));
}
```

**结果**：
```
dist/
├── node/articles/              ← 每级目录都有 index.html
│   └── welcome-to-the-shell/
├── node/games/
│   └── prototype/
├── node/music/
│   └── album/track/
└── index.html                  ← 首页
```

### 4. Content Query Layer

把 content collections 的原始数据转换成统一模型：

```typescript
// content-query.ts
getAllNodes()      → ContentNode[]   // 所有节点
getTree()          → TreeItem[]      // 树结构
getNodeByPath(p)   → ContentNode     // 按路径查找
getRelatedNodes(n) → ContentNode[]   // 关联节点
getFolderChildren(p) → ContentNode[] // 文件夹子节点
```

### 5. Custom Elements for 树交互

```typescript
class XFileTree extends HTMLElement {
  connectedCallback(): void {
    const current = this.getAttribute('data-current') ?? '';
    // 自动展开包含当前路径的 <details> 元素
    for (const details of this.querySelectorAll('details[data-path]')) {
      if (current.startsWith(path)) {
        details.open = true;
      }
    }
  }
}
```

**极简但有效**：树展开逻辑不依赖 JS 框架。`<details open>` 是原生 HTML 的手风琴效果。custom element 只做了一件事——根据当前 URL 路径自动展开对应的树节点。

## 反模式警示

### ❌ 文件和页面 1:1 映射

不要把每个 workspace 文件都变成一页 HTML。需要 query layer 做转换：Markdown → 渲染后的文章，树节点 → 目录页（列出子节点），多媒体 → 播放器页。

### ❌ 树从文件系统直接读

Untitled 的树是从 content graph 生成的，不是从 `readdir` 生成的。合成节点（folder）可以包含描述、封面、自定义排序——文件系统做不到这些。

## 来源

- Untitled 源码（`src/pages/node/[...slug].astro`、`src/lib/content-schema.ts`、`src/components/TreePane.astro`、`src/elements/x-file-tree.ts`）
- 2026-06-07 agent 阅读后提炼
