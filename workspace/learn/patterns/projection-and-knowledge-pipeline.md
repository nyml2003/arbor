# 模式：主数据投影与知识管道（work-context-2）

## 一句话

把「主数据」和「对外投影」拆成两层——主数据是单一真相源，投影是面向目标的派生品。知识从 Task 中抽取，经过审核管道流向全局知识库。

## 核心概念

### 1. 投影模式（Master → Projection → Target）

```
Skill（主数据）
  │
  ▼
SkillVersion（内容快照）
  │
  ▼
SkillProjection（面向 target 的投影）
  │
  ▼
SkillTarget（发布目标：.codex / .cursor / ...）
```

**projection 是派生物，可重建，不应反向成为主数据。** 发布目标（.codex, .cursor）下的内容由 SkillProjection 生成，主数据始终在 Skill 和 SkillVersion。

### 2. 知识管道（Task Memory → Global Knowledge）

```
Task Memory（属于 task）
  │  抽取
  ▼
Knowledge Candidate（draft → reviewing → approved → published）
  │
  ▼
Global Knowledge（全局资产）
```

经验从单个 task 的 memory 中抽取 → 审核 → 进入全局知识库。不是做了就入库——要走审核管道。

### 3. 「先冻结，后实现」的领域建模方法

work-context-2 的文档在写任何代码之前，先冻结了：
- 7 个核心对象及其字段
- 2 个状态机及其合法流转
- 对象间的关系图
- 明确的 V1 不做清单（如不引入 `paused` 状态）

## 核心架构

### 对象分层

```
全局稳定对象      Repo, Repo Pack           ← 平台不变事实
Task 实例对象     Task, Workspace, Binding   ← 一次工作的运行时
经验对象          Task Memory, Knowledge     ← 从实例中抽取
Skill 管理对象    Skill, Version, Target     ← 主数据 + 投影
```

### 一级能力

| 能力 | 职责 |
|------|------|
| Delivery | Task 生命周期、发布建议 |
| Skills | 技能注册、索引、装配、投影同步 |
| Knowledge | 记忆抽取、审核、发布、检索 |

## 关键设计

### State Machine：先设计流转再写代码

Task 状态机（9 状态）：

```
                 ┌──────────┐
                 │  draft   │
                 └────┬─────┘
                      ▼
                 ┌──────────┐     ┌───────────┐
                 │  active   │────▶│  blocked   │
                 └────┬─────┘◀────└───────────┘
                      │
          ┌───────────┼───────────┐
          ▼           ▼           ▼
    ┌──────────┐ ┌─────────┐ ┌──────────┐
    │validating│ │cancelled│ │   done   │
    └────┬─────┘ └─────────┘ └────┬─────┘
         │                        ▼
         ▼                   ┌──────────┐
    ┌──────────────┐         │ archived  │
    │ready-to-rel. │         └──────────┘
    └──────┬───────┘
           ▼
    ┌──────────┐
    │ releasing│
    └──────────┘
```

**状态迁移的唯一入口**：`TaskStore.transition_task_status()` —— 所有调用方必须经过同一个状态机校验。

**明确的设计取舍**：
- V1 **不**引入 `paused`
- `done` ≠ `archived`：done 是业务闭环，archived 是只读归档
- 从任意非终态都可以 `cancel`

### Knowledge Candidate 状态机

```
draft → reviewing → approved → published → superseded
              ↘
              rejected
```

这是一个**审核管道**，不是简单发布：
- `draft`：刚从 task memory 抽取，未审核
- `reviewing`：校验适用范围和表达质量
- `approved → published`：进入全局知识库
- `rejected`：不进入全局（但可能保留在 task memory）
- `superseded`：被更高质量的替代

### Schema Versioning

每个实体都携带 `schema_version`：

```yaml
Skill:
  schema_version: "1"
  id: string
  ...
```

**好处**：数据格式演进时不需要迁移所有旧数据，可以按版本做兼容读取。

### 平台约束显式化

work-context-2 在架构文档里直接写死约束：

> - `Python >= 3.12`
> - 标准库优先
> - 非必要不引入第三方库
> - 如需突破这些约束，必须先显式通知并说明理由

**把约束写成文档，让 agent 也知道边界在哪里。**

## 反模式警示

### ❌ Projection 变为 Source of Truth

派生内容不应反向成为主数据。Projection 始终可重建。

### ❌ 跳过审核管道直接发布

抽取出的经验不应直接视为完成。draft → review → approve → publish 的管道保证了质量。

### ❌ 状态机逻辑散落在多处

状态迁移规则应该集中在一个地方，所有入口调用同一个方法。

## 来源

- work-context-2 文档体系（`docs/03-domain-model/`、`docs/02-architecture/`）
- 2026-06-07 agent 阅读后提炼
