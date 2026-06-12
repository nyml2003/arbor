# 模式：Skill 校验管线与 Python 领域建模（work-context）

## 一句话

用 Python dataclass 做不可变领域模型，用一个分步骤的校验管线（多个 `collect_*_issues` 函数）做语义检查，Result/Option 类型来自 Rust 的设计移植。

## 跨语言模式确认

这是一个重要的信号：**同一个模式在第三种语言里再次出现。**

| 模式 | Rust (workshop) | TS (ObolosFS) | Python (work-context) |
|------|----------------|---------------|----------------------|
| Result type | `Result<T, E>` | `Result<T, FsError>` | `Result[T, E]` |
| Option type | `Option<T>` | — | `Option[T]` |
| 纯领域类型 | struct | interface/type | `@dataclass(frozen=True)` |
| 领域错误分类 | `DomainError` enum | `FsErrorCode` const | `AppError` |

**三种语言，同一个想法。这是可移植的模式，不是语言特化。**

## 核心架构

```
CLI (click/argparse)
  │
  ▼
Composition/Runtime (依赖注入容器)
  │  build_service_container()
  │  RuntimeContext (懒加载 + 缓存)
  ▼
Application Services (use cases)
  │  SkillService, ContextService, ReportService, ...
  ▼
Domain (dataclass 不可变对象)
  │  Skill, SkillFrontmatter, SkillBlock, SkillIssue, ...
  ▼
Infrastructure
    文件系统、Git、模板引擎、进程执行
```

## 关键设计

### 1. Python 版 Result 和 Option

```python
@dataclass(frozen=True)
class Result(Generic[T, E]):
    _state: str       # "ok" | "err"
    _payload: object  # T | E

    @classmethod
    def ok(cls, value: T) -> Result[T, E]: ...
    @classmethod
    def err(cls, error: E) -> Result[T, E]: ...

    def map(self, mapper) -> Result[U, E]: ...
    def and_then(self, mapper) -> Result[U, E]: ...
    def unwrap_or(self, default: T) -> T: ...
```

和 Rust/ObolosFS 版本的核心差异：
- Python 没有 discriminated union → 用 `_state` 字符串 + `_payload` 模拟
- `frozen=True` 确保不可变
- `Generic[T, E]` 保持类型安全

### 2. 领域模型：frozen dataclass 三原则

```python
@dataclass(frozen=True, slots=True)     # ← 不可变 + 内存优化
class SkillFrontmatter:
    name: str
    description: str
    role_fit: list[str]
    domain_tags: list[str]
    capabilities: list[str]
    default_blocks: list[str]
    recommends: list[str]
    handoff_outputs: list[str]
    blocks: list[SkillBlock]
    license: str | None = None           # ← Optional 用 | None，不用 Optional[]
```

**三原则**：
1. `frozen=True` — 值对象不可变，创建后不能改
2. `slots=True` — 减少每个实例的内存占用（Python 3.10+）
3. 字符串用 `str`（不是 `Optional[str]`），真正可选的用 `str | None`

### 3. 校验管线：分步骤检查

```
collect_frontmatter_issues(skill)     ← 检查 name/description/role/structure
        │
collect_agents_issues(skill)          ← 检查 agents/openai.yaml 的完整性
        │
collect_resource_reference_issues(skill) ← 检查引用的文件是否真实存在
        │
collect_fixture_issues(skill)         ← 检查 example/test JSON 格式
        │
collect_recommendation_issues(skill)  ← 检查 recommends 的引用完整性
        │
        ▼
collect_skill_issues(skill)           ← 汇总 → SkillLintPayload
```

**每个检查函数自包含**，互不依赖。新增一个检查维度只需加一个函数 → 在 `collect_skill_issues` 里加一行 `issues.extend(...)`。

**校验项的粒度**：
```python
# 命名规范
NAME_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

# 目录名和 skill 名必须一致
if skill.path.name != skill.name:
    issues.append(issue("error", f"Directory name does not match skill name", ...))

# 引用的资源文件必须存在
if not candidate.exists():
    issues.append(issue("error", f"Referenced resource does not exist", ...))

# Test fixture 必须是 JSON object
if not isinstance(fixture, dict):
    issues.append(issue("error", "Test fixture must be a JSON object", ...))
```

### 4. 元数据允许列表

```python
ALLOWED_FRONTMATTER_KEYS = {"name", "description", "license", ...}
ALLOWED_METADATA_KEYS = {"short-description", "workbench"}
ALLOWED_WORKBENCH_METADATA_KEYS = {"role-fit", "domain-tags", "capabilities", ...}
```

**为什么需要**：YAML front matter 是自由格式。如果用户打错了键名（如 `descriptoin` 而不是 `description`），不会有编译错误。Allowlist 可以捕获这些拼写错误。

### 5. 懒加载 Composition Root

```python
class RuntimeContext:
    def config(self) -> Result[WorkbenchConfig, AppError]:
        if self.cached_config is not None:     # ← 缓存命中
            return Result.ok(self.cached_config)
        config = load_config(self.repo_root)
        # ... 验证 + 缓存
        self.cached_config = config.value
        return Result.ok(self.cached_config)

    def services(self) -> Result[ServiceContainer, AppError]:
        if self.cached_services is not None:    # ← 再缓存一层
            return Result.ok(self.cached_services)
        # ... 装配
```

**和 workshop 的对比**：
- workshop (Rust)：编译期 DI，trait + generic
- work-context (Python)：运行时 DI，懒加载 + 手动缓存

同一个问题（依赖装配），不同语言的实现策略不同。但核心原则一致：**依赖在入口装配，不在业务逻辑里 new。**

## 来源

- work-context 源码（`src/workbench/core/`、`src/workbench/domain/`、`src/workbench/application/skill_validation.py`、`src/workbench/composition/runtime.py`）
- 2026-06-07 agent 阅读后提炼
