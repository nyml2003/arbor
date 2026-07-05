# Aster 演进计划

## 1. 现状

Aster 目前是一个 TUI 聊天应用。核心技术栈：

- **Rust** + `arbor-tui` 自研 TUI 框架（立即模式渲染、信号驱动响应式）
- **DeepSeek API**（OpenAI 兼容协议），SSE 流式输出
- **pulldown-cmark** + **syntect** 做 Markdown 渲染和代码高亮
- 代码按 DDD 风格拆成 5 个 crate：
  - `aster-domain`：消息、角色、会话状态和状态转换。
  - `aster-application`：聊天用例、流式事件和模型客户端端口。
  - `aster-adapters`：DeepSeek/OpenAI 兼容 HTTP + SSE 适配器。
  - `aster-markdown`：Markdown 到 arbor-tui `Span`/block 的转换。
  - `aster-tui`：终端运行循环、状态模型、`build_ui` 和性能统计。

当前能做的事：

- 在终端里和 DeepSeek 多轮对话
- 流式显示回复，Markdown 渲染 + 代码块语法高亮
- 键盘滚动历史消息
- 退出时输出帧率性能报告

当前**不能**做的事（也是一般 Agent 的必备能力）：

- 不能调用工具（读文件、写文件、执行命令、搜索代码）
- 不能多步推理——只做单轮请求，模型返回文本就结束
- 没有项目上下文感知（不读 CLAUDE.md、.gitignore）
- 没有上下文压缩——长对话会撑爆 token 窗口
- 没有权限控制——所有操作无安全闸门
- 没有会话持久化——退出即丢失
- 没有子 Agent 并行

**定位**：从"能聊天的 TUI"演进为"能在终端里干活的 Agent"。参考目标：Claude Code 的架构模式，但做减法——只保留个人日常使用的核心能力，不追求企业级的多租户、遥测、插件市场。

---

## 2. ofsh 集成分析

### 2.1 ofsh 是什么

ofsh（`@obolosfs/ofsh`）是一个构建在 ObolosFS 虚拟文件系统之上的 Shell。来源：`C:\Users\nyml\code\ObolosFS\packages\ofsh`。

**核心架构**（4 阶段流水线）：

```
输入行 → Lexer → Token[] → Parser → AST → Resolver → AST(已验证) → Executor → 输出
```

**关键抽象**：

- **`Vfs`** — 虚拟文件系统。支持 `mount(path, driver)` 挂载任意后端。文件操作走统一接口（`open`、`readdir`、`stat`、`mkdir`、`unlink` 等），不直接接触真实文件系统。
- **`Driver`** — 存储后端 trait。内置 `MemDriver`（纯内存），可扩展为真实文件系统 Driver。
- **`CommandRegistry`** — 可插拔命令注册表。每个命令是 `(context, args) => Promise<string>` 的函数。内置 11 个命令：cat、echo、grep、ls、mkdir、rm、rmdir、mv、touch、stat、exit。
- **管道** — 多命令用 `|` 连接时，并行执行，通过 VFS Pipe 传递数据。不是简单的字符串拼接，是真正的流式管道。
- **重定向** — `>` 和 `>>` 支持。

### 2.2 为什么适合 aster

| 能力 | 直接用 Bash | 用 ofsh |
|------|------------|---------|
| 文件系统隔离 | 无，所有操作直接落在真实磁盘 | VFS 沙箱，只读挂载保护源码 |
| 跨平台 | `bash` vs `cmd.exe` 语法不同 | 统一命令语法，不依赖系统 shell |
| 可组合性 | 依赖 shell 的管道语法 | VFS Pipe 原生支持，LLM 更容易理解和生成 |
| 权限控制 | 无，只能全局 sudo 或不可执行 | 按 mount 点控制 read/write/list/delete |
| 可扩展性 | 只能调系统命令 | 注册自定义命令（例如 `search code "pattern"`） |
| 转义安全 | 需要处理 shell 注入 | 词法分析器处理引号和转义，无 shell 注入风险 |
| 审计 | 无 | 所有 VFS 操作可记录 |

### 2.3 集成方案

ofsh 是 TypeScript。aster-rs 是 Rust。两条路径：

**路径 A：子进程嵌入（短期）**

aster-rs 启动时 spawn 一个 Node.js 子进程，跑 ofsh session。通过 JSON-RPC over stdin/stdout 通信。

```
aster-rs (Rust)                    ofsh-agent (Node.js)
     │                                    │
     ├── JSON-RPC: { "method": "exec",    │
     │     "params": { "line": "ls /src", │
     │     "mounts": [...] }}             │
     │ ─────────────────────────────────> │
     │                                    │── VFS mount real dirs
     │                                    │── lex → parse → execute
     │                                    │── collect output
     │ <───────────────────────────────── │
     │   { "result": { "output": "..." }}
```

优点：一周能跑通，不需要重写。
缺点：依赖 Node.js 运行时，启动成本 ~200ms。

**路径 B：Rust 移植（长期）**

将 ofsh 的核心概念移植到 Rust：
- `Driver` trait → Rust trait（async）
- `Vfs` → 结构体 + mount table
- Lexer/Parser → 手写递归下降（ofsh 的语法很简单，不需要 parser generator）
- `CommandRegistry` → `HashMap<String, CommandHandler>`
- `MemDriver` → 内存 BTreeMap 实现

优点：零外部依赖，编译为单一二进制，性能更好。
缺点：工作量大，估计 2-3 周。

**建议**：先走路径 A 验证集成价值。确认 ofsh 的沙箱模型在 Agent 场景好用之后，再决定是否移植。移植时保留 ofsh 的接口设计（Driver trait、Vfs API、CommandContext），因为这些设计已经过验证。

### 2.4 集成后的 ShellTool 行为

不管底层用路径 A 还是路径 B，aster 暴露给模型的 `ShellTool` 接口不变：

```rust
// ShellTool 的 call() 实现（伪代码）
fn call(&self, args: ShellArgs) -> ToolResult {
    // 1. 构建 ofsh session，挂载当前工作目录
    let session = ofsh.create_session();
    session.mount("/workspace", real_cwd, MountPermissions { read: true, write: self.allow_write });

    // 2. 执行命令
    let result = session.execute(&args.command);

    // 3. 如果命令有写操作，把变更写回真实文件系统
    if self.allow_write && result.has_writes() {
        session.sync_to_real_fs("/workspace");
    }

    ToolResult { output: result.output }
}
```

关键点：
- 默认只读挂载——模型可以读任何文件，但不能写
- 写操作需要显式声明，走权限检查（Phase 4）
- 所有文件变更先在 VFS 内存中完成，确认后再落盘

---

## 3. 自定义 SKILL 格式

### 3.1 为什么需要 SKILL

Claude Code 的 SKILL.md 解决的问题：

- 把"怎么干活"的知识从对话中抽离，存为可复用的文件
- 模型启动时加载相关 SKILL，直接注入系统提示词
- SKILL 可以声明自己需要的工具子集，避免无关工具的干扰

aster 需要同样的能力。但 Claude Code 的 SKILL.md 是纯 markdown，缺少结构化元数据。aster 的 SKILL 格式设计目标：

1. **机器可读的元数据**（YAML frontmatter）— 工具过滤、模型选择、参数声明
2. **人类可写的指令体**（Markdown body）— 注入系统提示词
3. **可组合**— 多个 SKILL 可以同时激活，指令拼接
4. **可参数化**— SKILL 可以接受参数（例如 `code-review --lang rust`）

### 3.2 文件格式

文件名：`*.aster-skill.md`

```markdown
---
name: rust-code-review
description: 审查 Rust 代码的安全性和正确性
version: 1
tools:
  - read
  - grep
  - bash
mounts:
  - path: /code
    source: ./
    permissions: { read: true, write: false }
params:
  lang:
    type: string
    default: rust
    description: 目标语言
model: claude-sonnet-4-6
---

# Rust 代码审查

你是 Rust 代码审查专家。审查代码时遵循以下规则：

## 检查清单

- [ ] 所有 `unsafe` 块有 `SAFETY` 注释说明为什么安全
- [ ] 所有 `Result` 被显式处理，没有 `let _ = ...`
- [ ] 库代码中没有 `unwrap()` 或 `expect()`
- [ ] 没有在 `unsafe` 块外使用裸指针解引用
- [ ] 错误类型实现了 `std::error::Error` trait

## 审查流程

1. 先读 `Cargo.toml` 了解依赖和 feature flags
2. 用 `grep` 找出所有 `unsafe` 块
3. 逐个审查每个 `unsafe` 块的 SAFETY 注释
4. 检查错误传播路径是否完整
5. 输出审查报告
```

### 3.3 YAML Frontmatter 字段

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 唯一标识，kebab-case。例如 `rust-code-review` |
| `description` | string | 是 | 一句话描述，用于 `--list-skills` 展示 |
| `version` | number | 否 | 版本号，默认 1 |
| `tools` | string[] | 否 | 可用工具白名单。不写则继承全局工具集。工具名对应 Tool trait 的 `name()` |
| `mounts` | Mount[] | 否 | VFS 挂载点。不写则不挂载任何真实文件系统 |
| `params` | map | 否 | 可接受参数。key 是参数名，value 是 `{ type, default, description }` |
| `model` | string | 否 | 推荐模型。不写则用默认模型。允许 skill 建议用更便宜/更强的模型 |
| `requires` | string[] | 否 | 依赖的其他 skill 名称。加载前先检查依赖是否满足 |

**Mount 子字段**：

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `path` | string | 是 | VFS 中的挂载点，例如 `/code` |
| `source` | string | 是 | 真实路径。`./` 表示启动 aster 时的当前目录 |
| `permissions` | object | 否 | `{ read, write, delete, list, rename }`，默认全 false |

### 3.4 加载与激活

**发现机制**（按优先级）：

1. 项目级：`<cwd>/.aster/skills/*.aster-skill.md`
2. 用户级：`~/.aster/skills/*.aster-skill.md`
3. 项目指引文件引用的 skill（CLAUDE.md 中写 `skill: rust-code-review`）

**激活方式**：

- 显式：用户输入 `/rust-code-review`（斜杠命令自动匹配 skill name）
- 自动：模型在系统提示词中看到可用 skill 列表，可以主动提议激活
- 条件：`requires` 声明的依赖 skill 自动级联加载

**注入方式**：

```
[System] 可用技能:
  - rust-code-review: 审查 Rust 代码的安全性和正确性
  - write-tests: 为修改过的代码生成单元测试
  - explain-error: 解释编译错误的根因和修复方案

[System] 当前激活技能: rust-code-review

[System] <skill:rust-code-review>
你是 Rust 代码审查专家。审查代码时遵循以下规则:
...
</skill:rust-code-review>
```

### 3.5 与 Claude Code SKILL.md 的区别

| | Claude Code SKILL.md | aster SKILL |
|------|------|------|
| 元数据 | 全在 markdown 正文里，靠约定解析 | YAML frontmatter，严格 schema |
| 工具过滤 | 不支持（skill 不限制工具集） | `tools` 白名单 |
| 参数 | 不支持 | `params` 声明，激活时传参 |
| 挂载 | 不支持 | `mounts` 声明 VFS 挂载 |
| 模型推荐 | 不支持 | `model` 字段 |
| 依赖 | 不支持 | `requires` 级联加载 |

aster 的格式更结构化，因为 aser 需要知道"这个 skill 只能用什么工具、能看到什么文件"来做权限控制。Claude Code 不需要这些因为它有完整的文件系统和工具权限。

### 3.6 示例：为 aster 自身开发用的 skill

```markdown
---
name: aster-dev
description: aster-rs 项目的开发助手
tools: [read, grep, bash, write]
mounts:
  - path: /code
    source: ./
    permissions: { read: true, write: true }
params:
  task:
    type: string
    description: 开发任务描述
---

# aster-rs 开发

当前项目是 aster-rs，一个 Rust TUI Agent 应用。

## 项目结构

- `crates/aster-domain` — 纯领域模型。不能依赖 TUI、HTTP、环境变量或文件系统。
- `crates/aster-application` — 用例层和端口。负责发送消息、轮询流式事件、更新会话状态。
- `crates/aster-adapters` — DeepSeek/OpenAI 兼容 API 适配器。负责环境变量、HTTP 和 SSE。
- `crates/aster-markdown` — Markdown 渲染。负责把文本转换成 arbor-tui 可渲染的 blocks。
- `crates/aster-tui` — 可运行 TUI。文件按 `state.rs -> ui.rs -> runner.rs` 分层。

## 编码规范

- 模块注释用 `//` 写文件头
- 函数注释只写行为、条件、风险，不解释一眼能懂的代码
- 用 `anyhow::Result` 做顶层错误类型
- 新增业务规则先放 `aster-domain` 或 `aster-application`，不要写进 `aster-tui`。
- `build_ui` 保持纯构建函数。输入状态和 theme，输出 widget tree。

## 测试

```bash
cd /code && cargo test --quiet
```

## 构建

```bash
cd /code && cargo build --release
```
```

---

## 4. 参考架构：Claude Code 的设计要点

以下是从 Claude Code 逆向分析中提炼的关键设计决策。完整资料来源见文末。

### 2.1 Agent Loop 就是一个 while 循环

```
while model_returns_tool_call:
    execute_tools(tool_calls)
    send_results_back_to_model()
return final_text
```

不做 DAG、不做任务路由、不做意图分类器。模型自己决定调哪个工具、什么顺序、何时结束。决策逻辑只占代码量的 ~1.6%，其余全是操作基础设施（权限、上下文压缩、工具执行）。

**对 Aster 的启示**：Agent Loop 本身不需要复杂设计。复杂的是 Loop 周围的基础设施。

### 2.2 工具系统：自描述 + 安全语义

每个工具是一个实现了统一接口的生命周期单元：

```typescript
type Tool = {
  name: string
  inputSchema: ZodSchema          // 输入校验
  isConcurrencySafe(): boolean    // 能否并发
  isReadOnly(): boolean           // 是否只读
  checkPermissions(): Permission  // 权限检查
  call(args, context): Result     // 执行逻辑
}
```

安全语义编码在接口方法里，不是外部配置文件——工具实现和它的安全属性永不脱钩。

**对 Aster 的启示**：Rust trait 很适合表达这个。`isReadOnly()` 和 `checkPermissions()` 放在 trait 里，编译器强制每个工具都回答这两个问题。

### 2.3 上下文压缩：五层递进

这是整个系统最难的工程问题。每次调模型前依次尝试：

| 层 | 机制 | 说明 |
|----|------|------|
| 1 | 预算裁剪 | 单个工具结果大小上限 |
| 2 | Snip | 删除历史中已无用的中间消息 |
| 3 | Microcompact | 合并连续的工具调用/结果对为摘要 |
| 4 | Context Collapse | 只读投影——不改原始存储，模型看到压缩版 |
| 5 | Auto-Compact | 模型自己生成对话摘要（最贵，最后手段）|

**对 Aster 的启示**：先做第 1 层和第 5 层。中间层可以后补。

### 2.4 权限模型：默认拒绝

```
Plan（只读）→ Default（每次确认）→ Accept Edits（编辑自动批）→ Auto（ML 评分）
```

允许规则和拒绝规则同时存在，拒绝规则优先级更高。

**对 Aster 的启示**：个人工具不需要 ML 评分。三层就够：Plan（只读）、Ask（确认）、Allow（自动批准）。用户通过配置文件声明哪些工具/路径自动放行。

### 2.5 子 Agent + Git Worktree 隔离

复杂任务拆给多个子 Agent，每个跑在独立的 Git worktree 里。做完后 diff 审查，合并成功的，丢弃失败的。

**对 Aster 的启示**：这是 Phase 5 的事。但设计工具系统时要预留子 Agent 作为一等工具类型的扩展点。

### 2.6 流式工具执行

工具不在模型输出结束后才开始——模型还在生成 token 时，已确定的工具调用就开始执行。减少端到端延迟。

**对 Aster 的启示**：这需要 SSE 流式解析和工具调用检测并行。Rust 的 async/tokio 很适合。

---

## 5. 分阶段演进路线

每个 Phase 都有明确的**输入**（依赖什么）、**产出**（交付什么）、**不做**（边界在哪）。

### Phase 1：工具系统基座 + ofsh 子进程嵌入

**目标**：定义 Tool trait，实现 5 个基础工具（含 ofsh 子进程版 ShellTool），让模型能读文件、搜代码、跑命令、写文件。

**输入**：当前 `main.rs` 的 SSE 流式聊天 + `arbor-tui` 渲染管线 + ofsh 的 `@obolosfs/ofsh` 包。

**产出**：

- `tool.rs`：`Tool` trait 定义
  ```rust
  pub trait Tool: Send + Sync {
      fn name(&self) -> &str;
      fn description(&self) -> &str;
      fn input_schema(&self) -> serde_json::Value;  // JSON Schema
      fn is_read_only(&self) -> bool;
      fn is_concurrency_safe(&self) -> bool;
      fn call(&self, args: serde_json::Value) -> ToolResult;
  }
  ```
- `tools/` 目录，5 个内置工具：
  - `ReadTool` — 读文件，`is_read_only = true`
  - `WriteTool` — 写文件，`is_read_only = false`
  - `GrepTool` — ripgrep 内容搜索，`is_read_only = true`
  - `GlobTool` — 文件名模式匹配，`is_read_only = true`
  - `ShellTool` — 通过 ofsh 子进程执行命令。启动时 spawn Node.js 进程，JSON-RPC over stdin/stdout 通信。ofsh session 挂载当前工作目录为 `/workspace`，默认只读。写操作需声明 `allow_write: true`
- `ToolRegistry` — 工具注册、Schema 聚合、按名查找
- `ofsh-bridge.rs` — ofsh 子进程管理
  ```rust
  struct OfshBridge {
      child: std::process::Child,
      stdin: BufWriter<ChildStdin>,
      stdout: BufReader<ChildStdout>,
  }
  impl OfshBridge {
      fn spawn() -> Result<Self>;                    // 启动 Node.js ofsh agent
      fn exec(&mut self, line: &str, mounts: &[Mount]) -> Result<String>;
      fn register_command(&mut self, name: &str, handler: ...) -> Result<()>;
      fn shutdown(&mut self) -> Result<()>;
  }
  ```
- 模型请求 payload 中注入 `tools` 字段（OpenAI function calling 协议）
- 解析模型返回的 `tool_calls`，执行工具，收集结果
- `skill.rs`：SKILL 文件解析器
  ```rust
  struct Skill {
      name: String,
      description: String,
      tools: Option<Vec<String>>,    // 工具白名单
      mounts: Option<Vec<Mount>>,    // VFS 挂载点
      params: HashMap<String, ParamDef>,
      model: Option<String>,
      requires: Vec<String>,
      body: String,                  // Markdown 指令体
  }
  fn load_skills(dir: &Path) -> Vec<Skill>;
  fn resolve_skills(skills: &[Skill], activate: &[String]) -> Vec<Skill>; // 处理 requires 级联
  ```
- `/skill-name` 斜杠命令支持——TUI 输入框里打 `/rust-code-review` 激活对应 skill

**不做**：
- 不做 Agent Loop（还是单轮，只是模型可以调工具了）
- 不做权限控制（Phase 1 默认全部允许，个人工具先跑通）
- 不做并发工具执行（串行调用即可）
- 不做 MCP 集成
- 不做 Skill 自动发现（只加载手动指定的 skill 文件）

**验收**：在 Aster TUI 里输入 `/rust-code-review` 激活对应 skill，然后说"审查一下 tool.rs"，模型能通过 ReadTool 读取文件并给出审查意见。输入"列出当前目录的文件"，模型能通过 ShellTool 调 ofsh 的 `ls /workspace` 返回结果。

### Phase 2：Agent Loop

**目标**：实现多步推理循环。模型调工具 → 拿到结果 → 继续思考 → 再调工具 → 直到输出最终答案。同时支持 SKILL 激活时自动注入系统提示词。

**输入**：Phase 1 的工具系统 + SKILL 解析器。

**产出**：

- `agent.rs`：Agent Loop 状态机
  ```rust
  enum LoopState {
      Thinking,                          // 等待模型响应
      ToolCalling(Vec<ToolCall>),        // 执行工具中
      WaitingInput,                      // 等待用户确认（权限）
      Done(String),                      // 最终回复
  }

  fn agent_loop(
      client: &dyn LlmClient,
      tools: &ToolRegistry,
      messages: &mut Vec<Message>,
      max_turns: usize,                  // 防止无限循环
  ) -> Result<String>
  ```
- 流式工具执行：模型输出 token 时，检测到完整 tool_call JSON 块就提前开始执行
- 工具结果注入回 `messages`，携带 `role: "tool"` + `tool_call_id`
- 兜底：max_turns 到上限后强制结束，注入"请基于已有信息回答"的系统消息
- SKILL 注入：Agent Loop 启动时，根据激活的 skill 列表，将对应的 markdown 指令体注入系统消息。如果 skill 声明了 `tools` 白名单，只暴露白名单中的工具给模型

**不做**：
- 不做上下文压缩（长对话就让它失败，先看到失败模式）
- 不做子 Agent
- 不做会话持久化

**验收**：输入"帮我看看这个项目的测试覆盖率"，模型多步执行：grep 找测试文件 → bash 跑 cargo test → 汇总结果。

### Phase 3：上下文管理与会话持久化

**目标**：长对话不崩，退出后能恢复。

**输入**：Phase 2 的 Agent Loop。

**产出**：

- `context.rs`：上下文预算管理
  - 工具结果长度上限（单个工具结果最大 N 字符，超出截断 + 标记）
  - 简单 token 估算（字符数 / 3.5，不需要精确 tokenizer）
  - 接近窗口上限时自动触发压缩
- `compact.rs`：对话摘要（Auto-Compact）
  - 用模型自身生成历史摘要
  - 摘要格式：`<summary>关键决策、已完成的工具调用、待解决问题</summary>`
  - 触发条件：估算 token 超过阈值（例如 80% 窗口）
- `session.rs`：会话持久化
  - 会话存储为 JSONL 文件（`~/.aster/sessions/<id>.jsonl`）
  - 每行一条消息（含角色、内容、时间戳、tool_call_id）
  - 启动时恢复上次会话，`--new` 开新会话
  - 会话列表命令 `--sessions`

**不做**：
- 不做 Snip / Microcompact 等中间压缩层（先只用 Auto-Compact）
- 不做向量化记忆（不做 RAG）
- 不做 Context Collapse（只读投影）——复杂度太高，个人场景收益不大

**验收**：连续对话 50 轮不崩，退出重进后恢复上次对话。

### Phase 4：权限与安全

**目标**：危险操作可控，不用每次都手动确认。

**输入**：Phase 2 的 Agent Loop + Phase 1 的工具系统。

**产出**：

- `permissions.rs`：三层权限
  ```rust
  enum Permission {
      Allow,    // 自动放行
      Ask,      // TUI 内弹确认框
      Deny,     // 直接拒绝
  }
  ```
- 权限规则文件 `~/.aster/permissions.toml`：
  ```toml
  [rules]
  # 只读工具在工作目录下自动放行
  allow = [
    { tool = "read", path = "~/code/*" },
    { tool = "grep", path = "~/code/*" },
  ]
  # 写操作在非工作目录下直接拒绝
  deny = [
    { tool = "write", path = "~/.ssh/*" },
    { tool = "bash", command = "rm -rf /*" },
  ]
  ```
- TUI 确认组件：工具调用在终端底部弹出确认条，Y/n/q 三选一
- 工具结果注入拒绝标记（模型看到"用户拒绝了此操作"）
- `--plan` 模式：所有写工具强制 Deny

**不做**：
- 不做 ML 自动评分（Auto 模式）——个人工具不需要
- 不做沙箱（不引入容器/虚拟机隔离）
- 不做 hook 系统（PreToolHook / PostToolHook）

**验收**：`--plan` 模式下模型可以读文件但不能写文件。正常模式下，工作目录内的编辑自动放行，`~/.ssh` 下的写操作被拒绝。

### Phase 5：项目感知

**目标**：Aster 启动时自动理解项目结构，不需要用户每次解释上下文。

**输入**：Phase 3 的上下文管理系统。

**产出**：

- `project.rs`：项目上下文加载
  - 从当前目录向上查找 `CLAUDE.md` / `AGENTS.md` / `README.md`
  - 读 `.gitignore` 并排除对应文件
  - 读 `Cargo.toml` / `package.json` 识别项目类型
  - 汇总为一条系统消息，注入到对话开头
- 文件选择策略：优先展示项目指引文件，跳过 `target/`、`node_modules/`
- 每次新会话自动重新扫描（文件可能已变更）

**不做**：
- 不做增量更新监听（不 watch 文件变更）
- 不做 LSP 集成（不接入 Language Server）
- 不做多项目 workspace 感知

**验收**：在 arbor 仓库根目录启动 aster，第一轮对话模型就知道项目结构，不需要用户描述。

### Phase 6：多模型 + 子 Agent

**目标**：支持切换模型提供商，简单任务用便宜模型。独立子 Agent 并行处理。

**输入**：Phase 2 的 Agent Loop + Phase 5 的项目感知。

**产出**：

- `providers/`：模型提供商抽象
  ```rust
  trait LlmProvider {
      fn chat_stream(&self, messages: &[Message], tools: &[ToolDef]) -> Stream;
      fn model_name(&self) -> &str;
      fn context_window(&self) -> usize;
  }
  ```
  - `DeepSeekProvider`（已有）
  - `AnthropicProvider`（Anthropic Messages API）
  - `OpenAiProvider`（OpenAI Chat Completions API）
  - `OllamaProvider`（本地模型）
- 配置文件 `~/.aster/config.toml`：
  ```toml
  [models]
  default = "deepseek-chat"
  fast = "claude-haiku-4-5"       # 简单任务用
  heavy = "claude-opus-4-8"       # 复杂任务用
  ```
- `agent.rs`：子 Agent 派生
  - `AgentTask` 类型，描述子任务 + 可用工具子集
  - 子 Agent 跑在独立 tokio task 里，通过 mpsc channel 回传结果
  - 简单场景串行，独立子任务并行（例如同时搜两个文件）
- 子 Agent 的结果汇总后作为工具结果注入主 Agent 的消息历史

**不做**：
- 不做 Git worktree 隔离（复杂度太高，先只在同一文件系统上工作）
- 不做跨 Agent 通信（不搞 coordinator 模式）
- 不做 budget 系统（不限 token 花费）

**验收**：`--model claude-sonnet-4-6` 切换到 Sonnet。输入"同时查一下 api.rs 和 chat.rs 的错误处理"，两个子 Agent 并行读取文件。

### Phase 7：MCP 集成

**目标**：通过 Model Context Protocol 接入外部工具服务器。

**输入**：Phase 1 的工具系统 + Phase 6 的多模型支持。

**产出**：

- `mcp.rs`：MCP 客户端
  - stdio 传输（本地子进程）
  - 工具发现：`tools/list` → 动态注册到 ToolRegistry
  - 工具调用：`tools/call` → 透传结果
  - 命名约定：`mcp__<server>__<tool>` 避免冲突
- MCP 服务器配置在 `~/.aster/config.toml`：
  ```toml
  [[mcp_servers]]
  name = "filesystem"
  command = "npx"
  args = ["-y", "@anthropic/mcp-filesystem", "."]
  ```

**不做**：
- 不做 SSE/HTTP 远程 MCP 服务器（先只做 stdio 本地）
- 不做 OAuth 认证

**验收**：启动时加载 MCP 文件系统服务器，模型可以调用 MCP 提供的文件操作工具，和内置工具一起出现。

---

## 6. 不做的事

有些事明确不做，避免把项目做重：

- **不做插件市场**——工具通过 MCP 接入，不自己造插件格式
- **不做 VS Code / JetBrains 插件**——Aster 是终端工具，不和 IDE 抢地盘
- **不做多租户 / 团队功能**——个人工具，没有"组织"概念
- **不做 Web UI**——终端就是它的界面
- **不做遥测 / 分析**——不收集使用数据
- **不做 Windows 优先**——开发在 Windows 上，但要保证 Linux/macOS 能跑
- **不做守护进程 / 后台 Agent**——Aster 是用户主动打开、主动关闭的工具

---

## 7. 技术决策记录

### 7.1 Agent Loop 用同步状态机，不用 async generator

Rust 的 async generator 还不够成熟（`async gen` 仍在 nightly）。用一个显式的 `LoopState` 枚举 + loop 更可控、更好调试。

### 7.2 工具 Schema 用 JSON Schema（不用 Rust 宏推导）

虽然 Rust 的 `#[derive(JsonSchema)]` 很诱人，但手写 JSON Schema 有几个好处：

- 和 OpenAI / Anthropic API 的协议完全一致，不需要转换层
- Schema 里的 `description` 字段是写给模型看的 prompt 工程，不是自动生成能搞定的
- 工具数量少（<20 个），手写成本可接受

### 7.3 不引入 async runtime 到 TUI 主循环

当前 `arbor-tui` 是同步渲染管线。流式 API 调用已经是后台线程 + mpsc channel。Agent Loop 中的工具调用也走同样的模式：后台线程执行，channel 回传，主循环 poll。这避免了在渲染循环里引入 tokio。

### 7.4 DeepSeek 优先，不锁定 Anthropic

Aster 是个人工具，DeepSeek 的价格优势对个人使用很重要。所有模型相关代码走 trait 抽象，不硬编码 Anthropic 的 API 细节。OpenAI 兼容协议是默认接口。

### 7.5 ofsh 集成先用子进程，验证后再考虑移植

ofsh（TypeScript）目前不移植到 Rust。原因：

- ofsh 的 VFS + Shell + 命令体系约 2000 行 TS，移植成本约 2-3 周
- 先用 JSON-RPC 子进程嵌入跑通整个 Agent Loop，验证沙箱模型在真实场景中好用
- 移植时保留 ofsh 的接口设计（Driver trait、Vfs API、CommandContext），这些设计已经过 ObolosFS 项目的测试验证
- 如果未来 ofsh 自身演进出新的能力，子进程方案能更快跟上，Rust 移植可能滞后

### 7.6 SKILL 用 YAML frontmatter + Markdown body，不走纯 markdown 约定解析

Claude Code 的 SKILL.md 是纯 markdown，元数据靠约定解析（例如标题、代码块、列表）。aster 需要结构化元数据（工具白名单、挂载权限、参数定义），用 YAML frontmatter 更可靠。

YAML frontmatter 的好处：

- 解析器简单——`serde_yaml` 一行反序列化，不需要 NLP 级别的 markdown pattern matching
- Schema 验证——写 skill 时如果 frontmatter 格式不对，解析器直接报错，不会静默失败
- 工具链友好——可以写 `aster skill validate ./skills/` 检查所有 skill 文件

代价是写 skill 时多一个 YAML 头部。对于个人工具，这个成本可接受。

---

## 8. 当前状态与下一步

| Phase | 状态 | 预计开始 |
|-------|------|----------|
| 1 — 工具系统基座 + ofsh 子进程嵌入 + SKILL 格式 | 未开始 | — |
| 2 — Agent Loop + SKILL 激活 | 未开始 | 依赖 Phase 1 |
| 3 — 上下文管理与会话持久化 | 未开始 | 依赖 Phase 2 |
| 4 — 权限与安全 | 未开始 | 依赖 Phase 1+2 |
| 5 — 项目感知 | 未开始 | 依赖 Phase 3 |
| 6 — 多模型 + 子 Agent | 未开始 | 依赖 Phase 2+5 |
| 7 — MCP 集成 | 未开始 | 依赖 Phase 1+6 |
| ofsh Rust 移植 | 未开始 | 依赖 Phase 1-3 验证通过 |

Phase 1 是最小可行下一步。做完后 aster 就能从"聊天"变成"能读文件、能跑命令的聊天"——这是从 Chat 到 Agent 的第一个质变。

---

## 资料来源

- [Claude Code from Scratch — 23 Component Architecture](https://github.com/FareedKhan-dev/claude-code-from-scratch)
- [cc-haha — Claude Code Agent Framework Deep Dive](https://github.com/NanmiCoder/cc-haha)
- [how-claude-code-works — Tool System Architecture](https://github.com/Windy3f3f3f3f/how-claude-code-works)
- [Claude Code Internals — 10-Lesson Deep Dive](https://github.com/yaniv-golan/claude-code-internals)
- [Claude Code Architecture — WaveSpeed Blog](https://wavespeed.ai/blog/posts/claude-code-architecture-leaked-source-deep-dive/)
- [Anthropic: Getting Started with Loops](https://claude.com/blog/getting-started-with-loops)
- [cwc-long-running-agents — Anthropic Official](https://github.com/anthropics/cwc-long-running-agents)
- [OpenDev — Rust Terminal Coding Agent (arXiv:2603.05344)](https://export.arxiv.org/html/2603.05344)
- [behest — Rust Agent Runtime](https://github.com/lazhenyi/behest)
- [agent-base — Rust Agent Kernel](https://github.com/chenkangzeng1/agent-base)
- [ObolosFS/ofsh — VFS Shell（本地项目）](C:\Users\nyml\code\ObolosFS\packages\ofsh)
- [ObolosFS/core — 虚拟文件系统核心（本地项目）](C:\Users\nyml\code\ObolosFS\packages\core)
