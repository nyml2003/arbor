# Kaubo VM 架构规格

从 kaubo-features/next_kaubo `vm/runtime/vm/` 提取。VM 实现的参考文档。

---

## Value：NaN-Boxing

64 位值，借用 IEEE 754 NaN 空间编码非浮点类型。

```
bits: [63 sign] [62:52 exponent] [51 quiet] [50:44 tag] [43:0 payload]

QNaN 基址: 0x7FF8_0000_0000_0000
Tag 掩码:   0x7F << 44  (bits 50-44)
Payload:    0x000F_FFFF_FFFF_FFFF  (bits 43-0)
```

### 判断是浮点还是装箱

```rust
fn is_float(v: u64) -> bool {
    (v & 0x7FF8_0000_0000_0000) != 0x7FF8_0000_0000_0000
}
// 普通 double 的 exponent 不是全 1，所以不匹配 QNaN
```

### Tag 表

| Tag | 值 (shifted) | 含义 | Payload |
|-----|-------------|------|---------|
| 0 | 0 | NaN（语言级，未使用） | — |
| 1 | 1 << 44 | null | 任意 |
| 2 | 2 << 44 | true | 任意 |
| 3 | 3 << 44 | false | 任意 |
| 4 | 4 << 44 | **SMI**（31-bit 有符号整数） | bits 30-0 存值 |
| 8–23 | (8..24) << 44 | **内联整数** -8..+7 | 必须为 0 |
| 32 | 32 << 44 | 通用堆对象 | 指针 |
| 33 | 33 << 44 | String (ObjString) | 指针 |
| 34 | 34 << 44 | Function (ObjFunction) | 指针 |
| 35 | 35 << 44 | List (ObjList) | 指针 |
| 36 | 36 << 44 | Iterator (ObjIterator) | 指针 |
| 37 | 37 << 44 | Closure (ObjClosure) | 指针 |
| 38 | 38 << 44 | Coroutine (ObjCoroutine) | 指针 |
| 39 | 39 << 44 | Result (ObjResult) | 指针 |
| 40 | 40 << 44 | Option (ObjOption) | 指针 |
| 41 | 41 << 44 | JSON (ObjJson) | 指针 |
| 42 | 42 << 44 | Module (ObjModule) | 指针 |
| 43 | 43 << 44 | Native (ObjNative) | 指针 |
| 44 | 44 << 44 | NativeVm (ObjNativeVm) | 指针 |
| 45 | 45 << 44 | Struct (ObjStruct) | 指针 |
| 46 | 46 << 44 | Shape (ObjShape) | 指针 |

### SMI 编码

- 范围：`-2^30 .. 2^30-1`（约 -10 亿到 +10 亿）
- 存储在 payload bits 30-0
- 符号扩展：若 bit 30 为 1，高位填 1

### 内联整数编码

- Tag = n + 16，payload = 0
- -8 → tag=8, 0 → tag=16, +7 → tag=23

### 堆指针编码

```rust
// 编码：(ptr >> 3) & PAYLOAD_MASK，嵌在 tag + qnan 中
// 解码：(compressed << 3) as *mut T
// 要求：所有堆对象 8 字节对齐
```

---

## VM 结构

```rust
struct VM {
    stack: Vec<Value>,                 // 共享操作数栈
    frames: Vec<CallFrame>,            // 调用栈（最后一个 = 当前帧）
    open_upvalues: Vec<*mut ObjUpvalue>, // 当前活跃的 upvalue
    globals: HashMap<String, Value>,   // 全局变量
    shapes: HashMap<u16, *const ObjShape>, // Shape 注册表
    inline_caches: Vec<InlineCacheEntry>,  // 内联缓存
    builtin_methods: BuiltinMethodTable,   // 内置方法
    builtin_modules: HashMap<String, Box<ObjModule>>, // 内置模块
    loaded_modules: HashMap<String, LoadedModule>, // 已加载模块
    loading_stack: Vec<String>,        // 循环依赖检测
}

// 默认配置:
//   stack 初始容量: 256
//   frames 初始容量: 64
//   inline_caches 初始容量: 64
```

---

## CallFrame 结构

```rust
struct CallFrame {
    closure: *mut ObjClosure,   // 当前闭包
    ip: *const u8,              // 指令指针 → closure.function.chunk.code
    locals: Vec<Value>,         // 局部变量数组（自动扩展）
    stack_base: usize,          // 帧创建时 vm.stack.len()
}
```

- `locals` **不在共享栈上**，每个帧独立持有
- `stack_base` 用于 upvalue 关闭：计算栈上局部变量地址，判断哪些 upvalue 需要关闭
- `closure.function.chunk` 是当前执行的字节码

---

## 函数调用机制

### CALL 指令的栈布局

执行 `CALL(argc)` 前，栈必须是：

```
[arg0] [arg1] ... [argN] [callee]    ← 栈顶
└─── argc 个参数 ───┘
```

**argc 不包含 callee 自身**。例如 `add(2, 3)` → argc=2，栈上是 `[2, 3, <fn add>]`。

### CALL 的执行流程

1. 读 `argc = code[ip]; ip += 1`
2. 弹出 callee = `stack.pop()`
3. 判断 callee 类型：

   **a. 编码的内置方法**（SMI 值在 0x0100..0x01FF 范围内）：
   - 解码 `(type_tag, method_idx)`
   - 弹出 argc 个值（包含 receiver 在 index 0）
   - 反转参数数组（栈顶的最后一个参数变成数组的第一个）
   - 查 `builtin_methods` 调用
   - **不创建新栈帧。** 结果推回栈顶。

   **b. Closure (ObjClosure*)**：
   - 弹出 argc 个值
   - 反转后作为新帧的 `locals`
   - `stack_base = vm.stack.len()`
   - IP = `closure.function.chunk.code` 开头
   - 新帧 push 到 `vm.frames`

   **c. Native (ObjNative*)**：
   - arity = 255 表示可变参数
   - 弹出 argc 个值，反转，调用 `native.call(&args)`
   - 结果推回栈顶。**不创建新帧。**

   **d. None of the above** → 尝试 `operator call`

### RETURN (0x95)
1. 关闭当前帧的所有 upvalue（阈值 slot=0）
2. 弹出当前帧
3. 推入 `null`
4. 若 frames 为空 → 返回 `InterpretResult::Ok`

### RETURN_VALUE (0x96)
1. 关闭当前帧的所有 upvalue（阈值 slot=0）
2. 弹出当前帧
3. 弹出返回值
4. **把返回值重新推入栈**（现在它在父帧的栈区域里）
5. 若 frames 为空 → 返回 `InterpretResult::Ok`

### 关键约束
- 栈是**跨帧共享**的。函数的参数和返回值通过共享栈传递。
- `locals` **不放在共享栈上**（和 CPython 的 `fastlocals` / Lua 的寄存器不同）。Frame 有自己的 `locals: Vec<Value>`。
- `stack_base` 记录帧边界，用于 upvalue 地址计算。

---

## Upvalue 捕获

### CLOSURE 指令 (0x91)

```
编码: [0x91] [u8: const_idx] [u8: upv_count] [对每个 upv: u8 is_local + u8 index]
```

1. 从常量池读 const_idx → 必须是 ObjFunction
2. 读 upv_count
3. 创建 ObjClosure
4. 对每个 upvalue：
   - 若 `is_local != 0`：取当前帧 locals 的指针，调 `capture_upvalue`——搜索 `open_upvalues`，同地址则复用，否则创建新 ObjUpvalue
   - 若 `is_local == 0`：从**当前闭包**的 upvalue 列表取（捕获外层已捕获的 upvalue）
5. 推闭包到栈顶

### ObjUpvalue

```rust
struct ObjUpvalue {
    location: *mut Value,      // 指向栈上局部变量；关闭后为 null
    closed: Option<Value>,      // 关闭后的值副本
}
```

`get()`：若 `closed` 有值，返回它；否则 deref `location`
`set(v)`：写入 `closed`（若已关闭），否则写入 `*location`
`close()`：复制 `*location` 到 `closed`，置 `location = null`

### GET_UPVALUE (0x92) / SET_UPVALUE (0x93)
- `GET_UPVALUE`：读 u8 idx → `closure.upvalues[idx].get()` → 推入栈
- `SET_UPVALUE`：读 u8 idx → **peek 栈顶**（不弹出）→ `closure.upvalues[idx].set(value)`

### CLOSE_UPVALUES (0x94)
- 读 u8 slot：计算阈值地址 = `frame.locals.as_ptr() + slot * sizeof(Value)`
- 遍历 `open_upvalues`：所有 `location >= threshold` 的 upvalue 执行 `close()`
- 从 `open_upvalues` 中移除已关闭的

---

## 操作数栈

```rust
// 基本操作
vm.stack.push(value)                         // push
vm.stack.pop().expect("Stack underflow")     // pop
vm.stack[vm.stack.len() - 1 - distance]      // peek(distance): 0=栈顶

// pop_two() 返回 (a, b): b 是栈顶, a 是次顶
// 用于二元运算: 先推入的左操作数是 a，后推入的右操作数是 b
```

---

## 全局变量

```rust
vm.globals: HashMap<String, Value>
```

- `DEFINE_GLOBAL name_idx`：常量池取名字，弹栈顶值，插入 globals
- `LOAD_GLOBAL name_idx`：常量池取名字，查 globals，推入栈；未定义则报错
- `STORE_GLOBAL name_idx`：**同 DEFINE_GLOBAL**——实现上无区别

---

## 运算符分派 (三级)

### Level 1：直接原语
字符串拼接、SMI 运算、浮点运算——已知类型直接算。

### Level 2：内联缓存
若 `cache_idx != 0xFF`，检查 `inline_caches[cache_idx]`：
- left_shape + right_shape 都匹配 → 直接调缓存的闭包
- 不匹配 → 执行 Level 3 查找，更新缓存

MVP 可以全部写 `0xFF` 跳过。

### Level 3：全量查找
`find_operator(shape_id, operator)` → 查 Shape 的 `operators` 表。若左操作数不支持，尝试反向运算符（`__radd` 等）。

---

## 内置 Shape ID

| ID | 类型 |
|----|------|
| 0 | Int / SMI |
| 1 | Float |
| 2 | String |
| 3 | List |
| 4 | JSON |
| 5 | Function / Closure |
| 6 | Module |

Struct 的 shape_id 编译时分配，从 100 开始。

---

## 内置方法编码

List / String / JSON 的方法通过 SMI 编码传递，CALL 指令检测后直接分派，不经过 Level 1-3。

```rust
fn encode_method(type_tag: u8, method_idx: u8) -> i32 {
    0x0100 + ((type_tag as i32) << 4) + method_idx as i32
}
```

| type_tag | 类型 |
|----------|------|
| 0 | List |
| 1 | String |
| 2 | JSON |

**List 方法**：push(0), len(1), remove(2), clear(3), is_empty(4), foreach(5), map(6), filter(7), reduce(8), find(9), any(10), all(11)

**String 方法**：len(0), is_empty(1)

**JSON 方法**：len(0), is_empty(1)

---

## 模块系统

### 模块加载流程
1. `GET_MODULE`：弹栈顶模块名（字符串）
2. 若 `loaded_modules` 已缓存 → 直接推模块到栈
3. 若 `builtin_modules` 里找到 → 初始化后缓存并推入
4. 否则报错

### 模块导出
- `BUILD_MODULE count`：弹 count 个导出值，创建 Module 对象
- `MODULE_GET shape_id`：按编译时 shape ID 取导出
- `GET_MODULE_EXPORT name_idx`：按运行时名字取导出

---

## 错误类型

| 错误 | 触发条件 |
|------|---------|
| `TypeError` | 运算符不支持的类型组合 |
| `UndefinedVariable` | LOAD_GLOBAL 找不到名字 |
| `IndexOutOfBounds` | list[idx] 越界 |
| `DivisionByZero` | 整数除零 |
| `StackOverflow` | 栈或帧超过限制 |
| `RuntimeError` | 其他运行时异常 |

---

## MVP 简化

相对于原实现的复杂度，MVP VM 可以做以下简化：

| 保留 | 暂不实现 |
|------|---------|
| SMI + 内联整数 + 浮点 + null/bool | NaN-boxing 完整实现（可以先用 enum 代替） |
| 基本对象：String, List, Function, Closure | ObjResult, ObjOption, ObjCoroutine |
| CALL / RETURN / RETURN_VALUE | CLOSURE / GET_UPVALUE / SET_UPVALUE / CLOSE_UPVALUES |
| LOAD_GLOBAL / STORE_GLOBAL / DEFINE_GLOBAL | 模块系统 (GET_MODULE 等) |
| 基础运算符 (ADD, SUB, MUL, DIV, MOD, NEG, NOT) | 运算符重载 / inline cache |
| PRINT, JUMP, JUMP_IF_FALSE, JUMP_BACK | JUMP_BACK（用 JUMP 替代即可） |
| 列表基本操作 (BUILD_LIST, INDEX_GET, INDEX_SET) | GET_ITER / ITER_NEXT（for 循环可改用 while+下标） |
| 类型转换 (CAST_TO_INT 等) | Type 相关的 SHAPE/STRUCT 体系 |
| — | 协程 (CreateCoroutine / Resume / Yield) |
| — | JSON 对象（先用列表和字符串） |
