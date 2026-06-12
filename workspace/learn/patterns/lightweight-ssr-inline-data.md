# 模式：轻量 SSR —— 内联 JSON + 前端 Hydrate（Aura）

## 一句话

不用 Next.js，不用 React SSR，不用 Node 渲染 JSX。服务器返回一个空 HTML 壳 + `<script type="application/json">` 内联数据，前端启动时读 `window.__INIT_DATA__` 直接渲染。首屏快，服务器轻。

## 为什么需要这个

传统的 SSR 方案（Next.js、Astro）引入一整套工具链。Aura 的模式简单到可以手写——**用 template string 生成 HTML，内联 JSON，前端 hydrate。**

## 模式结构

```
后端（Express + Node）
  │
  ├── GET /          → listPageHtml(articles)     → 返回空壳 + 文章列表 JSON
  ├── GET /:slug     → articlePageHtml(article)   → 返回空壳 + 文章内容 JSON
  │
  └── 前端（独立 bundle，由 Vite/其他打包）
      ├── /assets/index.js    → 列表页 hydrate
      └── /assets-article/index.js → 文章页 hydrate
```

## 核心实现

### 1. 服务器生成 HTML

```typescript
function pageShell(pageTitle, initData, options, appKind): string {
  const payload = escapeScriptPayload(JSON.stringify(initData));
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8" />
  <title>${escapeHtml(pageTitle)}</title>
  <link rel="stylesheet" href="${assetsBase}/index.css" />
</head>
<body>
  <div id="root"></div>
  <script id="__INIT_DATA__" type="application/json">${payload}</script>
  <script>window.__PERF_SERVER__={generatedAt:${generatedAt}};</script>
  <script>
    (function(){
      var el = document.getElementById('__INIT_DATA__');
      if (el && el.textContent) {
        try { window.__INIT_DATA__ = JSON.parse(el.textContent); } catch(e) {}
      }
    })();
  </script>
  <script type="module" src="${assetsBase}/index.js"></script>
</body>
</html>`;
}
```

### 2. 前端 hydrate

```typescript
// 前端入口：先读内联数据，再渲染
const data = window.__INIT_DATA__;
if (!data) {
  // 降级：fetch API
  const resp = await fetch("/api/articles");
  data = await resp.json();
}
render(() => <ArticlePage article={data.article} />, root);
```

**首屏不闪、不抖**——因为数据已经在 HTML 里了，前端不需要额外请求。

### 3. 安全转义

```typescript
// JSON 内联在 <script> 里的唯一安全问题：</script> 会提前闭合标签
function escapeScriptPayload(json: string): string {
  return json.replace(/<\//g, "<\\/");  // </ → <\/
}
// HTML 属性/内容转义
function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;")...;
}
```

## 反模式警示

### ❌ 把内联数据放在可见 DOM 里

```html
<!-- 不要 -->
<div id="data" style="display:none">{ "articles": [...] }</div>
```

HTML parser 会解析里面的内容，特殊字符可能破坏 DOM。应该用 `<script type="application/json">`，parser 把它当不可见脚本块处理。

### ❌ 用 `<script>` 的 `src` 加载数据

```html
<!-- 不要：多一次 HTTP 请求 -->
<script src="/api/init-data.js"></script>
```

内联就是省掉这次请求。首屏的 100ms 差距就在这里。

## 来源

- Aura `app/backend/src/ssr/html.ts`
- 2026-06-07 agent 阅读后提炼
