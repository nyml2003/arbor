# Arbor Skill Manager CLI

`@arbor/skill-manager-cli` 提供全局命令 `arbor`，用于校验、安装和清理 agent skill。

## 要求

- Node.js 24+
- npm 或 pnpm

## 安装

公开发布后：

```powershell
npm install -g @arbor/skill-manager-cli
```

当前仓库内本地验证：

```powershell
pnpm --filter @arbor/skill-manager-cli build
pnpm --dir packages/skill-manager-core pack
pnpm --dir packages/skill-manager-cli pack
npm install -g .\packages\skill-manager-core\arbor-skill-manager-core-0.1.0.tgz .\packages\skill-manager-cli\arbor-skill-manager-cli-0.1.0.tgz
```

## 验证

```powershell
arbor --version
arbor doctor
arbor doctor --json
```

针对某个项目清单：

```powershell
arbor doctor --manifest arbor.skills.json --cwd .
arbor skill lint --manifest arbor.skills.json --cwd .
arbor skill install --manifest arbor.skills.json --cwd . --dry-run
```

## 升级

公开发布后：

```powershell
npm install -g @arbor/skill-manager-cli@latest
```

本地 tgz 验证时，重新 pack 并安装新的 tgz。

## 发布

仓库根目录提供一键发布脚本。默认只做 dry-run：

```powershell
pnpm release:skill-manager
```

真实发布：

```powershell
npm login
pnpm publish:skill-manager
```

脚本会按顺序执行版本校验、npm 已发布版本检查、core/CLI 测试、pack 检查、临时全局安装 smoke，然后按 `@arbor/skill-manager-core`、`@arbor/skill-manager-cli` 的顺序发布。

## 卸载

```powershell
npm uninstall -g @arbor/skill-manager-cli
```

如果本地验证时也全局安装过 core 包，可以一起卸载：

```powershell
npm uninstall -g @arbor/skill-manager-core
```

## 开发验证

```powershell
pnpm --filter @arbor/skill-manager-cli test
pnpm --filter @arbor/skill-manager-cli pack:check
pnpm --filter @arbor/skill-manager-cli install:global:local
```

`install:global:local` 使用临时 npm prefix，不会写入真实全局 npm 目录。
