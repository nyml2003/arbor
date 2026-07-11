# Punctum

Punctum 是仓库内复用的离散网格 UI 基础。Poke Game 和 TUI AI Chater 是最终消费者。Punctum 不面向 crates.io 发布。

应用把状态绘制到二维网格表面。Terminal 和 GPU adapter 分别提交该表面。平台 IO、产品状态和业务规则不进入共享核心。

## 当前状态

- `punctum-grid` 已实现 geometry、`Surface<T>`、clip、blit、diff 和 `Patch<T>`。
- `punctum-input` 已实现规范化键盘事件和已提交 Unicode 文本事件。
- 两个纯逻辑 crate 均按 TDD 实现，line、function 和 region coverage 为 100%。
- [`apps/tetris`](../tetris/README.md) 已作为独立项目实现完整 headless 规则、Punctum surface 绘制和 Terminal 入口。
- `punctum-terminal` 已实现单槽 `TerminalCell`、patch planner、Crossterm 键盘规范化、presenter 和 raw-mode session。纯模块 coverage 为 100%，IO 集中在 `runtime.rs`。
- `punctum-gpu` 仍是 `S0` 空壳。
- 当前先在 Windows 11、Windows Terminal 和本机 GPU 上跑通，不建设 CI。

## 运行俄罗斯方块

```powershell
cargo run --manifest-path apps/tetris/Cargo.toml --example terminal --locked
```

方向键移动和旋转，空格直接落底，`R` 重新开始，`Esc` 或 `Q` 退出。项目边界和完整按键见 [`apps/tetris/README.md`](../tetris/README.md)。

## 下一步

1. 实现 GPU adapter，并让同一 Tetris core 在本地 GPU 窗口运行。
2. 为 TUI AI Chater 补齐 Terminal 的 Unicode width、continuation 和 cursor 行为。
3. 两个 adapter 本地验证通过后，再接入 Poke Game 和 TUI AI Chater。

Tetris 是 proof example。它的规则和状态不进入 Punctum 内核，也不能单独触发 widget、focus、layout 或 routing 抽取。

## 验证

```powershell
cargo test --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml
cargo clippy --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml -- -D warnings
cargo llvm-cov -p punctum-grid --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo llvm-cov -p punctum-input --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo llvm-cov -p punctum-terminal --all-targets --locked --manifest-path apps/punctum/Cargo.toml --ignore-filename-regex "runtime\.rs" --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
python packages/arbor-projects/run.py verify tetris
```

详细边界、wave 和门禁见[第一期架构计划](../../workspace/manage/punctum-ramus-architecture-plan.md)。技术决策记录见 [`peps/`](peps/README.md)。
