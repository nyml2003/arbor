# Kaubo 字节码指令集规格

从 kaubo-features/next_kaubo `vm/core/bytecode.rs` 提取。CodeGen 和 VM 实现的共同参考。

---

## 操作数编码

多字节操作数**小端序**。

| 操作数类型 | 字节数 | 读取函数 |
|-----------|--------|---------|
| u8 | 1 | `code[ip]; ip += 1` |
| u16 LE | 2 | `(code[ip+1] << 8) | code[ip]; ip += 2` |
| i16 LE | 2 | sign-extend 同上 |
| 变长 (CLOSURE) | 1+1+2N | 基础 2 字节，然后每个 upvalue 2 字节 |

---

## 完整指令表

### 常量加载 (0x00–0x1C)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x00–0x0F | `LOAD_CONST0`–`LOAD_CONST15` | 无 | → const[0..15] | 常量池前 16 个槽位直接寻址 |
| 0x10 | `LOAD_CONST` | u8: idx | → const[idx] | |
| 0x11 | `LOAD_CONST_WIDE` | u16: idx | → const[idx] | 常量池超过 256 时用 |
| 0x18 | `LOAD_NULL` | 无 | → null | |
| 0x19 | `LOAD_TRUE` | 无 | → true | |
| 0x1A | `LOAD_FALSE` | 无 | → false | |
| 0x1B | `LOAD_ZERO` | 无 | → 0 | SMI 零 |
| 0x1C | `LOAD_ONE` | 无 | → 1 | SMI 一 |

### 栈操作 (0x20–0x22)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x20 | `POP` | 无 | v → | 丢弃栈顶 |
| 0x21 | `DUP` | 无 | → copy_of_top | 复制栈顶 |
| 0x22 | `SWAP` | 无 | a,b → b,a | 交换栈顶两值 |

### 局部变量 — 加载 (0x30–0x38)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x30–0x37 | `LOAD_LOCAL0`–`LOAD_LOCAL7` | 无 | → locals[0..7] | |
| 0x38 | `LOAD_LOCAL` | u8: idx | → locals[idx] | |

### 局部变量 — 存储 (0x40–0x48)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x40–0x47 | `STORE_LOCAL0`–`STORE_LOCAL7` | 无 | v → | 写入 frame.locals[0..7] |
| 0x48 | `STORE_LOCAL` | u8: idx | v → | 写入 frame.locals[idx]，数组自动扩展 |

### 全局变量 (0x50–0x52)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x50 | `LOAD_GLOBAL` | u8: name_const_idx | → globals[name] | 未定义报错 |
| 0x51 | `STORE_GLOBAL` | u8: name_const_idx | v → | 已存在则覆盖 |
| 0x52 | `DEFINE_GLOBAL` | u8: name_const_idx | v → | 同 STORE_GLOBAL |

### 算术 + 比较 (0x60–0x78)

每个带 `cache_idx` 的指令：`0xFF` = 不用 inline cache，只用 Level 3 查表。

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x60 | `ADD` | u8: cache_idx | b,a → a+b | 运算符重载：查 operator `add` |
| 0x61 | `SUB` | u8: cache_idx | b,a → a-b | |
| 0x62 | `MUL` | u8: cache_idx | b,a → a*b | |
| 0x63 | `DIV` | u8: cache_idx | b,a → a/b | 结果总是浮点 |
| 0x64 | `MOD` | u8: cache_idx | b,a → a%b | |
| 0x68 | `NEG` | 无 | a → -a | 一元取负 |
| 0x70 | `EQUAL` | u8: (占位符) | b,a → bool | 值相等比较 |
| 0x71 | `NOT_EQUAL` | 0* | b,a → bool | *执行时仍读 1 字节占位符 |
| 0x72 | `GREATER` | u8: cache_idx | b,a → bool | |
| 0x73 | `GREATER_EQUAL` | u8: cache_idx | b,a → bool | |
| 0x74 | `LESS` | u8: cache_idx | b,a → bool | |
| 0x75 | `LESS_EQUAL` | u8: cache_idx | b,a → bool | |
| 0x78 | `NOT` | 无 | a → !bool(a) | 真值取反 |

### 控制流 (0x80–0x82)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x80 | `JUMP` | i16: offset | — | 从**下一条指令**偏移 |
| 0x81 | `JUMP_IF_FALSE` | i16: offset | cond → | 弹出条件，为假则跳 |
| 0x82 | `JUMP_BACK` | i16: offset | — | 无条件跳转（语义上向后，便宜为负） |

**偏移基准**：所有跳转的 offset 从**操作数之后的字节**算起。例如 `JUMP 3` 的 3 是从 `JUMP` 指令 + 2 字节操作数之后的下一个字节开始算。

### 函数 / 闭包 (0x90–0x96)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0x90 | `CALL` | u8: argc | argN,...,arg0,callee → result | argc 不含 callee |
| 0x91 | `CLOSURE` | u8: const_idx + u8: upv_cnt + 每 upv: u8:is_local + u8:index | → closure | 从函数创建闭包 |
| 0x92 | `GET_UPVALUE` | u8: idx | → upvalue[idx] | |
| 0x93 | `SET_UPVALUE` | u8: idx | v → | peek 栈顶不弹出 |
| 0x94 | `CLOSE_UPVALUES` | u8: slot_threshold | — | 关闭阈值以上的所有 open upvalue |
| 0x95 | `RETURN` | 无 | — | 弹出栈帧，推入 null |
| 0x96 | `RETURN_VALUE` | 无 | v → v | 弹出栈帧，保留栈顶返回值 |

### 列表 / 迭代 (0xB0–0xB4)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0xB0 | `BUILD_LIST` | u8: count | count 个元素 → list_obj | 从栈顶取 count 个值 |
| 0xB1 | `INDEX_GET` | 无 | idx,obj → obj[idx] | 支持 list/string/json/struct |
| 0xB2 | `INDEX_SET` | 无 | val,key,obj → | |
| 0xB3 | `GET_ITER` | 无 | iterable → iterator | |
| 0xB4 | `ITER_NEXT` | 无 | iterator → next \| null | null 表示迭代结束 |

### JSON / 模块 / Struct / 类型转换 (0xC0–0xE3)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0xC0 | `BUILD_JSON` | u8: kv_count | count keys + count vals → json_obj | 栈顶先 vals 后 keys |
| 0xC1 | `JSON_GET` | 无 | key,json → value | |
| 0xC2 | `JSON_SET` | 无 | val,key,json → | |
| 0xD0 | `BUILD_MODULE` | u8: export_count | exports → module | |
| 0xD1 | `MODULE_GET` | u16: shape_id | module → export | |
| 0xD2 | `GET_MODULE_EXPORT` | u8: name_const_idx | module → export | 动态按名字查 |
| 0xD3 | `GET_MODULE` | 无 | name_str → module | 加载或获取模块 |
| 0xD8 | `BUILD_STRUCT` | u16: shape_id + u8: field_count | field vals → struct | |
| 0xD9 | `GET_FIELD` | u8: field_idx | struct → value | |
| 0xDA | `SET_FIELD` | u8: field_idx | val,struct → | |
| 0xDB | `LOAD_METHOD` | u8: method_idx | (peek receiver) → method | 不弹出 receiver |
| 0xDC | `CALL_BUILTIN` | u8: type_tag + u8: method_idx + u8: argc | args + receiver → result | 直接调内置方法 |
| 0xE0 | `CAST_TO_INT` | 无 | v → int \| null | |
| 0xE1 | `CAST_TO_FLOAT` | 无 | v → float \| null | |
| 0xE2 | `CAST_TO_STRING` | 无 | v → string \| null | |
| 0xE3 | `CAST_TO_BOOL` | 无 | v → bool | |

### 调试 / 无效 (0xF0–0xFF)

| Hex | 助记符 | 操作数 | 栈效果 | 说明 |
|-----|--------|--------|--------|------|
| 0xF0 | `PRINT` | 无 | v → | 弹出并打印 |
| 0xFF | `INVALID` | 无 | — | 运行时错误 |

---

## 兼容缺陷记录

从原实现提取，新实现需要注意：

1. **NOT_EQUAL (0x71)**：`operand_size()` 返回 0，但 VM 执行时读了 1 字节 `_cache_idx`。CodeGen 必须 emit 一个 `0x00` 占位字节。
2. **EQUAL (0x70)**：`operand_size()` 返回 1，但 VM 读完后用 `_cache_idx` 忽略。cache_idx 有意义但未被实际使用。
3. **Inline cache**：MVP 可以先不做，所有 cache_idx 写 `0xFF` 即可。Level 3 查表对 MVP 足够了。
