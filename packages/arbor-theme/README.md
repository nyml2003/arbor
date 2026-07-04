# Arbor Theme

一套克制、有质感的 VSCode 主题族。[Arbor](https://github.com/nyml2003/arbor) 项目出品。

## 主题

| 名称 | 类型 | 风格 |
|------|------|------|
| **Arbor Nocturne** | 暗色 | 午夜图书馆 — 暖灰底色 + 暗金强调 |
| **Arbor Nocturne Deep** | 暗色 | 更深邃的变体，OLED 友好 |
| **Arbor Vellum** | 亮色 | 纸质感 — 暖米底色，像一本印得好的书 |

所有主题共享同一套语法高亮配色，满足 WCAG AA 对比度标准，语义色不超过 12 种。

## 安装

```bash
cd packages/arbor-theme
pnpm package
code --install-extension arbor-theme-0.1.0.vsix --force
```

然后 `Ctrl+K Ctrl+T` 选择主题。

## 圆角

VSCode 主题只能控制颜色，UI 圆角需要自定义 CSS。两个扩展可选：

### 方案 A：Custom CSS and JS Loader（经典）

扩展 ID：`be5invis.vscode-custom-css`

搜不到的话用 `Ctrl+Shift+P` → **Extensions: Install from VSIX**，从 [GitHub Releases](https://github.com/be5invis/vscode-custom-css/releases) 下载 `.vsix` 安装。

### 方案 B：Custom UI Style（2026 活跃维护）

扩展 ID：`subframe7536.vscode-custom-ui-style`

同样搜不到就走 VSIX 安装：[GitHub Releases](https://github.com/subframe7536/vscode-custom-ui-style/releases)

### 配置

安装后在 `settings.json` 中加入（Windows 路径用三个斜杠 `file:///`）：

```json
"vscode_custom_css.imports": [
  "file:///C:/Users/nyml/code/arbor/packages/arbor-theme/styles/workbench.css"
],
"vscode_custom_css.policy": true
```

`Ctrl+Shift+P` → **Reload Custom CSS and JS**（或 **Enable Custom CSS and JS**）生效。每次 VS Code/Cursor 更新后需要重新执行一次。

会给标签页、输入框、下拉菜单、命令面板、自动补全、通知和右键菜单加上 4–8px 的微圆角。

## 开发

主题由 token 生成，**不要直接改 `themes/*.json`**。

```bash
pnpm generate   # 从 tokens/palette.mjs 生成所有主题 JSON
pnpm validate   # 对比度校验
pnpm build      # generate + validate
pnpm package    # build + 打包 VSIX
```

### 新增变体

1. 在 `tokens/palette.mjs` 中加一个 export
2. 在 `package.json` 的 `contributes.themes` 中加一条
3. `pnpm generate`

## 设计原则

- **不出现纯黑或纯白** — 所有表面都有温度
- **WCAG AA 合规** — 正文对比度 ≥ 4.5:1，光标 ≥ 4.5:1
- **≤ 12 种语义色** — 不过度着色
- **明度分层，不靠色相** — UI 面板通过灰度区分层级，而非不同颜色

## 支持语言

针对 JS/TS、Rust、Python、HTML/CSS/SCSS、JSON/YAML/TOML、Markdown 做了优化，启用语义高亮。

## 许可

MIT — 见主仓库 [LICENSE](https://github.com/nyml2003/arbor)。
