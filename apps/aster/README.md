# Aster

Aster 是一个本地 DeepSeek agent CLI。它支持单轮提问和进程内连续对话，不读取本地文件，不执行命令，也不保存会话。

普通输出默认使用 DeepSeek `stream: true`。终端会先直接流式打印模型文本，等一条回复完成后，再追加一个轻量的终端 Markdown 渲染版。`--json` 仍使用非流式请求，用来查看原始响应。

## 使用

先设置 DeepSeek API key：

```powershell
$env:DEEPSEEK_API_KEY = "sk-..."
```

运行一次提问：

```powershell
pnpm --filter @arbor/aster build
node apps/aster/dist/cli.js "帮我解释这段错误"
```

普通模式会先看到流式文本，然后看到一段 `--- rendered markdown ---` 后的终端美化输出。

进入连续对话：

```powershell
node apps/aster/dist/cli.js
```

在连续对话里输入 `/exit` 或 `/quit` 退出。会话历史只保存在当前进程里，退出后不会保存。

连续对话里可以临时加载 skill：

```text
aster> /skill plain-tech-writing-cn
aster> 这个 skill 讲了什么
aster> /skills
```

指定 system prompt 或模型：

```powershell
node apps/aster/dist/cli.js --system "你是一个严谨的代码助手" "给我一个排查思路"
node apps/aster/dist/cli.js --model deepseek-v4-pro "复杂一点的问题"
```

使用本地 skill：

```powershell
node apps/aster/dist/cli.js --skill plain-tech-writing-cn "改写这段说明"
node apps/aster/dist/cli.js --skill packages/arbor-skills/skills/domain-core-architect "评审这个 core 设计"
```

`--skill` 会读取本地 `SKILL.md`，并把内容注入到本轮 system/context。它不会执行 skill 里的脚本，也不会安装或修改 skill。默认会搜索 `.codex/skills`、`packages/arbor-skills/skills` 和 `.installed-skills`。可以用 `--skill-dir <path>` 追加搜索目录。

构建后也可以用 package 脚本进入连续对话：

```powershell
pnpm --filter @arbor/aster chat
```

输出原始响应，不启用流式打印：

```powershell
node apps/aster/dist/cli.js --json "返回原始响应"
```

## 验证

```powershell
pnpm --filter @arbor/aster test
pnpm --filter @arbor/aster typecheck
pnpm --filter @arbor/aster build
```
