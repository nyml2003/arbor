/**
 * Kaubo WASM Boundary Types
 * ==========================
 *
 * Rust (kaubo-engine) 和 TypeScript (前端) 之间的唯一合同。
 * 四个 stage 的请求和响应格式全部在此定义。
 * 两边对着这份文件写代码，字段名/类型/必选可选全部写死。
 *
 * Generated from: kaubo-orchestrator Rust source
 */

// ============================================================================
// Common Types
// ============================================================================

export interface SourcePosition {
  line: number;         // 1-based
  column: number;       // 1-based
}

export interface SourceSpan {
  start: SourcePosition;
  end: SourcePosition;
}

// ============================================================================
// Stage 1: Lex (Source → Tokens)
// ============================================================================

export enum TokenKind {
  // Keywords
  Var = 11, If = 12, Else = 13, Elif = 14, While = 15, For = 16,
  Return = 17, In = 18, True = 20, False = 21, Null = 22,
  Struct = 25, Impl = 27, Import = 28, As = 29, From = 30,
  And = 32, Or = 33, Not = 34, Pub = 39, Print = 40, Json = 41,
  Operator = 38,
  // Literals
  LiteralInteger = 100, LiteralString = 101, LiteralFloat = 102,
  // Identifier
  Identifier = 120,
  // Multi-char symbols
  DoubleEqual = 130, ExclamationEqual = 131, GreaterThanEqual = 132,
  LessThanEqual = 133, FatArrow = 134,
  // Single-char symbols
  GreaterThan = 150, LessThan = 151, Plus = 152, Minus = 153,
  Asterisk = 154, Slash = 155, Percent = 156, Colon = 157, Equal = 158,
  Comma = 159, Semicolon = 160, LeftParenthesis = 161, RightParenthesis = 162,
  LeftCurlyBrace = 163, RightCurlyBrace = 164, LeftSquareBracket = 165,
  RightSquareBracket = 166, Dot = 167, Pipe = 168,
  // Special
  InvalidToken = 255,
}

export interface Token {
  kind: TokenKind;
  span: SourceSpan;
  text?: string;          // 字面量和标识符才有
}

export type LexErrorKind =
  | { type: "InvalidChar"; char: string }
  | { type: "UnterminatedString" }
  | { type: "InvalidEscape"; sequence: string }
  | { type: "InvalidNumber"; number: string };

export interface LexError {
  kind: LexErrorKind;
  position: SourcePosition;
  message: string;
}

export type LexOutput =
  | { success: true; tokens: Token[] }
  | { success: false; errors: LexError[]; partial_tokens: Token[] };

// ============================================================================
// Stage 2: Parse (Tokens → AST)
// ============================================================================

// --- Type Expressions ---

export type TypeExpr =
  | { type: "Named"; name: string }
  | { type: "List"; element: TypeExpr }
  | { type: "Tuple"; elements: TypeExpr[] }
  | { type: "Function"; params: TypeExpr[]; return_type?: TypeExpr };

// --- Expressions ---

export type LambdaParam = [string, TypeExpr | undefined];

export type Expr =
  | { type: "LiteralInt"; value: number }
  | { type: "LiteralFloat"; value: number }
  | { type: "LiteralString"; value: string }
  | { type: "LiteralTrue" } | { type: "LiteralFalse" } | { type: "LiteralNull" }
  | { type: "LiteralList"; elements: Expr[] }
  | { type: "Binary"; left: Expr; op: TokenKind; right: Expr }
  | { type: "Unary"; op: TokenKind; operand: Expr }
  | { type: "Grouping"; expression: Expr }
  | { type: "VarRef"; name: string }
  | { type: "FunctionCall"; function_expr: Expr; arguments: Expr[] }
  | { type: "Lambda"; params: LambdaParam[]; return_type?: TypeExpr; body: Stmt }
  | { type: "MemberAccess"; object: Expr; member: string }
  | { type: "IndexAccess"; object: Expr; index: Expr }
  | { type: "JsonLiteral"; entries: [string, Expr][] }
  | { type: "StructLiteral"; name: string; fields: [string, Expr][] }
  | { type: "As"; expr: Expr; target_type: TypeExpr };

// --- Statements ---

export interface FieldDef { name: string; type_annotation: TypeExpr; }

export interface MethodDef { name: string; lambda: Expr; }

export type Stmt =
  | { type: "Expr"; expression: Expr }
  | { type: "Empty" }
  | { type: "Block"; statements: Stmt[] }
  | { type: "VarDecl"; name: string; type_annotation?: TypeExpr; initializer: Expr; is_public: boolean }
  | { type: "If"; if_condition: Expr; then_body: Stmt; elif_conditions: Expr[]; elif_bodies: Stmt[]; else_body?: Stmt }
  | { type: "While"; condition: Expr; body: Stmt }
  | { type: "For"; iterator: Expr; iterable: Expr; body: Stmt }
  | { type: "Return"; value?: Expr }
  | { type: "Print"; expression: Expr }
  | { type: "Import"; module_path: string; items: string[]; alias?: string }
  | { type: "Struct"; name: string; fields: FieldDef[] }
  | { type: "Impl"; struct_name: string; methods: MethodDef[] };

export interface Module { statements: Stmt[]; }

// --- Parse Errors ---

export type ParseErrorKind =
  | { type: "UnexpectedToken"; found: string; expected: string[] }
  | { type: "InvalidNumberFormat"; message: string }
  | { type: "MissingRightParen" } | { type: "MissingRightBracket" }
  | { type: "MissingRightCurly" } | { type: "UnexpectedEndOfInput" }
  | { type: "ExpectedIdentifier"; found: string }
  | { type: "Custom"; message: string }
  | { type: "ModuleKeywordDeprecated" };

export interface ParseError {
  kind: ParseErrorKind;
  line: number;
  column: number;
}

export type ParseOutput =
  | { success: true; module: Module }
  | { success: false; errors: ParseError[] };

// ============================================================================
// Stage 3: CodeGen (AST → Bytecode)
// ============================================================================

export type Value = number;  // u64 NaN-boxed

export interface MethodTableEntry {
  shape_id: number;         // u16
  method_idx: number;       // u8
  const_idx: number;        // u8 — index into Chunk.constants
}

export interface OperatorTableEntry {
  shape_id: number;         // u16
  operator_name: string;    // "add", "sub", "mul", "eq", ...
  const_idx: number;        // u8
}

export interface InlineCacheEntry {
  left_shape: number;       // u16; u16::MAX = empty
  right_shape: number;      // u16
  closure: number;          // opaque pointer
  hit_count: number;        // u64
  miss_count: number;       // u64
}

export interface Chunk {
  /** 字节码指令序列。每条指令 1 字节 opcode + 0-3 字节操作数 (小端序) */
  code: number[];           // 序列化为 JSON 数组；WASM 内部用 Uint8Array
  /** 常量池 (NaN-boxed Values) */
  constants: Value[];
  /** code[i] 对应的源码行号 (1-based) */
  lines: number[];
  /** MethodTable */
  method_table: MethodTableEntry[];
  /** Operator overload table */
  operator_table: OperatorTableEntry[];
  /** Inline cache slots (pc -> cache_idx 映射) */
  inline_cache_slots: { pc: number; cache_idx: number }[];
  /** Inline cache entries (运行时可变) */
  inline_caches: InlineCacheEntry[];
}

export type CodeGenError =
  | { type: "InvalidOperator" }
  | { type: "TooManyConstants" }
  | { type: "TooManyLocals" }
  | { type: "VariableAlreadyExists"; name: string }
  | { type: "UninitializedVariable"; name: string }
  | { type: "Unimplemented"; message: string };

export type CodeGenOutput =
  | { success: true; chunk: Chunk; max_locals: number }
  | { success: false; error: CodeGenError };

// ============================================================================
// Stage 4: Execute (Bytecode → Result)
// ============================================================================

export type ExecuteOutput =
  | { success: true; exit_code: 0; stdout: string }
  | { success: false; exit_code: number; stdout: string; stderr: string };

// ============================================================================
// WASM FFI Signatures
// ============================================================================

/**
 * kaubo-engine WASM 模块导出的四个函数。
 * 每个函数的入参和返回值都是 JSON string。
 */
export interface KauboWasmExports {
  /** Stage 1: Source → JSON(LexOutput) */
  lex(source: string): string;

  /** Stage 2: JSON(Token[]) → JSON(ParseOutput) */
  parse(tokens_json: string): string;

  /** Stage 3: JSON(Module) → JSON(CodeGenOutput) */
  codegen(ast_json: string): string;

  /** Stage 4: JSON(Chunk) → JSON(ExecuteOutput) */
  execute(chunk_json: string): string;
}
