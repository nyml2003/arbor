# 模式：自研 Python 风格语言引擎（kaubo）

## 一句话

从零构建一个 Python 兼容的脚本语言引擎——手写 Lexer（状态机）、Parser（递归下降）、ByteCode VM（栈式 50+ 指令）、Python 兼容对象模型（PyObject 继承树）、GC、C ABI 导出。

## 核心架构

```
Lexer（状态机，手写）
  │  Token 流
  ▼
Parser（递归下降）
  │  AST
  ▼
IR（表达式/语句/类/函数/模块）
  │  语义分析 + IR 变换
  ▼
Generator（AST → ByteCode）
  │  ByteCode 序列
  ▼
VirtualMachine（栈式 VM）
  │  执行 ByteCode
  ▼
Object Model（PyObject 继承树）
     PyInteger, PyString, PyList, PyDict, PyFunction, PyCode, PyFrame...

Runtime：EventLoop + GarbageCollector + BinaryFileParser
Binding：C_API（extern "C" 导出）
```

## 关键设计

### 1. Python 兼容 ByteCode

```cpp
enum class ByteCode : uint8_t {
  POP_TOP = 1,        NOP = 9,
  UNARY_NEGATIVE = 11, UNARY_NOT = 12,
  BINARY_ADD = 23,     BINARY_SUBTRACT = 24,
  LOAD_CONST = 100,    LOAD_NAME = 101,
  LOAD_ATTR = 106,     COMPARE_OP = 107,
  JUMP_ABSOLUTE = 113, POP_JUMP_IF_FALSE = 114,
  RETURN_VALUE = 83,   YIELD_VALUE = 86,
  // 50+ 指令
};
```

指令码和 CPython 一致（`LOAD_CONST = 100`，`BINARY_ADD = 23`）——不是巧合，这是 Python bytecode 兼容层。CPython 的 `.pyc` 文件可能可以直接加载。

### 2. 对象模型：PyObject 继承树

```
PyObject（基类）
├── PyNone
├── PyBoolean
├── PyInteger
├── PyFloat
├── PyString
├── PyBytes
├── PyList
├── PyDictionary
├── PyFunction / PyMethod / PyNativeFunction / PyIIFE
├── PyType（类对象）
├── PyCode（编译后的代码对象）
├── PyFrame（栈帧）
├── PySlice
├── PyGenerator
└── PyPromise
```

所有运行时对象继承 `PyObject`。`PyType` 本身也是 `PyObject`——类型系统是自举的（Python 风格：type 的 type 是 type）。

### 3. VirtualMachine：单例 + 帧栈

```cpp
class VirtualMachine {
    static VirtualMachine& Instance();  // 单例
    void Run(const PyCodePtr& code);
    void SetFrame(const PyFramePtr& child);
    void BackToParentFrame();
    PyFramePtr CurrentFrame() const;
};
```

单例 VM 持有当前帧。函数调用 = 创建子帧 → push → 执行 → pop。和 CPython 的 `PyEval_EvalFrame` 模型一致。

### 4. Genesis：运行时自举

```cpp
Object::PyDictPtr Genesis() {
    LoadBootstrapClasses();       // 注册 PyObject, PyType, PyInteger...
    LoadRuntimeSupportClasses();  // 注册 PyList, PyDict, PyFunction...
    auto builtins = PyDictionary::Create();
    builtins->Put("None", PyNone::Create());
    builtins->Put("print", PyNativeFunction::Create(Print));
    // ...
}
```

`Genesis()` 在 VM 启动时调用一次——注册所有内置类型和函数。之后用户代码可以直接 `print("hello")`。

### 5. 手写 Lexer（状态机，非 Flex）

```
Lexer/
├── Builder.h         ← Lexer 构造器
├── Machines.h        ← 状态机定义
├── StateMachine/     ← 状态机框架
├── Token/            ← Token 类型 + 约束
└── Core/             ← 核心抽象
```

和 ofsh 的 lexer 一样——手写，无 Flex/Bison。但 kaubo 的实现更工程化：状态机框架是抽象出来的，Token 类型有约束系统。

### 6. 自建 JSON 解析器（作为子工具）

```
Tools/Json/
├── Lexer/Builder.cpp       ← JSON 词法分析
├── Lexer/Machines.h        ← JSON 状态机
├── Parser/Parser.cpp       ← JSON 语法分析
└── Parser/Value.cpp        ← JSON 值对象
```

语言引擎需要 JSON 解析器——不是引入 `nlohmann/json`，而是自己写。Lexer 和语言 Lexer 共享状态机框架。

### 7. 工具链：C API + Python 桥接

```
engine/src/Binding/C_API/c_api.h   ← extern "C" 导出 DLL
tools/dll/native_compiler_bridge.py ← Python 调 C API
```

C++ 引擎编译为 DLL，Python 工具通过 ctypes 调用。和 `dll_csv_transformer` 的模式完全一致——但规模大一百倍。

### 8. EventBus + Singleton + Config

```
Tools/
├── DesignPattern/Singleton.h    ← 单例模板
├── EventBus/EventBus.cpp        ← 事件总线
├── Config/Config.cpp            ← 配置系统
└── Terminal/                    ← 终端输出（多策略）
    ├── ConsoleTerminal.cpp
    ├── FileLogStrategy.cpp
    ├── ProxyTerminalStrategy.cpp
    └── VerboseTerminal.h
```

引擎自带基础设施——不引入外部库。EventBus 做模块间解耦，Config 做单例配置，Terminal 支持多输出策略（控制台/文件/代理/Verbose）。

## kaubo-features：实验性功能

```
kaubo-features/
├── gc/              ← C++ 垃圾回收器（GC.cpp, GCObject, GCPtr, Klass）
│   └── 自研 GC：mark-sweep，GC 根追踪，类型映射（Klass）
├── lexer/           ← JSON lexer（更早版本的实验）
└── next_kaubo/      ← Rust 重写（kaubo-cli, kaubo-config, kaubo-log, kaubo-orchestrator）
    └── docs/        ← 架构文档（module compilation, v2 migration）
```

## 反模式警示

### ❌ 引入外部 JSON 库

语言引擎只需要解析 JSON 配置文件和测试数据。但引入 `nlohmann/json` 或 `rapidjson` 意味着依赖外部维护者的发布周期。自己写的 JSON 解析器 200 行，和语言 Lexer 共享框架——投入小，控制权大。

### ❌ 对象模型不用继承树

如果 PyInteger、PyString、PyList 不是同一个 PyObject 继承树，VM 的通用操作（`STORE_NAME` 存一个对象到变量、`LOAD_ATTR` 读对象属性）需要每个类型各写一份。

## 来源

- kaubo 源码（`engine/src/` 全部模块）
- kaubo-features 源码（`gc/`、`lexer/`、`next_kaubo/`）
- 2026-06-07 agent 阅读后提炼
