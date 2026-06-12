# Kaubo CodeGen 规格

AST → 字节码的完整映射。从 kaubo-features/next_kaubo `pipeline/codegen/` 提取。

---

## Compiler 结构

```rust
struct Compiler {
    chunk: Chunk,                              // 正在构建的字节码
    locals: Vec<Local>,                        // 局部变量表
    upvalues: Vec<Upvalue>,                    // upvalue 描述符
    scope_depth: usize,                        // 当前嵌套深度
    max_locals: usize,                         // 任意时刻最大局部变量数
    enclosing: *mut Compiler,                  // 外层编译器（null = 根）
    struct_infos: HashMap<String, StructInfo>, // struct 元数据
    var_types: HashMap<String, VarType>,      // 变量类型推导缓存
    module_aliases: HashMap<String, String>,   // import alias 映射
}
```

### Chunk 写入方法

| 方法 | 写入 |
|------|------|
| `write_op(op, line)` | 1 字节 opcode |
| `write_op_u8(op, u8, line)` | 1 字节 opcode + 1 字节操作数 |
| `write_op_u16_u8(op, u16, u8, line)` | 1 + 2(LE) + 1 字节 |
| `write_jump(op, line) → offset` | 1 字节 opcode + 2 字节占位符 (-1)，返回占位符位置 |
| `patch_jump(offset)` | 覆写占位符为 `(code.len() - offset - 2)` 的 i16 LE |
| `write_loop(loop_start, line)` | JumpBack + `-(code.len() - loop_start + 2)` i16 LE |
| `add_constant(v) → u8` | 追加常量池，返回索引（panic if >255） |
| `allocate_inline_cache() → u8` | 追加空 cache entry + slot，返回索引 |

---

## 变量作用域

### Local 结构

```rust
struct Local {
    name: String,
    depth: usize,          // 声明时的 scope_depth
    is_initialized: bool,  // 初始化表达式编译完毕后置 true
    is_captured: bool,     // 被内层闭包捕获
}
```

### 作用域操作

- `begin_scope()` → `scope_depth += 1`
- `end_scope()` → `scope_depth -= 1`，弹出所有 `depth > scope_depth` 的 local
- `add_local(name) → u8` → 追加 Local，返回索引。同一 scope 不允许重名
- `mark_initialized()` → `locals.last().is_initialized = true`
- `resolve_local(name) → Option<u8>` → 反向搜索 locals，返回首个已初始化的匹配
- `mark_captured(idx)` → `locals[idx].is_captured = true`
- `resolve_upvalue(name) → Option<u8>` → 递归查 enclosing 链
- `resolve_variable(name) → Option<Local(u8) | Upvalue(u8)>` → 先 local，后 upvalue

### 优化 Load/Store 发射

- 索引 0-7：用 `LoadLocal0`..`LoadLocal7` / `StoreLocal0`..`StoreLocal7`
- 索引 8-255：用 `LoadLocal` + u8 / `StoreLocal` + u8
- 常量 0-15：用 `LoadConst0`..`LoadConst15`
- 常量 16-255：用 `LoadConst` + u8

---

## 表达式编译

### 字面量

| AST | 字节码 |
|-----|--------|
| `LiteralInt(v)` | `add_constant(SMI(v))` → `emit_constant(idx)` |
| `LiteralFloat(v)` | `add_constant(float(v))` → `emit_constant(idx)` |
| `LiteralString(s)` | `add_constant(string_ptr)` → `emit_constant(idx)` |
| `LiteralTrue` | `LoadTrue` |
| `LiteralFalse` | `LoadFalse` |
| `LiteralNull` | `LoadNull` |
| `LiteralList(els)` | 逐个编译 els → `BuildList <u8 count>` |
| `JsonLiteral(entries)` | **逆序**编译每个 entry（先 value 后 key）→ `BuildJson <u8 count>` |

### 运算符

| AST | 字节码 |
|-----|--------|
| `Binary(+, l, r)` | l, r, `Add <cache_idx>` |
| `Binary(-, l, r)` | l, r, `Sub <cache_idx>` |
| `Binary(*, l, r)` | l, r, `Mul <cache_idx>` |
| `Binary(/, l, r)` | l, r, `Div <cache_idx>` |
| `Binary(%, l, r)` | l, r, `Mod <cache_idx>` |
| `Binary(==, l, r)` | l, r, `Equal <cache_idx>` |
| `Binary(!=, l, r)` | l, r, `NotEqual 0xFF` |
| `Binary(<, l, r)` | l, r, `Less <cache_idx>` |
| `Binary(<=, l, r)` | l, r, `LessEqual <cache_idx>` |
| `Binary(>, l, r)` | l, r, `Greater <cache_idx>` |
| `Binary(>=, l, r)` | l, r, `GreaterEqual <cache_idx>` |
| `Unary(-, x)` | x, `Neg` |
| `Unary(not, x)` | x, `Not` |
| `Grouping(x)` | x（无包装指令） |

cache_idx = `allocate_inline_cache()`；MVP 可以全部硬编码 `0xFF`。

### 逻辑 AND / OR

**AND**：
```
compile(left)
Dup
JumpIfFalse <end>        ; left 为假 → 跳到 end，栈上留着 left 作为结果
Pop                       ; left 为真 → 丢弃 left
compile(right)            ; 栈上是 right
patch end                 ; 无论哪条路径，栈顶就是结果
```

**OR**：
```
compile(left)
Dup
JumpIfFalse <eval_right>  ; left 为假 → 跳去算 right
Jump <end>                ; left 为真 → 跳过 right（栈顶是 left）
patch eval_right
Pop                        ; 丢弃 left
compile(right)             ; 栈上是 right
patch end
```

### VarRef（变量引用）

```
resolve_variable(name):
  Local(idx)    → emit_load_local(idx)
  Upvalue(idx)  → emit_load_upvalue(idx)
  None          → 全局查找：add_constant(name_str) → LoadGlobal <idx>
```

### FunctionCall

**路径 1：内置方法**（`list.push()` / `str.len()` / `json.keys()`）：
```
compile(receiver)
逐个编译 args
CallBuiltin <type_tag> <method_idx> <arg_count>
```

**路径 2：Struct 方法**：
```
compile(receiver)
逐个编译 args
LoadMethod <method_idx>     ; 弹出 receiver，推入 [receiver, method]
Call <argc>                 ; argc 含 receiver
```

**路径 3：通用函数调用**：
```
逐个编译 args（左到右）
compile(callee)
Call <argc>                 ; argc 不含 callee
```

### Lambda

1. 创建子 Compiler（`enclosing` 指向当前）
2. 为每个参数 `add_local(name)` + `mark_initialized()`
3. `compile(body)` 到子 Compiler 的 chunk
4. 子 chunk 末尾追加 `LoadNull; Return`
5. 收集子 Compiler 的 `upvalues` 列表
6. 函数放入父常量池
7. 在**父** chunk 发射：

```
Closure
<u8: const_idx>
<u8: upv_count>
<对每个 upv: u8 is_local + u8 index>
```

**方法编译**（impl 块内的方法）不同——不使用子 Compiler，而是**独立的 Compiler**（enclosing=null），所以方法不捕获闭包 upvalue。

### StructLiteral

```
struct_infos[name] → 取 shape_id + field_names
按 field_names 顺序，逆序编译每个 field 的值
BuildStruct <u16 shape_id LE> <u8 field_count>
```

### MemberAccess

**模块导出**：`compile(module); ModuleGet <u16 shape_id LE>`

**Struct 字段**（编译期已知）：`compile(struct); GetField <u8 field_idx>`

**通用**（JSON 等）：`compile(obj); emit_constant(key_str); IndexGet`

### IndexAccess

```
compile(object)
compile(index)
IndexGet
```

### As（类型转换）

| target_type | 指令 |
|-------------|------|
| `"int"` | `CastToInt` |
| `"float"` | `CastToFloat` |
| `"string"` | `CastToString` |
| `"bool"` | `CastToBool` |

---

## 赋值

### VarRef 赋值

```
compile(right)           ; 推入值
emit_store_local(idx)    ; 或 emit_store_upvalue(idx)
LoadNull                 ; 赋值表达式返回 null
```

### IndexAccess 赋值

```
compile(right)            ; 值
compile(index)            ; 下标
compile(object)           ; 容器
IndexSet                  ; 弹出值、下标、容器，设置，推入 null
```

### MemberAccess 赋值

```
compile(right)            ; 值
emit_constant(key_str)    ; key 字符串
compile(object)           ; 容器
IndexSet                  ; 同上
```

---

## 语句编译

### VarDecl

```
add_local(name) → idx
compile(initializer)       ; 推入值
mark_initialized()
emit_store_local(idx)      ; 弹入局部变量
```

若 `is_public`：记录到 `current_module.exports`，分配 shape_id。

### Return

| AST | 字节码 |
|-----|--------|
| `Return(Some(v))` | v, `ReturnValue` |
| `Return(None)` | `LoadNull`, `Return` |

### Print

```
compile(expr)
Print
```

### Block

```
begin_scope()
逐条 compile(stmt)
end_scope()              ; 弹出本块的 local
```

### ExprStmt

```
compile(expr)
Pop                      ; 丢弃表达式结果
```

### If

```
compile(condition)
JumpIfFalse <then_jump>  ; 条件为假，跳到 elif/else
compile(then_body)
Jump <end_jump>           ; 跳过 elif/else

patch then_jump
对每个 elif: 同上模式
若有 else: compile(else_body)
若无 else: LoadNull

patch end_jump（及所有 elif 的出口跳转）
```

### While

```
loop_start:
compile(condition)
JumpIfFalse <exit>
compile(body)
write_loop(loop_start)    ; JumpBack + 负偏移
patch exit
```

### For

```
begin_scope()

; 获取迭代器
compile(iterable)
GetIter                   ; → iterator
iter_idx = add_local("$iter"); mark_initialized(); emit_store_local(iter_idx)

; 声明循环变量
var_idx = add_local(name); mark_initialized()

loop_start:
emit_load_local(iter_idx)
IterNext                  ; → next_value | null
Dup
LoadNull
Equal <0xFF>              ; next == null ?
JumpIfFalse <real_value>  ; 不等 → 是有效值

; 是 null：迭代结束
Pop
Jump <exit>

; 有效值：赋值给循环变量
patch real_value:
emit_store_local(var_idx)

; body
compile(body)
write_loop(loop_start)

exit:
end_scope()
```

### Struct / Impl

Struct 定义**不产生字节码**——仅记录 `StructInfo` 到编译器元数据。

Impl 定义**不产生运行时指令**——为每个方法：
1. 用独立 Compiler 编译方法体
2. 函数放入常量池
3. 追加 `MethodTableEntry`（VM 初始化时注册到 Shape）
4. 若方法名以 `"operator "` 开头 → 追加 `OperatorTableEntry`

### Import

**`from module import item`**：
```
add_local(item_name) → idx
emit_constant(module_name_str)
GetModule                 ; → module
emit_constant(item_name_str)
GetModuleExport <item_idx> ; → export value
emit_store_local(idx)
mark_initialized()
```

**`import module`**：无字节码。记录到 `imported_modules`。

**`import module as alias`**：
```
add_local(alias) → idx
emit_constant(module_name_str)
GetModule                 ; → module
emit_store_local(idx)
mark_initialized()
```

---

## 模块编译 (compile_module)

```
对每条顶层 stmt: compile(stmt)
LoadNull
Return
```

返回 `(chunk, max_locals)`。模块编译为一个匿名函数，VM 调用它来执行模块顶层代码。

---

## 跳转偏移规则

- **前向跳转**（Jump, JumpIfFalse）：`write_jump` 写占位符 → 后续代码 → `patch_jump` 计算 `(当前末尾 - 占位符位置 - 2)` i16 LE
- **后向跳转**（循环）：`write_loop(loop_start)` → 计算 `-(当前末尾 - loop_start + 2)` i16 LE
- 偏移从**操作数字节之后的下一条指令**算起

---

## 常量池

- 最大 255 条/Chunk（u8 索引）
- 索引 0-15：专用单字节指令
- 索引 16-255：`LoadConst` + u8
- 存储类型：SMI、Float、String 指针、Function 指针

---

## MVP 简化

| 保留 | 暂不实现 |
|------|---------|
| 字面量（int/float/string/bool/null/list/json） | struct 字面量 + impl 方法 |
| 全部算术/比较/逻辑运算符 | 运算符重载、inline cache |
| VarDecl、赋值、Print、Return | 模块系统、import |
| If/While/For 全支持 | 闭包/upvalue（先用全局变量） |
| Lambda + 函数调用 | 方法调用、LoadMethod |
| Block 作用域 | CastToXxx（用内置函数替代） |
