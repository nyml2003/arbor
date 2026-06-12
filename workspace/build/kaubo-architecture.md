# Kaubo — WASM Rust + TS 接口层 + SolidJS UI

## 命名

- Rust 侧叫 `kaubo-engine`。一个 crate，做一件事。
- TS 侧叫 `apps/kaubo/`（Vite + SolidJS 项目）。
- 不再引用 kaubo-features/next_kaubo 的旧 crate 命名。

---

## 总览

```
┌──────────────────────────────────────────────────────────────┐
│                    Browser (JS Runtime)                       │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              SolidJS UI                                 │  │
│  │  CodeEditor  │  StagePanels  │  PipelineControl        │  │
│  └────────────────────────┬───────────────────────────────┘  │
│                           │  runPipeline(source, stages)      │
│                           ▼                                   │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              TS Interface Layer                         │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │  │
│  │  │ Orchestrator │  │  Visualizer  │  │WASM Adapter  │  │  │
│  │  └──────────────┘  └──────────────┘  └──────┬───────┘  │  │
│  └─────────────────────────────────────────────┴──────────┘  │
│                                                  │            │
├──────────────────────────────────────────────────┼────────────┤
│                  WASM boundary (JSON strings)    │            │
├──────────────────────────────────────────────────┼────────────┤
│                                                  ▼            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              kaubo-engine (Rust, wasm32)               │  │
│  │  Lexer │ Parser │ CodeGen │ VM                         │  │
│  │  Source → Tokens → AST → Chunk → Result                │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│   apps/kaubo/kaubo-engine/                                   │
└──────────────────────────────────────────────────────────────┘
```

**两个运行时，一个协议**：JSON string 进出 WASM boundary。Rust 不知道浏览器。TS 不知道编译器内部。

---

## 目录结构

```
apps/kaubo/
├── package.json
├── tsconfig.json
├── vite.config.ts              ← vite-plugin-wasm
├── index.html
│
├── kaubo-engine/               ← Rust crate
│   ├── Cargo.toml              ←   wasm-bindgen, serde
│   └── src/
│       ├── lib.rs              ←   #[wasm_bindgen] lex, parse, codegen, execute
│       ├── lexer.rs
│       ├── parser.rs
│       ├── codegen.rs
│       ├── vm.rs
│       └── types.rs            ←   Token, AST, Chunk, Result<T,E>
│
└── src/                        ← TypeScript
    ├── entry.tsx               ←   SolidJS 挂载点
    ├── App.tsx
    │
    ├── wasm/
    │   └── adapter.ts          ←   initWasm(), runLex(), runParse(), ...
    │
    ├── orchestrator/
    │   ├── pipeline.ts         ←   runPipeline(source, stages)
    │   └── types.ts
    │
    ├── visualizer/
    │   ├── index.ts
    │   ├── tokens.ts
    │   ├── ast.ts
    │   ├── bytecode.ts
    │   └── result.ts
    │
    └── ui/
        ├── CodeEditor.tsx
        ├── PipelineControl.tsx
        ├── StagePanels.tsx
        └── panels/
            ├── TokenPanel.tsx
            ├── ASTPanel.tsx
            ├── BytecodePanel.tsx
            └── ResultPanel.tsx
```

一个 `apps/kaubo/` 目录，里面 Rust crate 和 TS 源码共存。Vite 的 `vite-plugin-wasm` 处理 WASM 打包。

---

## Rust 侧：kaubo-engine

### 约束

- 一个 crate，不拆 workspace
- 依赖：`wasm-bindgen`、`serde`、`serde_json`——尽量少
- 不做 VFS、Log、Config
- 不做 TypeChecker
- 纯逻辑。零 IO。零 DOM。

### 四个 WASM 导出函数

```rust
#[wasm_bindgen] pub fn lex(source: &str) -> String;
#[wasm_bindgen] pub fn parse(tokens_json: &str) -> String;
#[wasm_bindgen] pub fn codegen(ast_json: &str) -> String;
#[wasm_bindgen] pub fn execute(chunk_json: &str) -> String;
```

String 进，String 出。编排逻辑在 TS Orchestrator。

---

### 1. Lexer

```
Source: "var add = |a, b| { return a + b; };"
        │
        ▼  single-pass, position pointer, char-by-char
Tokens: [VAR, ID("add"), OP("="), PIPE, ID("a"), COMMA, ID("b"),
         PIPE, LBRACE, RETURN, ID("a"), OP("+"), ID("b"), SEMI, RBRACE, SEMI, EOF]
```

**Token 类型**（参考旧版缩减）：

| 类别 | Token |
|------|-------|
| 关键字 | `VAR`, `IF`, `ELSE`, `WHILE`, `FOR`, `RETURN`, `TRUE`, `FALSE`, `NULL`, `IMPORT`, `PUB` |
| 字面量 | `INT`, `FLOAT`, `STRING` |
| 标识符 | `ID` |
| 运算符/标点 | `OP`, `PIPE`, `LBRACE`, `RBRACE`, `LPAREN`, `RPAREN`, `LBRACKET`, `RBRACKET`, `COMMA`, `SEMI`, `DOT`, `COLON`, `ARROW` |
| 特殊 | `EOF` |

**扫描算法**（不建状态机框架，`match char` 直派）：

```rust
fn scan(source: &str) -> Vec<Token> {
    let mut pos = 0;
    let mut tokens = vec![];
    while pos < source.len() {
        let c = source[pos..].chars().next().unwrap();
        match c {
            '/' if peek('/') => skip_line_comment(&mut pos),
            '"'              => tokens.push(scan_string(&mut pos)),
            '0'..='9'        => tokens.push(scan_number(&mut pos)),
            'a'..='z' | '_'  => tokens.push(scan_ident_or_keyword(&mut pos)),
            '|'              => tokens.push(Token::new(PIPE, "|", line, col)),
            '{'              => tokens.push(Token::new(LBRACE, "{", line, col)),
            // ...
            _ if c.is_whitespace() => pos += c.len_utf8(),
            _ => tokens.push(error_token(&mut pos)), // 不中断，收集错误继续扫
        }
    }
    tokens.push(Token::new(EOF, "", line, col));
    tokens
}
```

- 不依赖 `split()`，不依赖正则
- 关键字在 `scan_ident_or_keyword` 里查 HashMap 区分
- 错误 token 标记 `kind: "ERROR"`，不中断扫描

---

### 2. Parser

```
Tokens → AST

AST =
  Module { statements: Stmt[] }

  Stmt = VarDecl { name, type_annotation?, value: Expr }
       | ExprStmt { expr: Expr }
       | ReturnStmt { value: Expr? }
       | IfStmt { condition, then_branch, else_branch? }
       | WhileStmt { condition, body }
       | ForStmt { init?, condition?, update?, body }
       | Block { statements: Stmt[] }

  Expr  = IntegerLiteral { value: i64 }
       | FloatLiteral { value: f64 }
       | StringLiteral { value: String }
       | BoolLiteral { value: bool }
       | NullLiteral
       | Identifier { name: String }
       | BinaryOp { left: Expr, op: BinOp, right: Expr }
       | UnaryOp { op: UnaryOp, expr: Expr }
       | Call { callee: Expr, args: Expr[] }
       | Lambda { params: Param[], body: Stmt[] }
       | Index { target: Expr, index: Expr }
       | Member { target: Expr, member: String }
       | JsonLiteral { fields: (String, Expr)[] }
       | ListLiteral { elements: Expr[] }
```

**递归下降结构**：

```rust
fn parse_module(tokens: &[Token], pos: &mut usize) -> Result<Module, ParseError>;

// 按优先级分层
fn parse_expression(tokens, pos)  → parse_assignment → parse_lambda → parse_or → parse_and → parse_equality → parse_comparison → parse_term → parse_factor → parse_unary → parse_call → parse_primary;

fn parse_statement(tokens, pos)   → match current token { VAR => parse_var_decl, IF => parse_if, RETURN => parse_return, ... };
```

**Pratt parser 还是优先级爬升？** 不用 Pratt——kaubo 的表达式语法简单（没有三元运算符、没有复杂前缀/后缀混合），直接用优先级分层 + 递归下降。每层一个函数，左结合用循环，右结合用递归。

**错误处理**：每个 `parse_*` 返回 `Result<T, ParseError>`。ParseError 携带 `message + line + column`。遇到错误不 panic，返回 Err，上层决定是否继续。

---

### 3. CodeGen

```
AST → Chunk

Chunk {
    code: Vec<u8>,           // 字节码序列
    constants: Vec<Value>,   // 常量池
    lines: Vec<u32>,         // code[ip] → 源码行号
}
```

**指令集**（精简自旧版，保留 MVP 必需的）：

| 类别 | 指令 | 操作数 | 栈效果 |
|------|------|--------|--------|
| 常量 | `LOAD_CONST(idx)` | u16 | → value |
| 变量 | `STORE_NAME(idx)` | u16 | value → |
| | `LOAD_NAME(idx)` | u16 | → value |
| 栈操作 | `POP` | — | value → |
| 二元运算 | `ADD / SUB / MUL / DIV / MOD` | — | a,b → a+b |
| 比较 | `EQ / NEQ / LT / GT / LTE / GTE` | — | a,b → bool |
| 一元 | `NEG / NOT` | — | a → -a |
| 跳转 | `JUMP(offset)` | i16 | — |
| | `JUMP_IF_FALSE(offset)` | i16 | bool → |
| 函数 | `MAKE_FUNCTION(idx)` | u16 | → fn |
| | `CALL(argc)` | u8 | fn,args... → result |
| | `RETURN` | — | value → (to caller) |
| 属性 | `LOAD_ATTR(idx)` | u16 | obj → obj.attr |
| 下标 | `LOAD_INDEX` | — | obj,idx → obj[idx] |
| 构造 | `BUILD_LIST(count)` | u8 | items... → list |
| | `BUILD_JSON(count)` | u8 | k,v... → json |

**AST → ByteCode 遍历**（递归 walk，emit 指令到 Vec）：

```rust
fn compile_expr(expr: &Expr, chunk: &mut Chunk) -> Result<(), CodeGenError> {
    match expr {
        Expr::IntegerLiteral { value } => {
            let idx = chunk.add_constant(Value::Int(*value));
            chunk.emit(LOAD_CONST, idx);
        }
        Expr::BinaryOp { left, op, right } => {
            compile_expr(left, chunk)?;
            compile_expr(right, chunk)?;
            chunk.emit(match op { Add => ADD, Sub => SUB, ... }, 0);
        }
        Expr::Lambda { params, body } => {
            // 编译函数体到独立 Chunk，存入常量池
            let fn_chunk = compile_function(params, body)?;
            let idx = chunk.add_constant(Value::Function(fn_chunk));
            chunk.emit(MAKE_FUNCTION, idx);
        }
        // ...
    }
}
```

**不做**：inline cache、shape table、多模块链接、二进制序列化。

---

### 4. VM

```
Chunk → ExecutionResult

ExecutionResult {
    prints: Vec<String>,
    return_value: Option<Value>,
    steps: u64,
}
```

**栈机模型**：

```
┌─────────────────────────────────┐
│            VM                    │
│  ip: usize          ← 指令指针  │
│  stack: Vec<Value>  ← 操作数栈  │
│  globals: HashMap   ← 全局变量  │
│  chunk: &Chunk      ← 当前代码  │
│  output: Vec<String>← print 捕获│
└─────────────────────────────────┘
```

**指令分发循环**：

```rust
fn execute(chunk: &Chunk) -> ExecutionResult {
    let mut ip = 0;
    let mut stack: Vec<Value> = vec![];
    let mut globals: HashMap<String, Value> = HashMap::new();
    let mut output: Vec<String> = vec![];
    let mut steps = 0;

    loop {
        let opcode = chunk.code[ip];
        ip += 1;
        steps += 1;

        match opcode {
            LOAD_CONST => {
                let idx = read_u16(&chunk.code, &mut ip);
                stack.push(chunk.constants[idx].clone());
            }
            ADD => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(add(&a, &b));
            }
            CALL => {
                let argc = chunk.code[ip]; ip += 1;
                // 从栈顶取 args，取 callee
                // callee 是 Function => 创建新栈帧，切换 ip
                // callee 是 Native => 直接调用（print 等）
            }
            RETURN => {
                // 弹出返回值，恢复到调用者的 ip 和栈
            }
            // ...
        }

        if ip >= chunk.code.len() { break; }
    }

    ExecutionResult { prints: output, return_value: stack.pop(), steps }
}
```

**Value 类型**（运行时所有值的枚举）：

```rust
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Json(HashMap<String, Value>),
    Function(FunctionObject),
    NativeFunction(fn(&[Value]) -> Value),
}
```

**不做**：多栈帧（MVP 只有全局作用域 + 一层函数调用）、协程、闭包捕获（先做简单的参数传递）、GC。

**内置函数**（硬编码在 VM 里）：

| 函数 | 行为 |
|------|------|
| `print(args...)` | 每个 arg 转字符串，push 到 output |
| `len(x)` | List/String 长度 |
| `type(x)` | 返回类型名字符串 |

---

### 5. 四个阶段的 JSON schema

Rust 和 TS 两边保持同步：

```
lex:      string → {"tokens":[{"kind":"VAR","value":"var","line":1,"col":1},...]}
parse:    string → {"module":{"statements":[...]}}
codegen:  string → {"code":[0,1,42,...],"constants":[...],"lines":[1,1,2,...]}
execute:  string → {"prints":["hello"],"return_value":null,"steps":42}
```

TS 侧 `orchestrator/types.ts` 定义同名 interface。

---

## TS 侧：三层

### 1. WASM Adapter（`wasm/adapter.ts`）

```typescript
import init, { lex, parse, codegen, execute } from 'kaubo-engine';

export async function initWasm(): Promise<void> { ... }

export function runLex(source: string): Token[] { ... }
export function runParse(tokens: Token[]): AST { ... }
export function runCodegen(ast: AST): Chunk { ... }
export function runExecute(chunk: Chunk): ExecResult { ... }
```

- 每个函数：序列化输入 → 调 WASM → 反序列化输出
- 失败返回 `{ ok: false, error }` —— 不抛异常
- 这是唯一 import WASM 的模块

### 2. Orchestrator（`orchestrator/pipeline.ts`）

```typescript
export function runPipeline(
  source: string,
  stages: Set<'lexer' | 'parser' | 'codegen' | 'vm'>,
): PipelineResult
```

- 按依赖顺序调 adapter 函数
- 跳过未选中的阶段
- 收集每个阶段的产物 + 耗时
- 纯逻辑，零 DOM 依赖

### 3. Visualizer（`visualizer/`）

- 纯函数：IR 数据 + 模式 → 字符串
- Text 模式：给人看（CLI 将来用）
- JSON 模式：给 UI 渲染（`For`/`Show` 直接消费）
- 不 import DOM、WASM、SolidJS

### 4. UI（`ui/`）

- `createResource(() => runPipeline(sourceCode(), stages()))` —— 声明式，key 变自动重跑
- `Show when={}` —— 未选中的阶段不在 DOM 中
- `For` —— 渲染 token 行、指令行
- 递归 `TreeNode` —— 渲染 AST 树

---

## 不做

- TypeChecker / TypedAst
- 多文件 / 模块系统
- 二进制格式
- VFS / Log / Config 基础设施
- CLI
- Electron / Tauri
- 新 repo（都在 `apps/kaubo/` 下）
