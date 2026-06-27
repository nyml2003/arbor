# Kaubo VM 性能优化 Phase 1

**日期**: 2026-06-28

## 背景

Kaubo v0.1.8 完成了核心语言特性（struct/enum/lambda/match/interface）后，benchmark 显示比 CPython 慢 ~200x。经过分析，瓶颈不在算法——Kaubo 是 JVM/WASM 派"类型在指令里"的架构，编译期已消解类型——而在 VM 调度开销。

## 已完成的优化

### Wave 1: VM 热路径去分配

**Branch 去 `to_vec()`**：每次 Branch 从 3 次 `Vec::to_vec()` 堆分配降为 1 次切片克隆。loop benchmark (40,000 次迭代 × 2 Branch = 80,000 次分配) 直接受益。

**Call 寄存器复用**：`mem::take` + `resize` 替代 `vec![0; n]` × 2。消除函数调用时的双数组零填充。

**效果**：整体提速 ~2x，Kaubo vs Python 从 294x 降到 196x。

### Wave 2: 统一寄存器组

`RegFile { ints: Vec<i64>, floats: Vec<f64> }` → `RegFile { regs: Vec<u64> }`。

理由：Kaubo 编译期知道类型，opcode 决定位模式解释方式（`AddInt` → `as i64`，`FAdd` → `f64::from_bits`），不需要运行时分离寄存器组。消除了 `write_int`/`write_float`/`write_bool` 三函数的跨数组同步。

### Wave 3: 中端优化

**空 block 消除**：消除 `[ ] | Jump(target, [])` 空转发块，重写前驱跳转目标。loop benchmark 块数从 17 降到 16。

**Move 折叠**：`BinOp(r9, AddInt, total, r8); Move(total, r9)` → `BinOp(total, AddInt, total, r8)`。消除 temp 寄存器和冗余 Move 指令。fib 指令数 16→15，fact 20→18。

### Wave 4: Benchmark 工具修复

- 修复 sieve.kaubo 缺少 `break` 导致算法不公平（Python 找到因子跳出，Kaubo 跑完整个内层循环）
- 用 Rust 重写 `kaubo2 bench` 命令，替代有 Windows GBK 编码问题的 Python benchmark runner
- 修复 benchmark runner 的 `.exe` 检测和 subprocess cwd 问题

## 性能水位

| Benchmark | Rust | Node.js V8 | CPython 3.13 | Kaubo |
|-----------|------|------------|-------------|-------|
| fib(40) | 0.05μs | 6.6μs | 0.7μs | 1.6μs |
| fact(12) | 0.04μs | 6.5μs | 1.7μs | 3.9μs |
| loop 200×200 | 0.04μs | 63.5μs | 716μs | 1,520μs |
| pipeline 100K | 26.5μs | 63.6μs | 2,985μs | 6,652μs |
| sieve 100K | 3,418μs | 3,258μs | 78,113μs | 129,565μs |

**Kaubo vs CPython: 几何平均 2.0x**

## 剩余优化空间

中端（不碰 VM）：
- 循环不变量外提（~60 行）
- LoadImm 立即数指令（~30 行）
- 寄存器活性分析 + 重编号（~100 行）
- Peephole 合并（~40 行）

VM 层：
- Superinstructions 融合
- 直接线程化调度（computed goto）
- 轻量 JIT 模板

## 关键教训

1. **先修 benchmark 公平性再优化**。sieve 从 932ms→130ms 只是加了 `break`，不是优化 VM。
2. **纯中端优化不动 VM**。Move 折叠和空 block 消除改的是 CPS IR，VM 一行没动。
3. **Windows subprocess 是坑**。Python subprocess 的 cwd + encoding 问题浪费了大量调试时间。
4. **字节码对比更准**。Kaubo vs Python bytecode 逐行对比比只看 benchmark 数字更能定位冗余。
