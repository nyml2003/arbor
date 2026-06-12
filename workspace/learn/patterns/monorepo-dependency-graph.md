# 模式：Monorepo 依赖图与能力校验（Ethereal）

## 一句话

用 WorkspaceGraph（forward + reverse 邻接表）做 monorepo 包依赖管理，用 import 扫描 enforce 架构约束（如"core 不能 import Node.js"）。

## 核心架构

```
NodeConfig (workspace 配置)
  │  from_node_configs()
  ▼
WorkspaceGraph
  ├── nodes: HashMap<NodeId, GraphNode>     ← 包元数据
  ├── forward: HashMap<NodeId, Vec<NodeId>> ← 正向依赖（A → B）
  ├── reverse: HashMap<NodeId, Vec<NodeId>> ← 反向依赖（B ← A）
  └── artifact_nodes: Vec<NodeId>           ← 需要构建产出的包

Scan (SWC)
  │  扫描每个包的 import/global 使用
  ▼
Validate
  │  enforce 架构约束
  ▼
ExecutionPlan
  │  topological_order()
  ▼
Build
```

## 关键设计

### 1. 依赖图的三层结构

```rust
pub struct WorkspaceGraph {
    pub nodes: HashMap<NodeId, GraphNode>,       // 包 → 元数据
    pub forward: HashMap<NodeId, Vec<NodeId>>,    // A → [B, C]  (A 依赖 B, C)
    pub reverse: HashMap<NodeId, Vec<NodeId>>,    // B → [A]     (A 依赖 B)
    pub artifact_nodes: Vec<NodeId>,              // 有产出的包
}
```

**为什么需要 forward + reverse**：
- forward：构建 A 前先构建 B, C（依赖方向）
- reverse：B 改了，找出所有依赖 B 的包（affected detection）

**和 pnpm/pnpm-workspace.yaml 的关系**：pnpm 已经解析了依赖。Ethereal 做的是在此基础上加**架构校验**和**构建编排**。

### 2. 拓扑排序 —— 构建顺序

```rust
pub fn topological_order(&self) -> Vec<NodeId> {
    // Kahn's algorithm
    // 1. 计算每个节点的入度（被多少节点依赖）
    // 2. 入度 = 0 的节点进入队列
    // 3. 出队 → 减少被依赖节点的入度 → 入度 = 0 进队
    // 4. 循环到队列为空
}
```

**循环依赖检测**：如果 `ordered.len() != self.nodes.len()`，存在循环 → 报出不在排序结果中的节点。

### 3. 依赖闭包（affected detection）

```rust
pub fn dependency_closure(&self, start: &NodeId) -> Vec<NodeId> {
    // 从 start 出发，DFS 收集所有可达节点
    // 结果 = 改了 start 之后需要重新构建的所有包
}
```
// 改了 package-a → dependency_closure("package-a") → 需要重新构建 package-b + package-c

### 4. Import 扫描 + 架构约束

```rust
pub fn validate_node_capabilities(summaries: &[ScanSummary]) -> Result<(), ValidationError> {
    for summary in summaries {
        // 检查：pure-logic 包有没有 import 平台模块？
        let forbidden_import = summary.imports.iter()
            .find(|item| item.starts_with("node:")
                     || item == &"electron"
                     || item == &"react-native");

        // 检查：pure-logic 包有没有用 window/document/fetch？
        let forbidden_global = summary.globals.iter()
            .find(|item| matches!(item.as_str(),
                "window" | "document" | "fetch" | "process" | "Buffer"));
    }
}
```

**这就是 DECISIONS.md 和 CONVENTIONS.md 的自动化 enforcement**：
- core 包不能 import `electron` → swc 扫描 → 自动检查 → CI 失败
- 不需要人工 code review 去记住约定

### 5. Adapter Trait —— 工具抽象

```rust
pub trait ToolAdapter {
    fn slot(&self) -> ToolSlot;         // 工具在哪个阶段运行
    fn execute(&self, ctx: &AdapterContext) -> AdapterOutcome;
}

pub struct AdapterContext {
    pub plan: ExecutionPlan,            // 当前构建计划
}

pub struct AdapterOutcome {
    pub slot: ToolSlot,
    pub visited_nodes: Vec<NodeId>,     // 这次执行影响了哪些包
}
```

每个构建工具（swc、esbuild、tsc、vitest）实现同一个 trait。构建编排器不需要知道具体工具是什么——只调用 `adapter.execute(ctx)`。

## 反模式警示

### ❌ 手动维护依赖关系

不要写一个手动的 `const buildOrder = ["core", "cli", "container"]`。依赖关系应该从 package.json 和 tsconfig references 自动提取。

### ❌ 架构约束靠 code review

不要在 PR 里评论"core 不能依赖 electron"。写一个 scan + validate 步骤，CI 自动拒绝。

## 来源

- Ethereal 源码（`src/graph/`、`src/validate/`、`src/adapter/`）
- 2026-06-07 agent 阅读后提炼
