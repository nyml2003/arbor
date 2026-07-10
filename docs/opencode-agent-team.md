# OpenCode Agent Team 使用原则

结论：OpenCode 作为交互入口，oh-my-opencode-slim 作为调度层。默认让 Orchestrator 负责拆任务、委派、整合和验证。专家 agent 只处理边界清楚的子任务。

## 总体结构

```text
User
  |
  v
OpenCode session
  |
  v
oh-my-opencode-slim Orchestrator
  |
  +-- Explorer    代码库侦察、文件定位、影响面分析
  +-- Librarian   官方文档、外部资料、API/SDK 查询
  +-- Fixer       小范围修 bug、测试修复、低风险实现
  +-- Designer    UI/UX、前端交互、视觉结构
  +-- Oracle      架构评审、复杂调试、方案把关
  +-- Council     多模型共识，只用于高风险决策
```

## 角色原则

| Agent | 职责 | 写代码权限 | 使用场景 |
| --- | --- | --- | --- |
| Orchestrator | 拆任务、调度、整合结果、最终验证 | 可写，但大任务少直接写 | 默认入口 |
| Explorer | 搜代码、梳理依赖、定位影响面 | 不写 | 开始实现前的代码库侦察 |
| Librarian | 查官方文档、版本变化、外部 API | 不写 | 依赖外部资料或最新文档的任务 |
| Fixer | 修测试、修类型、局部 bug、低风险实现 | 可写 | 范围明确的小改动 |
| Designer | 设计 UI、交互、布局和视觉状态 | 可选 | 前端和界面任务 |
| Oracle | 架构审查、复杂 bug 判断、方案挑战 | 默认不写 | 高风险实现前后 |
| Council | 多模型对比和裁决 | 不写 | 架构分歧、重大取舍 |

## 模型分配

- Orchestrator 使用最强的通用编码模型，推理强度设为中高。
- Oracle 使用最强的高推理模型。
- Council 使用一个强汇总模型，加多个不同提供商或不同能力倾向的模型。
- Fixer 使用稳定、成本可控、擅长代码修改的模型。
- Explorer 使用快、便宜、上下文足够大的模型。
- Librarian 使用适合联网检索和文档阅读的模型。
- Designer 使用前端和 UI 理解较强的模型。

## 权限边界

- Explorer、Librarian、Oracle、Council 默认只读。
- Fixer 可以写代码，但任务必须有明确文件范围或行为目标。
- Orchestrator 可以写代码，但主要职责是调度和整合。
- 多个可写 agent 不要同时改同一批文件。
- 大范围重构优先使用 worktree 隔离。
- 任何 agent 修改 Git 状态、删除文件、迁移目录或引入依赖前，都必须有明确理由和验证路径。

## 工作流

### 小任务

```text
User -> Orchestrator
Orchestrator -> Explorer 查影响面
Orchestrator -> Fixer 改代码
Orchestrator -> 运行测试
Orchestrator -> Oracle 轻量评审
Orchestrator -> 汇报结果
```

适用场景：

- failing test 修复。
- 小范围 bug。
- 类型错误。
- 单模块行为调整。

### 大任务

```text
/deepwork <任务>
  |
  v
Orchestrator 生成阶段计划
  |
  +-- Explorer: 代码地图和风险点
  +-- Librarian: 外部约束和官方文档
  +-- Oracle: 评审方案
  +-- Fixer: 分阶段实现
  +-- Oracle: 最终审查
  +-- Orchestrator: 跑测试、整合、收尾
```

适用场景：

- 多文件重构。
- 新功能跨多个模块。
- 架构边界变化。
- 需要迁移数据、接口或目录结构。

### 高风险并行任务

高风险并行任务使用 worktree：

```text
主仓库
  |
  +-- .slim/worktrees/feature-auth/
  +-- .slim/worktrees/refactor-core/
```

每个 worktree 只交给一个实现 agent。Orchestrator 负责合并判断和最终验证。

## 调用习惯

直接进入 OpenCode 后，先验证 agent 是否可用：

```text
ping all agents
```

常用调用：

```text
@explorer 先梳理这个仓库的认证流程
@librarian 查一下这个库当前版本的官方用法
@oracle 评审这个架构方案的风险
@fixer 修复当前 failing tests
@council 比较这两种重构路线
/deepwork 重构这个模块并补齐测试
```

## 成本控制

- Council 只用于重大取舍，不作为日常自动步骤。
- Oracle 用在方案前、实现后和疑难问题上。
- Explorer 和 Librarian 可以频繁使用，但要给出明确问题。
- Fixer 适合短任务。长任务先拆阶段。
- 默认 preset 使用 balanced。小修使用 cheap。高风险任务使用 deep。

## 完成标准

一次 agent team 工作完成前，Orchestrator 必须确认：

- 修改范围与任务目标一致。
- 没有两个 agent 写同一文件区域造成冲突。
- 相关测试、类型检查或构建已经运行。
- Oracle 或等价评审已经覆盖高风险改动。
- 最终回复包含改动文件、验证结果和剩余风险。

## 反模式

- 让所有 agent 都能写代码。
- 未侦察代码库就直接让 Fixer 大改。
- 小任务调用 Council。
- 同一个模块同时交给多个实现 agent。
- 把 Orchestrator 当普通 coder 长时间直接写代码。
- 不跑测试就宣称完成。
