# Kaubo 展品 — Roadmap

## 总览

| Phase | 产出 | 可验证 |
|-------|------|--------|
| 1 | 项目骨架 + Lexer | 浏览器里看到 token 表 |
| 2 | Parser + AST 可视化 | 浏览器里看到 AST 树 |
| 3 | CodeGen + 字节码可视化 | 浏览器里看到反汇编 |
| 4 | VM + 结果输出 | 流水线跑通，hello world 出结果 |
| 5 | 收尾 | 预加载示例、阶段控制、样式 |

每个 Phase 结束时都有一条可演示的东西。Phase 1 用 `<pre>` 硬输出 JSON 也行——**先看到东西，再变好看**。

---

## Phase 1：项目骨架 + Lexer

**目标**：Rust 编译到 WASM，TS 加载 WASM，浏览器里输入 kaubo 代码 → 看到 token 列表。

### 1a. 初始化 `apps/kaubo/`

```
apps/kaubo/
├── package.json          ←   vite, @solidjs/router, vite-plugin-wasm
├── tsconfig.json
├── vite.config.ts
├── index.html
└── src/
    ├── entry.tsx
    └── App.tsx           ←   textarea + <pre> 输出
```

`pnpm dev` 能跑，空白页面可见。

### 1b. 初始化 `kaubo-engine/`

```
apps/kaubo/kaubo-engine/
├── Cargo.toml            ←   wasm-bindgen, serde, serde_json
└── src/
    ├── lib.rs            ←   #[wasm_bindgen] pub fn lex()
    ├── lexer.rs          ←   scan()
    └── types.rs          ←   Token, TokenKind
```

`wasm-pack build --target web` 成功，生成 `.wasm` + JS glue。

### 1c. 实现 Lexer

Token 全集：关键字（11 个）、字面量（3 种）、标识符、运算符/标点（13 个）、EOF。

- `scan()` —— single-pass，char-by-char
- `scan_number()` —— 整数 + 浮点
- `scan_string()` —— 双引号字符串，转义
- `scan_ident_or_keyword()` —— 标识符 vs 关键字查表
- `skip_line_comment()` —— `//` 到行尾
- 错误 token 标记 `ERROR`，不中断扫描

### 1d. WASM 导出 + TS 适配

```rust
#[wasm_bindgen]
pub fn lex(source: &str) -> String {
    let tokens = lexer::scan(source);
    serde_json::to_string(&tokens).unwrap()
}
```

TS 侧 `wasm/adapter.ts`：加载 WASM → `runLex()` → `JSON.parse()` → `Token[]`

### 1e. 最小 UI

```
┌────────────────────────────────────────────────────┐
│  Kaubo                                            │
├────────────────────┬───────────────────────────────┤
│  Code              │  Tokens                      │
│  ┌──────────────┐  │  ┌─────────────────────────┐ │
│  │var x = 42;   │  │  │[1:1] VAR    "var"       │ │
│  │print(x);     │  │  │[1:5] ID     "x"         │ │
│  │              │  │  │[1:7] OP     "="         │ │
│  │              │  │  │[1:9] INT    "42"        │ │
│  │              │  │  │...                      │ │
│  └──────────────┘  │  └─────────────────────────┘ │
└────────────────────┴───────────────────────────────┘
```

左边 `<textarea>`，右边 `<pre>` 显示 `JSON.stringify(tokens, null, 2)`。够用就行，后面再美化。

**验收**：在 textarea 里输入 `var x = 42;`，右边出现 token JSON 数组。

---

## Phase 2：Parser + AST 可视化

**目标**：Token 流 → AST 树 → 递归组件渲染。

### 2a. 实现 Parser

递归下降。按优先级分层：
```
parse_module → parse_statement → parse_expression
                                    → parse_assignment
                                    → parse_lambda
                                    → parse_or / parse_and
                                    → parse_equality / parse_comparison
                                    → parse_term / parse_factor
                                    → parse_unary / parse_call / parse_primary
```

支持完整的 Stmt 和 Expr 集合（见架构文档 §2）。

每个 `parse_*` 返回 `Result<T, ParseError>`。

### 2b. WASM 导出

```rust
#[wasm_bindgen]
pub fn parse(tokens_json: &str) -> String {
    let tokens: Vec<Token> = serde_json::from_str(tokens_json).unwrap();
    let module = parser::parse_module(&tokens);
    serde_json::to_string(&module).unwrap()
}
```

### 2c. AST 树形渲染

递归 `TreeNode` 组件：
```tsx
function ASTNode(props: { node: ASTNode; depth: number }) {
  return (
    <div style={{ "padding-left": `${props.depth * 1.25}rem` }}>
      <span>{props.node.type}</span>
      <For each={props.node.children}>
        {child => <ASTNode node={child} depth={props.depth + 1} />}
      </For>
    </div>
  );
}
```

**验收**：输入 `var x = 1 + 2 * 3;`，AST 树显示 `BinaryOp(*)` 是 `BinaryOp(+)` 的右子树（运算符优先级正确）。

---

## Phase 3：CodeGen + 字节码可视化

**目标**：AST → Chunk → 反汇编视图。

### 3a. 实现 CodeGen

实现指令集（见架构文档 §3）。递归 walk AST，emit 指令 + 常量池。

关键路径：
- 表达式 → LOAD_CONST / 运算指令
- 变量声明 → STORE_NAME
- 变量引用 → LOAD_NAME
- if/while → JUMP / JUMP_IF_FALSE + 回填
- lambda → MAKE_FUNCTION（编译函数体到独立 Chunk）
- call → CALL

### 3b. WASM 导出

```rust
#[wasm_bindgen]
pub fn codegen(ast_json: &str) -> String {
    let module: AST = serde_json::from_str(ast_json).unwrap();
    let chunk = codegen::compile(&module);
    serde_json::to_string(&chunk).unwrap()
}
```

### 3c. 字节码反汇编视图

TS Visualizer 把 `Chunk` JSON 转成人类可读的指令列表：

```
0000  LOAD_CONST    0    ; 42
0002  STORE_NAME    0    ; "x"
0004  LOAD_NAME     0    ; "x"
0006  RETURN_VALUE
```

**验收**：输入 `var x = 42;`，右边反汇编面板显示正确的指令序列。

---

## Phase 4：VM + 全流水线

**目标**：字节码 → 执行 → 输出。四阶段全部跑通。

### 4a. 实现 VM

栈机（见架构文档 §4）。指令分发循环。内置函数（print、len、type）。

关键路径：
- LOAD_CONST / STORE_NAME / LOAD_NAME
- 运算 + 比较指令
- JUMP / JUMP_IF_FALSE
- MAKE_FUNCTION / CALL / RETURN

### 4b. WASM 导出

```rust
#[wasm_bindgen]
pub fn execute(chunk_json: &str) -> String {
    let chunk: Chunk = serde_json::from_str(chunk_json).unwrap();
    let result = vm::execute(&chunk);
    serde_json::to_string(&result).unwrap()
}
```

### 4c. TS Orchestrator 串联全流水线

```typescript
// orchestrator/pipeline.ts
export function runPipeline(source: string): PipelineResult {
  const tokens = runLex(source);
  const ast = runParse(tokens);
  const chunk = runCodegen(ast);
  const result = runExecute(chunk);
  return { stages: [tokens, ast, chunk, result] };
}
```

### 4d. 四面板 UI

```
┌──────────────────────────────────────────────────────────────┐
│  Code                                                        │
│  ┌────────────────────────────────────────────────────────┐  │
│  │ var add = |a, b| { return a + b; };                    │  │
│  │ print(add(2, 3));                                      │  │
│  └────────────────────────────────────────────────────────┘  │
├──────────────────────────────────────────────────────────────┤
│  Tokens │ AST │ Bytecode │ Result                            │
│  ┌──────┐┌─────┐┌────────┐┌────────┐                        │
│  │12 tk ││树形 ││反汇编  ││> 5     │                        │
│  └──────┘└─────┘└────────┘└────────┘                        │
└──────────────────────────────────────────────────────────────┘
```

**验收**：输入 `print(1 + 2 * 3);`，Result 面板显示 `7`。

---

## Phase 5：收尾

**目标**：从一个能跑的东西变成一个能展示的东西。

### 5a. 预加载示例

打开页面时默认加载 `hello.kaubo` 示例，流水线已经跑完。来访者第一眼就看到东西。示例列表：hello、fib、closure、control-flow。

### 5b. 阶段开关

checkbox 控制每个阶段是否运行/显示。关了 Lexer → Token 面板消失，后续阶段也不跑。

### 5c. 错误处理

Lexer 错误、Parser 错误、CodeGen 错误、VM 错误——每种错误在对应的面板里渲染，不崩全页。

### 5d. 样式

从 Arbor container 的 `tokens.css` 引入 design tokens。深色主题。响应式。

### 5e. 底部信息栏

"486 tests passing · kaubo-engine v0.1.0 · View source"

---

## 不在此 roadmap 中的

- TypeChecker / 类型推导
- 多文件编译 / import
- 协程 / generator
- 闭包捕获（先做简单参数传递）
- CLI
- 移动端适配
- SEO / meta tags
- 任何后端

---

## 里程碑总览

```
Phase 1 ████░░░░░░░░░░░░░░░░  项目骨架 + Lexer
Phase 2 ████████░░░░░░░░░░░░  Parser + AST 可视化
Phase 3 ████████████░░░░░░░░  CodeGen + 字节码
Phase 4 ████████████████░░░░  VM + 全流水线
Phase 5 ████████████████████  收尾
```
