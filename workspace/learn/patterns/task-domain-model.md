# 模式：任务领域模型（workshop/workc）

## 一句话

用 DDD 的 Aggregate Root + Repository + 状态机，把任务管理的领域逻辑封装为纯 Rust 类型，CLI 和存储都是薄壳。

## 为什么需要领域模型

任务管理看起来简单（CRUD），但现实中：

- 任务有状态流转规则（Draft → Active → Closed | Archived）
- 状态转换有业务约束（不能关闭一个已关闭的任务）
- 每个操作要更新多个时间戳（created_at, updated_at, last_opened_at）
- 任务关联仓库选择（repo groups + explicit repos）
- CLI 和 GUI 需要读同一套数据、遵循同样的规则

如果逻辑散落在 CLI handler 和存储之间，每个入口都要重复状态校验 → 不一致。做领域模型的核心收益：**业务规则只写一次，所有入口（CLI、GUI、agent）都过同一扇门。**

## 核心架构

```
CLI (clap + presenter)
  │
  ▼
Application Service (use cases: create_task, list_tasks, close_task)
  │
  ▼
Domain (TaskWorkspace aggregate, TaskStatus state machine, TaskRepository trait)
  │
  ▼
Infrastructure (FsTaskRepository, SystemClock, MemoryFileSystem for tests)
```

**依赖方向**：外层依赖内层。Domain 不知道 CLI 和文件系统的存在。

## 关键设计

### 1. Newtype ID（类型安全的标识符）

```rust
// Rust macro 生成，四行代码搞定一个 ID 类型
macro_rules! nanoid_id {
    ($name:ident) => {
        pub struct $name(String);
        impl $name {
            pub fn generate() -> Self { Self(nanoid::nanoid!(8)) }
        }
        // Display, From<String>, From<&str>, Serialize, Deserialize...
    };
}

nanoid_id!(TaskSlug);
nanoid_id!(RepoId);
nanoid_id!(RepoGroupId);
```

**效果**：
- `TaskSlug` 和 `RepoId` 不能互相赋值（编译期拦住）
- 和自己用 `String` 一样轻（零开销包装）
- 8 位 nanoid，够短够唯一

**TS 等价**：ObolosFS 的 brand type 模式（`string & { [brand]: true }`），或者直接用 template literal type。

### 2. TaskWorkspace Aggregate Root

只有这一个类型暴露 mutation 方法。外部不能直接改 `TaskMeta.status`。

```rust
pub struct TaskWorkspace {
    pub meta: TaskMeta,            // slug, title, status, description, tags
    pub repos: TaskRepoSelection,  // selected repo groups + repos
    pub activity: TaskActivity,    // created_at, updated_at, last_opened_at, last_editor
    pub paths: TaskPaths,         // 子目录布局
}

impl TaskWorkspace {
    pub fn create(...) -> Result<Self, DomainError> { ... }  // 工厂方法
    pub fn mark_opened(&mut self, occurred_at, editor) { ... }  // 记录打开
    pub fn close(&mut self, occurred_at) -> Result<(), DomainError> { ... }  // 状态转换
}
```

**原则**：
- `create()` 是唯一构造入口，内调 `TaskMeta::new()` 做所有验证
- `close()` 检查当前状态 → 不能关一个已关的，不能关一个已归档的 → 返回 `Conflict` 错误
- `mark_opened()` 是无条件更新（打开一个任务不需要状态检查）

### 3. TaskStatus 状态机

```rust
pub enum TaskStatus {
    Draft,
    Active,
    Closed,
    Archived,
}
```

**转换规则由 Aggregate 的方法 enforce**：

| 当前状态 | 操作 | 结果 |
|---------|------|------|
| Draft/Active | `close()` | → Closed |
| Closed | `close()` | Conflict: "already closed" |
| Archived | `close()` | Conflict: "archived tasks cannot be closed" |

状态机不在一个独立文件里，不在一个配置表里——**就写在 aggregate 方法里**。这是最简单的状态机实现：方法体内的 if/match 就是转换规则。

### 4. DomainError 分类

```rust
pub enum DomainError {
    NotFound       { entity: EntityKind, slug: String },
    AlreadyExists  { entity: EntityKind, slug: String },
    InvalidInput   { field: FieldKind, reason: String },
    Conflict       { entity: EntityKind, reason: String },
    PersistenceFailed { operation: &'static str, detail: String },
}
```

**每个变体携带足够的上下文信息用于展示**：
- CLI 可以把 `Conflict` 打印成 `"Task conflict: already closed"`
- UI 可以针对不同错误类型显示不同视觉反馈
- 不是笼统的 `throw new Error("something wrong")`

**TS 等价**：用 discriminated union（和 ObolosFS Result 一样的模式）：
```typescript
type DomainError =
  | { type: 'NotFound'; entity: EntityKind; slug: string }
  | { type: 'Conflict'; entity: EntityKind; reason: string };
```

### 5. Repository 抽象

```rust
pub trait TaskRepository {
    fn find(&self, slug: &TaskSlug) -> Result<Option<TaskWorkspace>, DomainError>;
    fn list(&self) -> Result<Vec<TaskWorkspace>, DomainError>;
    fn save(&self, task: &TaskWorkspace) -> Result<(), DomainError>;
}
```

- Trait 定义在 domain 层（不依赖任何存储实现）
- `FsTaskRepository` 实现在 infrastructure 层（读/写 TOML 文件）
- `MemoryFileSystem` 用于测试——记录所有文件操作，可以断言序列化结果

**测试不需要碰真实文件系统**：
```rust
#[test]
fn task_create_and_list() {
    let memfs = MemoryFileSystem::new();        // ← 内存文件系统
    let ctx = test_context(&memfs);
    run_command(cli, ctx).unwrap();
    // 检查写入的 TOML 内容
    assert!(toml.contains("my-task"));
}
```

### 6. CLI 的 Presenter 模式

```
CLI 命令 → Application Service → 返回数据
                                      │
Presenter ◄───────────────────────────┘
  │
  ├── TextPresenter  → 人类可读输出
  └── JsonPresenter  → 机器可读输出
```

```rust
let presenter: Box<dyn Presenter> = if cli.json {
    Box::new(JsonPresenter)
} else {
    Box::new(TextPresenter)
};
// 命令逻辑不关心输出格式
Ok(presenter.render_task_list(&items))
```

**好处**：加一个 `MarkdownPresenter` 或 `TerminalColorPresenter` 不需要改任何命令逻辑。

## Clean Architecture 层间依赖

```
domain          ← 纯 Rust，零外部依赖（除了 serde）
  ↑
application     ← use cases + DTOs，依赖 domain
  ↑
infrastructure  ← FsTaskRepository, SystemClock, EditorLauncher
  ↑
cli             ← clap + presenter，依赖 application + infrastructure
```

关键约束：
- **domain 不引入 anyhow** —— 领域错误用 `DomainError`，不是笼统的 `anyhow::Error`
- **application 用 anyhow** —— 组合多个 domain service 的调用，用 `anyhow::Result` 传递
- **infrastructure 知道所有平台细节**（`#[cfg(target_os = "windows")]`）

## 反模式警示

### ❌ 在 CLI handler 里写业务规则

```typescript
// 不要这样做
if (task.status === 'closed') {
    console.log('Task already closed');
    return;
}
task.status = 'closed';
```

这段逻辑应该在一个 `Task.close()` 方法里，所有调用方共享。

### ❌ 用字符串做 ID

```typescript
function closeTask(taskId: string) { ... }  // ← 任何 string 都能传进去
```

应该用 brand type 或 template literal 让编译器区分 `TaskId` 和 `RepoId`。

### ❌ 笼统的错误类型

```typescript
throw new Error('something wrong');  // ← 调用方没法区分处理
```

应该用 discriminated union 把错误分类（NotFound / Conflict / InvalidInput）。

## 来源

- workshop/workc 源码（`crates/workc-domain/`、`crates/workc-cli/`）
- 2026-06-07 agent 深度阅读后提炼
