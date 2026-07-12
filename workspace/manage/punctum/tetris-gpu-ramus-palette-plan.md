# Tetris GPU Ramus Command Palette 执行计划

- 状态：Wave 1 已验证；下一节点为 `TP-G2 GPU Palette Overlay`
- 日期：2026-07-12
- 架构依据：[PEP 7](./pep-0007.md)
- 目标：在 Tetris GPU example 中完成一个 Ramus 驱动、类似 fzf 的键盘命令面板

## 当前状态

| Node | Status | Evidence |
| --- | --- | --- |
| `TP-R1 Ramus Palette Core` | Completed | 7 个 contract tests、Tetris all-target test/Clippy/fmt 通过 |
| `TP-G1 Composable WGPU Frame` | Completed | Punctum workspace all-target test/Clippy/fmt 通过；本地 GPU smoke 仍为 ignored |
| `TP-I1 Winit Committed Text` | Completed | 13 个 punctum-wgpu input tests 通过，覆盖 None、ASCII、CJK、多 code point 和空文本 |
| `TP-G2 GPU Palette Overlay` | Ready | 等待派发 |
| `TP-H1 Host Integration` | Pending | 等待 `TP-G2` |
| `TP-B1 Proof Barrier` | Pending | 等待 `TP-H1` |

Wave 1 的实现当前尚未提交。`present_plan_with_overlay` 向 closure 暴露完整 `wgpu::Queue`，因此“一帧一次 submit”目前是调用合同，不是类型系统保证。`TP-G2` 禁止调用 `queue.submit`；只有 `GpuRuntime` 可以提交并 present 当前 surface frame。

## 纵向验收

最终程序必须满足以下场景：

1. 玩家运行 Tetris GPU example。
2. 玩家按 `Ctrl+P` 打开命令面板。
3. 游戏 tick 暂停，不发生关闭面板后的追赶 tick。
4. 玩家输入查询文本。面板只显示当前 principal 可以 discover/complete 的 Ramus 命令。
5. 玩家使用上下方向键移动选中项。
6. 玩家按 Enter。选中的 invocation 经过 Ramus parse、seal 和 runtime execute。
7. Ramus Provider 产生现有 `TetrisCommand`。Host 使用现有 `transition` 修改游戏状态。
8. 成功后面板关闭。失败时面板保留并显示结构化 diagnostic。
9. 棋盘和面板在同一个 surface frame 中绘制，只 acquire、submit 和 present 一次。
10. resize 后面板仍在窗口内，文字和候选项不重叠。

第一版只执行无参数命令：

```text
/tetris/piece left
/tetris/piece right
/tetris/piece rotate
/tetris/piece soft-drop
/tetris/piece hard-drop
/tetris/game restart
```

第一版不是完整 shell。它不实现参数编辑、光标移动、选区、剪贴板、IME composition、历史记录或多行命令。

## 边界

```text
winit Key/Text Event
        |
        v
Gameplay | CommandPalette
              |
              v
  capability-filtered candidates
              |
          selected invocation
              |
              v
parse -> seal -> Runtime -> Tetris Provider
                              |
                              v
                        TetrisCommand
                              |
                              v
                          transition

Tetris board plan + palette overlay
              |
              v
       one WGPU frame/present
```

- `ramus-core` 负责 catalog、authorization、discover、complete、parse、seal 和 execute。
- 模糊匹配属于 Tetris command palette，不修改 Ramus completion 语义。
- Tetris Provider 不保存第二份 `TetrisState`。它只把调用写入 host-owned command queue。
- 直接键盘输入和 Ramus 都产生同一个 `TetrisCommand`。
- `PaletteState` 拥有 query、候选项、selected index 和 diagnostic。
- 第一版使用 `Gameplay | Palette` 顶层模式，不建立公共 Focus Graph。
- GUI 文字、矩形和 grid 是不同 primitive。禁止把命令文字编码成 `GpuCell`。
- 新 GUI 类型先归 Tetris GPU example 所有，不进入 `punctum-ui`。
- `ramus-core` 现有 API 足以完成第一版。发现缺口时停止并提出 change request，不能顺手扩张 Ramus。

## 非目标

- Terminal command palette。
- 通用 `Fzf`、`CommandPalette` 或 `TextInput` widget。
- 公共 Punctum Focus service。
- Ramus 参数表单或 schema-aware value editor。
- CJK shaping 的最终公共合同。
- 图片、动画、滚动容器和离屏合成。
- 修改 Tetris 规则、方块状态机或 Terminal view。

## 任务 DAG

```text
Wave 1

TP-R1 Ramus Palette Core --------+
                                  |
TP-G1 Composable WGPU Frame ------+--> TP-G2 GPU Palette Overlay
                                  |             |
TP-I1 Winit Committed Text -------+-------------+
                                                |
                                                v
                                     TP-H1 Host Integration
                                                |
                                                v
                                       TP-B1 Proof Barrier
```

`TP-R1`、`TP-G1` 和 `TP-I1` 可以并行。它们没有重叠写入文件。

`TP-G2` 必须等待三个 Wave 1 节点完成。原因是它需要接收 palette model、frame API 和最终文本事件合同，并接管 Tetris manifest/lockfile。

`TP-H1` 是唯一负责修改 `apps/tetris/examples/gpu/main.rs` 的节点。

## 所有权规则

1. 同一时刻一个文件只有一个 owner。
2. Agent 不得回退工作区已有修改。
3. 需要扩大写入范围时停止，提交 change request。
4. 上游节点完成后，下游可以接管其文件做窄集成修正。
5. `apps/tetris/src/lib.rs`、`apps/tetris/tests/tetris_contract.rs` 和两个现有 `view.rs` 默认只读。
6. `packages/ramus/**` 默认只读。
7. 前置节点不得修改 GPU host `main.rs`。
8. 不因任务拆分创建新 crate、Service Locator 或动态插件注册表。

## `TP-R1`：Ramus Palette Core

### 结果

建立 Tetris-owned 的 Ramus 注册、授权、执行 adapter 和纯 palette state model。它可以在不启动窗口和 GPU 的情况下完成：授权候选发现、模糊过滤、选择和 invocation 执行。

### 行为

1. `ramus-core` 只作为 Tetris dev-dependency 接入，不能进入 Tetris core 的正常依赖路径。
2. 注册六个无参数 Invoke method。
3. 创建 `local-player` principal，只授予对应路径的 Discover、Complete 和 Invoke。
4. 额外注册一个未授权 developer method，用测试证明它不会出现在候选列表中。
5. Provider 把调用映射为现有 `TetrisCommand`，写入共享的有序 command queue。
6. Provider 不调用 `transition`，不保存 `TetrisState`，不读取 GPU 或键盘类型。
7. `PaletteState` 至少表达 closed/open、query、filtered items、selected index 和 diagnostic。
8. Palette 接收语义 intent，不接收 winit event：open、close、insert text、backspace、next、previous、execute。
9. 模糊匹配使用独立、维护中的 matcher crate。不要修改 `ramus-core::Compiler::complete`，也不要手写复杂评分器。
10. 排名相同时使用 invocation 字典序，保证 fixture 稳定。
11. query 改变后 selected index 必须保持有效。没有候选时 execute 返回明确 outcome。
12. 选中项执行必须走 `parse_with_limits -> Compiler::seal_with_limits -> Runtime::execute`。

### 写入范围

- `apps/tetris/Cargo.toml`
- `apps/tetris/Cargo.lock`
- `apps/tetris/examples/gpu/ramus_palette.rs`
- `apps/tetris/examples/ramus_palette_contract.rs`

不得修改 GPU `main.rs`、两个 `view.rs`、Tetris core、Punctum 或 Ramus。

### 验证

- 六个授权命令按稳定顺序 discover/complete。
- 未授权 developer method 不可见、不可执行。
- 模糊查询、空查询、零结果和稳定 tie-break。
- next/previous 在首尾循环或 clamp 的规则必须明确并测试。
- execute 产生且只产生一个正确的 `TetrisCommand`。
- parse、seal、provider failure 分别产生结构化 diagnostic。
- Tetris all-target tests、Clippy 和 fmt。

### Agent Prompt

```text
你叫 TetrisRamusPaletteCore。

执行 TP-R1。阅读 workspace/manage/punctum/tetris-gpu-ramus-palette-plan.md、PEP 7、Tetris core 和 ramus-core README/API。

在 Tetris GPU example 边界内建立 Ramus adapter 和纯 PaletteState。注册计划指定的六个无参数命令。local-player 只能发现、补全和调用授权命令。Provider 只把调用映射为 TetrisCommand 并写入 host-owned command queue，不能保存第二份 TetrisState，也不能调用 GPU。

Palette 接收语义 intent，不接收 winit event。模糊匹配留在 Tetris，不修改 ramus-core completion。选中 invocation 必须经过 parse_with_limits、seal_with_limits 和 Runtime::execute。

只修改任务列出的四个路径。不要修改 main.rs、view.rs、Tetris core、Punctum、Ramus 或管理文档。先写合同测试，再实现最小代码。运行 Tetris all-target test、Clippy 和 fmt。完成后报告 API、验证和下游集成方式，然后停止。
```

## `TP-G1`：Composable WGPU Frame

### 结果

让现有 `GpuRuntime` 在一次 surface frame 中先绘制 grid，再允许一个显式的 consumer-owned overlay encoder 追加命令，最后只 submit/present 一次。

### 行为

1. 保持现有 `present_surface`、`present_patch` 和 `present_plan` 行为兼容。
2. 新增 `GpuRuntime` public method，使用闭包接收具体的 wgpu `Device`、`Queue`、`TextureView`、`CommandEncoder`、format 和 surface size。不要新增需要从 `lib.rs` 重新导出的命名 frame context 类型。
3. Runtime 仍是唯一 surface acquire、configure、submit 和 present owner。
4. Consumer 可以获得编码 overlay 所需的 `Device`、`Queue`、surface `TextureView`、`CommandEncoder`、format 和 surface size，但不能取得 surface 所有权。
5. 绘制顺序固定为 clear、grid、overlay。
6. 没有成功 acquire frame 时不调用 overlay encoder。
7. 一次调用最多创建一个 surface frame、一个 command encoder、一次 queue submit 和一次 present。
8. Grid diff/upload、resize 和 `PresentOutcome` 语义保持现状。
9. 不增加文字、矩形、Focus、Palette 或 Tetris 类型。

### 写入范围

- `apps/punctum/crates/punctum-wgpu/src/runtime.rs`

不得修改 `lib.rs`、manifest、lockfile、`punctum-gpu`、Tetris 或管理文档。如果实现确实需要新的 public re-export，停止并提出 change request，不能与 `TP-I1` 同时写 `lib.rs`。

### 验证

- 现有 punctum-wgpu tests 全部通过。
- 新 API 的 grid-only 路径与 `present_plan` 等价。
- surface skip/lost/outdated 时 overlay encoder 调用规则有测试或明确的纯状态 oracle。
- grid 与 overlay 顺序固定。
- Punctum workspace test、Clippy 和 fmt。
- 现有 ignored local GPU smoke 继续可编译。

### Agent Prompt

```text
你叫 ComposableWgpuFrame。

执行 TP-G1。阅读任务计划、PEP 7、punctum-wgpu runtime 和现有 submission plan。

在不改变现有 grid API 行为的前提下，给 GpuRuntime 增加最小的 wgpu-specific closure composition API。闭包直接接收具体 wgpu 参数，不新增需要从 lib.rs 导出的命名 context。Runtime 必须继续独占 surface acquire/configure/submit/present；consumer 只能在同一个 encoder/view 上追加 overlay。顺序固定为 clear、grid、overlay，一帧只 submit/present 一次。

不要建立通用 Layer trait、动态 registry、UI command enum，也不要加入 Text、Rect、Palette 或 Tetris 类型。只修改 runtime.rs。先补最窄合同测试，再实现。运行 Punctum workspace test、Clippy 和 fmt。完成后报告新 API 和 surface outcome 语义，然后停止。
```

## `TP-I1`：Winit Committed Text

### 结果

补齐 winit 到 `punctum_input::TextEvent` 的 committed text adapter，让 Host 不必把 `LogicalKey::Character` 假装成文本输入。

### 行为

1. 新增与 `WinitKeyEventSnapshot` 同级的最小 text snapshot/normalizer。
2. `None` 表示没有 committed text。
3. 非空 committed text 原样进入 `TextEvent`，包括多 code point 字符串。
4. 空文本遵守现有 `TextEventError::EmptyText` 合同。
5. adapter 不处理 Backspace、Enter、方向键或快捷键。
6. adapter 不实现 IME preedit/composition。`Ime::Commit` 是否接入留给后续任务。
7. Host 负责在 Ctrl/Alt/Super 快捷键期间不把文本插入 query。
8. 不从 physical/logical key 猜测文本。

### 写入范围

- `apps/punctum/crates/punctum-wgpu/src/input.rs`
- `apps/punctum/crates/punctum-wgpu/src/lib.rs`
- `apps/punctum/crates/punctum-wgpu/tests/input_contract.rs`

不得修改 runtime、manifest、lockfile、Tetris 或管理文档。

### 验证

- None、ASCII、CJK、多 code point、空字符串。
- KeyEvent 与 TextEvent 保持两个独立事件合同。
- Punctum workspace test、Clippy 和 fmt。

### Agent Prompt

```text
你叫 WinitCommittedText。

执行 TP-I1。阅读任务计划、punctum-input 的 TextEvent 合同和 punctum-wgpu 现有 input adapter。

增加最小 committed text snapshot/normalizer。不要从 LogicalKey 猜测文本，不处理编辑键，不实现 IME composition。None 表示没有文本；非空字符串原样进入 TextEvent；空字符串遵守现有错误合同。

只修改 input.rs、lib.rs 和 input_contract.rs。不要修改 runtime、manifest、Tetris 或管理文档。先写合同测试，再实现。运行 Punctum workspace test、Clippy 和 fmt。完成后报告 Host 应如何调用，然后停止。
```

## `TP-G2`：GPU Palette Overlay

### 依赖

等待 `TP-R1`、`TP-G1` 和 `TP-I1` 全部完成。

### 结果

建立 Tetris-owned 的 GPU command palette overlay。它把 `PaletteState` 投影为整数像素布局和真实文字/矩形绘制，并通过 `TP-G1` 的 frame API 追加到 grid 后面。

### 行为

1. Overlay 使用 Tetris-owned `UiPx/UiRect` 或等价整数几何，不使用 `GridRect` 表达 GUI bounds。
2. 至少绘制 panel background、query、候选列表、selected row 和 diagnostic。
3. 文字使用与当前 wgpu 版本兼容的成熟 shaping/rasterization renderer。不得手写 Unicode shaping，也不得把 glyph 写成 `GpuCell`。
4. Renderer 可以在 Tetris 内部把 Text/Rect 降低为 wgpu 命令，但不能新增 Punctum 公共 GUI tree。
5. selected row 的外观变化不能改变行高或 panel 外部 bounds。
6. 面板默认位于窗口下部或中央，最多显示固定数量候选；不足空间时裁剪低优先级行。
7. 支持至少 `480x704`、`320x480` 和 `960x1408` 三个 framebuffer fixture。
8. 所有 bounds 和 clip 保持在 surface 内。零尺寸窗口返回空 overlay plan。
9. Overlay renderer 不 acquire、submit 或 present。

### 写入范围

- `apps/tetris/Cargo.toml`
- `apps/tetris/Cargo.lock`
- `apps/tetris/examples/gpu/palette_overlay.rs`
- `apps/tetris/examples/palette_overlay_contract.rs`
- `apps/tetris/examples/gpu/assets/**`，仅在字体或 renderer 确实需要静态 asset 时

不得修改 GPU `main.rs`、两个现有 `view.rs`、Tetris core、Punctum、Ramus 或管理文档。

### 验证

- 三个 viewport 的精确/性质 layout fixture。
- query、零候选、长 invocation、selected row 和 diagnostic 不重叠。
- 所有 rect/clip 位于 surface 内。
- Text 和 Rect 是不同 plan primitive。
- renderer 可以接入上游 frame context，但不会自行 present。
- Tetris all-target test、Clippy 和 fmt。
- 可用时增加 ignored local GPU smoke；不能用 mock 声称验证真实 surface。

### Agent Prompt

```text
你叫 TetrisGpuPaletteOverlay。

执行 TP-G2。确认 TP-R1、TP-G1、TP-I1 已完成。阅读它们的 API、任务计划和 PEP 7。

在 Tetris GPU example 内实现 command palette overlay。使用整数 GUI 几何，绘制背景、query、候选、selected row 和 diagnostic。文字使用与当前 wgpu 版本兼容的成熟 renderer；不能手写 shaping，不能把文字塞进 GpuCell。通过上游 composition API 追加到 grid 后面，renderer 不得 acquire/submit/present。

只修改任务列出的 Tetris manifest、lockfile、新 overlay/harness 文件和必要静态 asset。不要修改 main.rs、view.rs、Tetris core、Punctum、Ramus 或管理文档。先写 layout/plan fixture，再实现 renderer。运行 Tetris all-target test、Clippy 和 fmt。完成后报告 Host 集成 API，然后停止。
```

## `TP-H1`：Host Integration

### 依赖

等待 `TP-R1`、`TP-G1`、`TP-I1` 和 `TP-G2` 完成。

### 结果

把 Ramus palette、committed text、GPU overlay 和 Tetris event loop 接成最终可运行 vertical slice。

### 行为

1. Host 明确持有 `Gameplay | Palette` 模式。
2. `Ctrl+P` 在两个模式间切换。该按键不能把字符 `p` 插入 query。
3. Palette 模式捕获 Up、Down、Enter、Escape、Backspace 和 committed text。
4. Palette 模式下不把方向键、空格或文本传给 Tetris gameplay mapping。
5. Enter 执行 selected invocation，立即 drain Provider command queue，并通过同一 `transition` 应用命令。
6. 成功后关闭面板。失败时保留 query/selection 并显示 diagnostic。
7. 面板打开时暂停 tick。关闭时把 `next_tick` 重置为 `now + TICK_INTERVAL`，不能追赶暂停期间的 tick。
8. 打开、输入、移动选择、执行、失败、关闭和 resize 都 request redraw。
9. Redraw 使用一份 grid plan 和一份 overlay plan，经同一 runtime frame 提交。
10. 保持现有 surface lost、reconfigure、minimized、timeout 和 occluded 处理。
11. 直接 gameplay key 与 Ramus invocation 对同一命令产生相同最终 `TetrisState`。

### 写入范围

- `apps/tetris/examples/gpu/main.rs`
- `apps/tetris/examples/gpu/ramus_palette.rs`，仅允许窄集成修正
- `apps/tetris/examples/gpu/palette_overlay.rs`，仅允许窄集成修正
- `apps/tetris/Cargo.toml` 和 `apps/tetris/Cargo.lock`，仅允许最终集成修正

默认不得修改 `view.rs`、Tetris core、Terminal、Punctum、Ramus 或管理文档。确实需要修改上游 API 时停止并提出 change request。

### 验证

- Host routing fixture：Ctrl+P、文本、上下、Enter、Escape、Backspace。
- Palette 打开时 gameplay command 和 tick 不执行。
- 关闭后没有 tick catch-up。
- Ramus 与直接 keyboard command 的最终状态相同。
- 成功关闭、失败保留。
- resize 不改变 Tetris 或 Palette semantic state。
- Tetris all-target test、Clippy、fmt 和 GPU example build。

### Agent Prompt

```text
你叫 TetrisGpuPaletteHost。

执行 TP-H1。确认四个上游任务完成。阅读任务计划、上游 API 和当前 GPU main.rs。

只在 Host 中组合 Gameplay/Palette 模式。Ctrl+P 打开或关闭；Palette 捕获文本和导航；Enter 走 Ramus parse/seal/runtime/provider，再把 queue 中的 TetrisCommand 通过现有 transition 应用。直接键盘与 Ramus 必须进入同一 transition 路径。

面板打开时暂停 tick，关闭时重置 next_tick，不能追赶。棋盘和 overlay 必须共用一次 frame acquire/submit/present。保持现有 surface outcome 行为。

只修改任务列出的 main.rs 和必要的窄集成文件。默认不要修改 view.rs、Tetris core、Terminal、Punctum、Ramus 或管理文档。先写 routing/暂停/等价性测试，再集成。运行 Tetris all-target test、Clippy、fmt 和 GPU example build。完成后报告行为和残余风险，然后停止。
```

## `TP-B1`：Proof Barrier

### 依赖

等待 `TP-H1` 完成。

### 结果

接收最终 manifest/lockfile 和验证证据，完成本地 smoke，记录这次 probe 证明和没有证明的边界。

### 行为

1. 检查依赖仍是无环的：Tetris core 不依赖 Ramus、wgpu 或 GUI renderer。
2. 检查 `ramus-core` 没有为 palette 修改 completion 或 authorization 语义。
3. 检查普通文字没有进入 `GpuCell`。
4. 检查 Runtime 仍唯一拥有 surface lifecycle。
5. 检查没有新增公共 Focus、Fzf、CommandPalette、TextInput 或 universal Layer 抽象。
6. 运行完整聚焦验证。
7. 本地运行 GPU example，执行至少 rotate、hard-drop 和 restart 三个 Ramus 命令。
8. 记录未授权命令不可见、面板暂停、resize 和一次 present 的证据。
9. 只把真实重复和稳定合同列为后续抽取候选，不在 barrier 实现抽取。

### 写入范围

- `apps/tetris/README.md`
- `workspace/manage/punctum/tetris-gpu-ramus-palette-plan.md`
- `workspace/manage/punctum/pep-0007.md`，只更新实现证据和后续问题

不得修改生产 Rust、manifest、lockfile、Punctum、Ramus 或 Tetris core。

### 验证

```powershell
cargo fmt --all --manifest-path apps/punctum/Cargo.toml -- --check
cargo clippy --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml -- -D warnings
cargo test --workspace --all-targets --locked --manifest-path apps/punctum/Cargo.toml

cargo fmt --all --manifest-path apps/tetris/Cargo.toml -- --check
cargo clippy --all-targets --locked --manifest-path apps/tetris/Cargo.toml -- -D warnings
cargo test --all-targets --locked --manifest-path apps/tetris/Cargo.toml
cargo build --locked --manifest-path apps/tetris/Cargo.toml --example gpu
python packages/arbor-projects/run.py verify tetris
```

### Agent Prompt

```text
你叫 TetrisRamusPaletteBarrier。

执行 TP-B1。确认 TP-H1 完成。阅读任务计划、最终 diff、上游验证和 PEP 7。

不要实现新功能。检查 core 依赖、Ramus capability、GpuCell 边界、surface ownership 和是否出现提前公共抽象。运行计划中的全部聚焦验证，并本地 smoke rotate、hard-drop、restart、未授权隐藏、暂停和 resize。

只修改 Tetris README、本计划状态和 PEP 7 的实现证据。不得修改 Rust、manifest 或 lockfile。记录已经证明、尚未证明和下一步抽取候选，然后停止。
```

## 完成定义

整个 DAG 只有在以下条件同时满足时完成：

- GPU Tetris 可通过键盘打开和操作 Ramus command palette。
- 候选来自 capability-filtered Ramus view。
- Ramus invocation 和直接键盘命令进入同一 Tetris transition。
- 面板打开时游戏暂停，关闭时不追赶 tick。
- 文字和矩形不进入 `GpuCell`。
- 棋盘和 overlay 每次 redraw 只 present 一次。
- 三个 viewport fixture 和本地 GPU smoke 通过。
- Punctum、Tetris、Ramus 的聚焦 test/Clippy/fmt 通过。
- 没有为了本次 probe 发布通用 GUI、Focus 或 command palette API。

## 后续门禁

本次 probe 完成后，只允许提出候选，不立即抽取：

- wgpu frame composition API 是否已成为稳定平台边界。
- committed text adapter 是否可以直接保留。
- PaletteState 是否应由未来 Game console 复用。
- Ramus capability-filtered candidate projection 是否存在第二个消费者。
- Text/Rect overlay plan 是否应留在产品、进入 GUI target，或由成熟 renderer 直接拥有。

只有 Game 的真实 in-game Ramus console 出现相同语义和 oracle 后，才能决定公共抽取。
