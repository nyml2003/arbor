# Punctum

Punctum 是仓库内复用的离散网格 UI 基础。Poke Game 和 TUI AI Chater 是最终消费者。Punctum 不面向 crates.io 发布。

应用把状态绘制到二维网格表面。Terminal 和 GPU adapter 分别提交该表面。平台 IO、产品状态和业务规则不进入共享核心。

## 当前状态

- `punctum-grid` 已实现 geometry、`Surface<T>`、clip、blit、diff 和 `Patch<T>`。
- `punctum-input` 已实现规范化键盘事件和已提交 Unicode 文本事件。
- 各 pure module 均按 TDD 实现，line、function 和 region coverage 为 100%。
- [`apps/tetris`](../tetris/README.md) 已作为独立项目实现完整 headless 规则、Punctum surface 绘制和 Terminal 入口。
- `punctum-terminal` 保留 `TerminalCell`、Unicode 文本、resize 和 patch planning，不依赖 Crossterm。
- `punctum-crossterm` 提供 Crossterm 输入转换、presenter、raw-mode session 和终端 IO。
- `punctum-gpu` 保留 atlas、viewport、cell、submission planning、instance encoding 和 uniform encoding，不依赖 winit、wgpu 或 `punctum-input`。
- `punctum-wgpu` 提供 winit 输入转换、wgpu runtime、shader、pipeline、surface 和 device 操作。
- `punctum-terminal` 和 `punctum-gpu` 的 line、function 和 region coverage 为 100%。平台 crate 使用合同测试、Clippy 和 headless smoke，不设置 coverage 百分比。
- 当前先在 Windows 11、Windows Terminal 和本机 GPU 上跑通，不建设 CI。

## 运行俄罗斯方块

```powershell
cargo run --manifest-path apps/tetris/Cargo.toml --example terminal --locked
```

方向键移动和旋转，空格直接落底，`R` 重新开始，`Esc` 或 `Q` 退出。项目边界和完整按键见 [`apps/tetris/README.md`](../tetris/README.md)。

## 下一步

1. 在 `B2` 接通 Tetris GPU 入口，并完成 Terminal/GPU 双后端本地验收。
2. 双后端验收通过后，建立 provisional UI foundation。
3. Game 使用并收窄 provisional API 后，再评估稳定公共合同。TUI AI Chater 当前暂停。

Tetris 是 proof example。它的规则和状态不进入 Punctum 内核，也不能单独触发 widget、focus、layout 或 routing 抽取。

## 验证

```powershell
cargo test --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml
cargo clippy --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml -- -D warnings
cargo llvm-cov -p punctum-grid --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo llvm-cov -p punctum-input --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo llvm-cov -p punctum-terminal --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo llvm-cov -p punctum-gpu --all-targets --locked --manifest-path apps/punctum/Cargo.toml --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
cargo test -p punctum-crossterm --all-targets --locked --manifest-path apps/punctum/Cargo.toml
cargo test -p punctum-wgpu --all-targets --locked --manifest-path apps/punctum/Cargo.toml
cargo test -p punctum-wgpu --locked --manifest-path apps/punctum/Cargo.toml runtime::tests::headless_pipeline_smoke -- --ignored --exact --nocapture
python packages/arbor-projects/run.py verify tetris
```

详细边界、wave 和门禁见[第一期架构计划](../../workspace/manage/punctum-ramus-architecture-plan.md)。技术决策记录见 [`peps/`](peps/README.md)。
