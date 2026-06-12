# 模式：C++ 代码通过 C ABI 导出 DLL（dll_csv_transformer）

## 一句话

用 `extern "C"` + 跨平台导出宏 + CMake SHARED 库，把 C++ 代码封装为 C ABI 的 DLL/SO。外部调用方不需要知道 C++ 的存在。

## 为什么需要这个模式

C++ 有 name mangling——同一个函数名在不同编译器、不同版本下编译出的符号名不同。外部调用方（Python ctypes、C# P/Invoke、Rust FFI、另一个 C++ 编译器编译的代码）无法直接链接 C++ 的 .dll/.so。

解决方案：**暴露一层 C ABI**。C 的符号名是稳定的，`extern "C"` 关闭了 name mangling。C++ 实现在内部，外面只看到函数指针参数的 C 接口。

## 核心架构

```
C++ 实现（c_abi.cpp）
  │  extern "C" { EXPORT_API void add(...) }
  ▼
C ABI 头文件（c_abi.h）
  │  __declspec(dllexport) / __attribute__((visibility("default")))
  ▼
CMake SHARED 库（CMakeLists.txt）
  │  add_library(MyDLL SHARED ...)
  ▼
外部调用方（Demo, Python ctypes, C#, Rust FFI...）
```

## 关键设计

### 1. 跨平台导出宏

```cpp
#if defined(_WIN32)
  #if defined(EXPORT_BUILDING_DLL)
    #define EXPORT_API __declspec(dllexport)
  #elif defined(EXPORT_USING_DLL)
    #define EXPORT_API __declspec(dllimport)
  #else
    #define EXPORT_API
  #endif
#else
  #define EXPORT_API __attribute__((visibility("default")))
#endif
```

四态切换：
- `EXPORT_BUILDING_DLL` 定义时 = 导出（编译 DLL 本身）
- `EXPORT_USING_DLL` 定义时 = 导入（链接 DLL 的调用方）
- 都没定义 = 静态链接
- Linux/macOS 用 `visibility("default")`

### 2. C 接口声明

```cpp
#ifdef __cplusplus
extern "C" {
#endif

EXPORT_API void add(int a, int b, int *result);
EXPORT_API void sub(int a, int b, int *result);

#ifdef __cplusplus
}
#endif
```

约束：
- 参数和返回值必须是 C 兼容类型（`int`、`float`、`void*`、指针）
- 不能传 C++ 对象（`std::string` 不行，`const char*` 可以）
- 输出参数用指针（`int *result`）
- 调用方负责分配内存

### 3. 空指针保护

```cpp
EXPORT_API void add(int a, int b, int* result) {
  if (result != nullptr) {
    *result = a + b;
  }
}
```

C ABI 没有 Rust 的 `Option<&T>` 或 TS 的类型安全——调用方传错指针直接 crash。防御性空指针检查是最低成本的保护。

### 4. CMake SHARED 库

```cmake
add_library(MyDLL SHARED
    src/c_abi.cpp
    src/c_abi.h
)
target_compile_definitions(MyDLL PRIVATE EXPORT_BUILDING_DLL)

# Demo 链接 DLL
add_executable(Demo src/main.cpp)
target_include_directories(Demo PRIVATE src)
target_link_libraries(Demo PRIVATE MyDLL)
```

CMake 的一个 SHARED 目标在 Windows 上输出 `.dll` + `.lib`，在 Linux 上输出 `.so`。`EXPORT_BUILDING_DLL` 只在 DLL 自己的编译中定义，调用方不需要。

## 常见坑

### ❌ 头文件里直接 include C++ 标准库

C 调用方 include 头文件时会报错。C 接口头文件只能用 `#include <cstdint>` 这种 C 兼容头。

### ❌ 在 C ABI 边界传 C++ 对象

```cpp
// ❌ std::string 不能越过 C ABI
EXPORT_API void process(std::string input);

// ✅ const char* + 长度可以
EXPORT_API void process(const char* input, int len);
```

### ❌ DLL 端分配内存，调用端释放

不同编译器/运行时可能用不同的 allocator。要么同侧分配同侧释放，要么提供显式的 `free_result()` 导出函数。

## 来源

- dll_csv_transformer 源码（`src/c_abi.h`、`src/c_abi.cpp`、`CMakeLists.txt`）
- 2026-06-07 agent 阅读后提炼
