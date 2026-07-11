# Tetris

这是 `apps/tetris` 下的独立俄罗斯方块项目，也是 Punctum 的首个可玩消费者。

游戏状态机、方块序列、碰撞、锁定、消行、绘制投影和命令映射位于 `src/lib.rs`。它们只依赖 `punctum-grid` 和 `punctum-input`，不读取时钟、随机源或终端。

Terminal 入口位于 `examples/terminal/`。`view.rs` 使用纯逻辑 `punctum-terminal`，`main.rs` 使用平台侧 `punctum-crossterm` 处理事件循环、tick 和 IO。项目拥有自己的 `Cargo.toml` 和 `Cargo.lock`，不属于 Punctum workspace。

## 运行

在仓库根目录执行：

```powershell
cargo run --manifest-path apps/tetris/Cargo.toml --example terminal --locked
```

按键：

- 左右方向键：移动。
- 上方向键：顺时针旋转。
- 下方向键：加速下落。
- 空格：直接落底。
- `R`：重新开始。
- `Esc` 或 `Q`：退出。

## 验证

```powershell
cargo test --manifest-path apps/tetris/Cargo.toml --all-targets --locked
python packages/arbor-projects/run.py verify tetris
```
