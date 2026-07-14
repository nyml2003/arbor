# Gen3 架构门禁

状态：生效

更新日期：2026-07-14

运行命令：

```powershell
python scripts/test_architecture.py
```

脚本只使用 Python 3 标准库。规则使用 `dataclass` 和类型标注定义。

门禁使用 `cargo metadata` 检查平台依赖 allowlist。门禁也扫描纯 crate 的源码禁止项。当前 GPU、字体和帧提交只允许出现在 `game-native-target`。`game-host` 与 `map-editor` 只保留 `winit` 平台事件依赖。

host 只允许保留一个平台唤醒 deadline：`next_wakeup`。

以下旧字段已经删除，门禁禁止恢复：`next_playback`、`next_sprite_frame`、`next_world_tick`、`turn_hold_ends`、`run_stop_ends`。表现计时统一由 `game-ui::PresentationState` 使用逻辑 `Duration` 推进。

经审查的其他副作用位置：

- `game-host/src/lib.rs`：`OnceLock`、`AtomicU64`、`SystemTime`，阶段 2 删除。
- `game-ui/src/presentation.rs`：使用显式传入的逻辑 `Duration`，不读取系统时间。
- `game-host/src/map.rs` 与 `map-editor/src/assets.rs`：文件和素材 adapter，允许保留在 host。
- `map-editor/src/main.rs`：文件保存和窗口事件，允许保留在 editor host。
- `game-data-import`：解析与文件 adapter 尚未拆分，阶段 5 处理。
- `battle-ramus-adapter`：`Arc<Mutex<_>>` action queue，仅允许在该 adapter 内部存在。

纯 crate 扫描只是快速反馈。最终证据仍包括依赖图、公开 API、单元测试、Clippy 和真实 GPU smoke。
