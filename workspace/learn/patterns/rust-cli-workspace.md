# 模式：Rust CLI + Domain 分离（forgebench）

## 一句话

一个 Rust CLI 工具的标准结构：clap 命令定义 → domain 纯类型 → infrastructure 文件系统/进程 → output 格式分离。CLI 层不写业务逻辑。

## 核心架构

```
main.rs
  │
  ▼
cli.rs (clap Parser + Subcommand)
  │
  ├── local.rs    → EntryKind 枚举，文件系统遍历
  │
  └── skill.rs    → SkillSummary + SkillLintPayload
       │
       ├── SkillIssue { level, message, path }
       ├── SkillBlock { name, kind, path }
       └── SkillLintPayload { skill_count, issue_count, skills, issues }
  │
  ▼
output.rs (print_error, print_success)
config.rs (WorkbenchConfig)
error.rs (AppError + AppErrorCode)
system.rs (系统调用封装)
```

## 关键设计

### 1. 命令层级：clap Subcommand 嵌套

```rust
Cli
├── Local(LocalArgs)
│   └── LocalCommands::Read { path, ... }
│   └── LocalCommands::List { ... }
│
└── Skill(SkillArgs)
    └── SkillCommands::Lint { name }
    └── SkillCommands::Inspect { name }
```

两层 Subcommand：顶层分 `local` 和 `skill` 两个域，每个域有自己的子命令。不需要在代码里手动匹配字符串——clap 自动解析并分派到对应的枚举变体。

### 2. Domain 类型：纯数据，可序列化

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SkillIssue {
    pub level: String,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLintPayload {
    pub skill_count: usize,
    pub issue_count: usize,
    pub skills: Vec<SkillSummary>,
    pub issues: Vec<SkillIssue>,
}
```

所有输出类型都 derive `Serialize`——CLI 可以输出 JSON 而不需要手写 formatter。`SkillLintPayload` 携带了统计信息（`skill_count`、`issue_count`），调用方不需要自己数。

### 3. 错误类型：错误码 + 嵌套

```rust
pub enum AppErrorCode {
    ConfigNotFound,
    ConfigParseFailed,
    SkillNotFound,
    IoError,
    // ...
}

pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
}
```

不是笼统的 `anyhow::Error`——`AppErrorCode` 让调用方可以按错误类型做不同处理（如 ConfigNotFound 提示初始化，SkillNotFound 建议检查名称）。

### 4. 输出分离：print vs data

```rust
// output.rs
pub fn print_error(err: &AppError) { ... }
pub fn print_success(message: &str) { ... }
```

命令函数返回数据结构（`Result<Payload, AppError>`），output 层负责格式化。要加 JSON 输出只需加一个 `--json` flag → 换一个 output formatter。

### 5. EntryKind：类型安全的文件类型

```rust
pub enum EntryKind {
    Skill,
    Repo,
    Unknown,
}
```

遍历目录时不是返回 `String`——返回 `EntryKind` 枚举。后续 match 穷尽所有变体，编译器检查遗漏。

## 和 workshop/workc 的对比

| 维度 | forgbench | workshop |
|------|----------|----------|
| 语言 | Rust | Rust |
| CLI 框架 | clap | clap |
| 领域层 | domain 类型 + 校验逻辑 | full DDD aggregate + repository |
| 复杂度 | 轻量（一个 CLI + 文件遍历） | 中等（任务生命周期 + workspace 管理） |

forgebench 是 workshop 的轻量版本——同样用 clap + domain 分离，但只有一个 CLI 入口，没有多 crate workspace。

## 来源

- forgbench 源码（`src/cli.rs`、`src/skill.rs`、`src/output.rs`、`src/error.rs`）
- 2026-06-07 agent 阅读后提炼
