# Thorn

Thorn 是 Arbor 里的 Rust UI runtime 实验。

Thorn 正在按新概念重建。它不是 widget 库，也不是浏览器 DOM 的复刻。它的目标是把 `App + State + Action + Component DSL` 转成 `Host Tree`、`Layout Tree`、`Paint Primitive` 和后端输出。

TUI 是第一个后端。Native GUI、Web 和 headless test 后端必须保留可能性。

## 核心管线

```text
App / State / Signals
  -> Component
  -> Element Tree
  -> Host Tree
  -> Layout Tree
  -> Paint Primitive
  -> Backend Output
```

对于 TUI 后端，后端输出继续下降为：

```text
Paint Primitive
  -> Cell Grid
  -> Dirty Patch
  -> Terminal Backend
```

## 术语

- `Component`：作者编写的组合单位。它读取 props、state 或 signals，返回 elements。
- `Element`：组件返回的声明式 UI 节点。
- `Host Tree`：后端无关的规范 UI 对象模型。它承担类似 DOM 的职责，但不是浏览器 DOM。
- `Layout Tree`：完成测量和定位后的树。
- `Paint Primitive`：面向 renderer 的绘制命令，例如 fill、text run、border、cursor、clip。
- `Backend Output`：后端自己的输出形态，例如终端 cell、原生 display list、Web DOM 或测试快照。
- `App`：拥有 state、update、view 和 runtime 配置的应用结构体。

## THEP

架构决策放在 [`docs/THEPs`](docs/THEPs/README.md)。

## Demo

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter
```

当前第一个可见 demo 使用标准输入。输入 `+`、`-` 或 `q` 后按 Enter。

## 当前状态

状态：第一版 MVP。

已具备：

- `thorn-core`：纯类型、Element 到 Host Tree、Layout、Paint、Screen。
- `thorn-runtime`：共享 App runtime、KeyIntent 映射、Action dispatch、render scheduling。
- `thorn-headless`：可测试 adapter 和 snapshot。
- `thorn-terminal`：最小 stdio terminal adapter，共用 `thorn-runtime`。
- `thorn`：public facade 和 Counter example。

旧代码和旧 THEP 已删除。新实现必须服从当前 THEP 文档，不继承旧 Thorn MVP 或 `arbor-tui` 的 widget 协议。
