# 模式：C++ 编译器管线（changfen/SysY）

## 一句话

一个完整的 C++ 编译器：Flex/Bison 前端 → 自定义 IR → 后端多 pass 优化（寄存器分配、活性分析、死代码消除、CFA），最终生成目标代码。

## 核心架构

```
input.c
  │
  ▼
Frontend（frontend/）
  ├── scanner.l (Flex)      ← 词法分析
  ├── parser.y (Bison)      ← 语法分析 → AST
  ├── symbolTable.cpp       ← 符号表
  └── irgen.cpp             ← IR 生成
  │
  ▼
IR（ir/）
  ├── instruction.cpp       ← IR 指令定义
  ├── codegen.cpp           ← 目标代码生成
  └── passes/               ← IR 优化 pass
  │
  ▼
Backend（backend/）
  ├── basic_block.cpp       ← 基本块划分
  ├── function.cpp          ← 函数帧布局
  ├── register.cpp          ← 寄存器分配
  ├── passes/
  │   ├── regalloc.cpp      ← 寄存器分配 pass
  │   ├── liveness.cpp      ← 活性分析
  │   ├── dce.cpp           ← 死代码消除
  │   ├── cfa.cpp           ← 控制流分析
  │   ├── phi_elimination   ← φ 节点消除
  │   └── unused_store_elim ← 无用存储消除
  └── builder.cpp           ← 最终代码构造
```

## 关键设计

### 1. 三层 IR：前端 IR → 中端 IR → 后端 IR

```
frontend/irgen.cpp   → ir/ 模块（平台无关）→ backend/ 模块（平台相关）
```

每层有自己的 IR 类型、自己的 pass。前端 IR 接近 AST，中端 IR 做优化，后端 IR 做寄存器分配和指令选择。三层之间通过 Builder 模式转换——不是直接操作对方的内部结构。

### 2. Pass 系统

每个 pass 是一个独立的编译单元：

```
backend/passes/regalloc.cpp        ← 寄存器分配
backend/passes/liveness.cpp        ← 变量活性分析
backend/passes/dce.cpp             ← 死代码消除
backend/passes/cfa.cpp             ← 控制流分析
backend/passes/phi_elimination.cpp ← φ 节点消除
backend/passes/unused_store_elim   ← 无用存储消除
```

Pass 之间通过 IR 传递数据——前一个 pass 的输出是后一个 pass 的输入。新增一个 pass 不需要改其他 pass 的代码。

### 3. Frontend 工具链：Flex + Bison

```
scanner.l  → lex.yy.c    (Flex)
parser.y   → parser.tab.c (Bison)
```

Flex 生成词法分析器，Bison 生成 LALR(1) 语法分析器。`driver.cpp` 是入口——连接 scanner、parser、AST、符号表。

### 4. Docker 构建

```dockerfile
# CMakeLists.txt + Dockerfile + docker-compose.yml
```

编译器本身是 C++ 项目，但在 Docker 里构建和运行——意味着不依赖本地编译器版本。SysY 编译器的执行环境是可复现的。

### 5. 输出管道的渐进选项

```cpp
if (options.token_file) { /* 输出 token 流 */ }
if (options.ast_file)   { /* 输出 AST */ }
if (options.ir_file)    { /* 输出 IR */ }
// 最终输出目标代码
```

和 ofsh 的 lexer → parser → executor 完全同构——每一阶段都可以独立输出检查。调试时不需要看最终汇编，可以停在中间环节。

## 反模式警示

### ❌ 单一 pass 做太多事

regalloc、liveness、DCE 各自一个文件。如果把它们合并成一个 `optimizer.cpp`，单个文件会膨胀到几千行且无法独立测试每个优化。

### ❌ 忽略构建环境可复现性

编译器依赖特定 Flex/Bison 版本、C++ 编译器版本。Docker + CMake 保证不管在哪台机器上，构建结果一致。

## 来源

- changfen/Sed 编译器源码（`src/main.cpp`、`src/backend/`、`src/frontend/`、`src/ir/`、`Dockerfile`）
- 2026-06-07 agent 阅读后提炼
