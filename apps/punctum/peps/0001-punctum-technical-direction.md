# PEP 0001：Punctum 技术方向与共享边界

- Status: Provisional
- Type: Standards Track
- Created: 2026-07-10

## 摘要

Punctum 首先定义一种可验证的离散二维表面与增量提交协议。TUI 和点阵 2D 游戏共享整数网格、裁剪、表面写入和帧差分，不共享完整的输入、文本、合成或游戏循环。

项目使用 Rust stable。首批原型使用 Crossterm 构建终端后端，使用 winit + wgpu 构建图形后端。Ratatui、Bevy、Taffy 和完整文本栈不进入核心依赖。

这份决策保持 `Provisional`，直到三个原型实验通过。

## 问题

终端 UI 和点阵游戏都能被画成格子，但它们并不拥有相同的能力：

- 终端使用 Unicode 字形、有限样式和 ANSI 更新。
- 游戏窗口使用纹理图集、透明混合、连续时间和 GPU 批处理。
- 终端通常按事件重绘，游戏通常持续运行并区分更新与渲染。

如果 Punctum 把这些差异压进一个万能渲染接口，各后端会出现大量空实现和假语义。真正稳定的交集是离散网格数据，而不是所有视觉能力。

## 目标

- 同一个场景函数可以产出供终端和图形窗口消费的网格帧。
- 核心可以独立测试，不依赖终端、窗口、GPU 或操作系统。
- 后端只解释最终单元和帧变更，不反向控制应用状态。
- 首版保持小而可证伪，实验失败时可以调整共享边界。

## 非目标

- 不在首版实现完整组件框架。
- 不构建 ECS、物理、音频、资源管线或场景编辑器。
- 不统一终端键鼠、游戏手柄和连续轴输入。
- 不要求终端支持 alpha、旋转、缩放或 shader。
- 不以像素级排版或富文本编辑器为目标。
- 不追求 `no_std`。

## 核心判断

Punctum 的最小核心是离散表面协议，不是统一渲染 API。

```text
应用状态 ── paint ──> next Surface<Cell>
                           │
previous Surface<Cell> ─ diff ─> Patch<Cell>
                           │
                 ┌─────────┴─────────┐
          terminal presenter    gfx presenter
           字形/样式 -> ANSI     图集/颜色 -> GPU
```

`Surface<T>` 保持泛型。核心不提前定义万能 `Cell`。这样终端可以使用字形资源键，游戏可以使用图块资源键，测试可以使用简单整数。

建议的基础类型：

```rust
pub struct GridSize {
    pub cols: u32,
    pub rows: u32,
}

pub struct GridPos {
    pub col: i32,
    pub row: i32,
}

pub struct GridRect {
    pub origin: GridPos,
    pub size: GridSize,
}

pub struct Surface<T> {
    size: GridSize,
    cells: Vec<T>,
}

pub struct Patch<T> {
    pub size: GridSize,
    pub rows: Vec<RowPatch<T>>,
}
```

`Surface<T>` 本身不要求 `T: Copy + Eq`。只有 diff 操作按需要求 `T: Eq + Clone`。核心提供严格的 `get`、`set`、`fill`、`blit`、裁剪视图和 diff。越界错误必须结构化返回；允许裁剪的写入使用不同的显式 API。

Presenter 端口保持窄：

```rust
pub trait Presenter<C> {
    type Error;

    fn present(
        &mut self,
        size: GridSize,
        patch: &Patch<C>,
    ) -> Result<(), Self::Error>;
}
```

Presenter 不拥有组件状态、布局、输入或时钟。

## 语言与库选型

### Rust stable

选择 Rust，原因是 Punctum 的核心工作负载适合其数据模型：紧凑泛型网格、显式错误、可预测内存、无垃圾回收停顿，以及在同一语言内覆盖终端、原生窗口、GPU 和 WebAssembly 的可能性。

首版允许 `std`。现在引入 `no_std` 会扩大约束，却不能帮助验证共享模型。

### Crossterm

终端后端使用 Crossterm。它提供跨平台终端控制和输入能力。Punctum 自己维护表面、diff 和输出策略，不把 Ratatui 作为核心依赖。

Ratatui 仍然是重要参考。它采用 immediate-mode rendering，并把实际终端绘制交给 Backend。Punctum 采用相似的“应用每帧描述画面”思路，但核心协议要同时服务非终端后端。

### winit + wgpu

图形原型使用 winit 管理窗口和事件循环，使用 wgpu 提交跨平台 GPU 绘制。wgpu 当前覆盖 Vulkan、Metal、D3D12、OpenGL，以及 WebAssembly 上的 WebGPU/WebGL2 路径。

首个图形 presenter 只需要：

- 固定尺寸图集。
- 每个非空单元一个实例。
- 整数网格到屏幕矩形的映射。
- 最近邻采样。
- 窗口缩放时保持网格比例。

它暂时不需要通用 scene graph、材质系统或 shader 插件架构。

### 文本与布局后置

任意 Unicode 文本、字形 shaping 和宽字符规则进入可选 `punctum-text`。若图形后端以后需要高质量文本，可评估 cosmic-text + glyphon。glyphon 当前建立在 wgpu、cosmic-text 和 etagere 之上。

首版 UI 布局使用整数矩形和简单约束。Taffy 支持成熟的 Flexbox/Grid 布局，但它的连续尺寸模型不是验证离散布局语义的必要条件。只有原型证明自有布局不足时才引入。

### 候选矩阵

| 候选 | 适合复用的部分 | 不进入核心的原因 | 定位 |
| --- | --- | --- | --- |
| Crossterm | raw mode、alternate screen、输入、resize、光标和 ANSI 输出 | 只服务终端 | 首个外部依赖 |
| Ratatui | Buffer、Widget、diff 和 immediate-mode API 参考 | 公共 API 会被终端 cell 语义塑形 | 参考或未来互操作 |
| winit | 窗口生命周期、输入、resize、DPI | event loop 属于 host | 图形后端依赖 |
| wgpu | 跨平台 GPU 提交 | device、surface 和 pipeline 不能进入核心 | 图形后端依赖 |
| cosmic-text + glyphon | shaping、fallback、atlas 和 wgpu 文本 | 固定点阵 MVP 不需要完整文本栈 | 按需的 gfx 内部实现 |
| Bevy | ECS、资产和完整游戏运行时 | Punctum 会退化为 Bevy plugin | 未来可选集成 |
| macroquad | 快速 2D/WASM 原型 | 自带主循环和渲染模型 | 一次性实验工具 |
| egui | 调试面板和编辑工具 | 自带布局、交互和三角形绘制模型 | 开发工具集成 |

## 模块边界

建议按验证顺序创建 crate，而不是一次建全：

| Crate | 职责 | 首批依赖 |
| --- | --- | --- |
| `punctum-grid-core` | geometry、`Surface`、裁剪、diff、`Patch`、错误 | 标准库 |
| `punctum-terminal` | 终端生命周期、ANSI 提交、终端输入、能力降级 | Crossterm |
| `punctum-gfx` | 窗口、图集、GPU 实例和缩放 | winit、wgpu |
| `punctum-ui` | 网格布局、焦点、widget、事件分发 | 暂不创建 |
| `punctum-text` | grapheme、宽字符、换行和文本光栅化 | 暂不创建 |

应用层自己把平台事件映射为领域动作。核心不定义覆盖所有设备的 `InputEvent`。

## 共享与非共享边界

| 能力 | 是否共享 | 位置 |
| --- | --- | --- |
| 整数位置、尺寸、矩形 | 是 | `punctum-grid-core` |
| 泛型致密表面 | 是 | `punctum-grid-core` |
| 裁剪、平移、覆盖 | 是 | `punctum-grid-core` |
| 帧 diff 和 row spans | 是 | `punctum-grid-core` |
| Unicode 宽度和 continuation | 否 | `punctum-text` / `punctum-terminal` |
| 图集、纹理、alpha、shader | 否 | `punctum-gfx` |
| 键盘、鼠标、手柄、连续轴 | 否 | 各 host |
| event loop、fixed update、clock | 否 | 各 host |
| camera、世界坐标、物理、ECS | 否 | 游戏应用或集成层 |

透明层在共享核心中只表达“空”或“覆盖”。alpha 和 blend mode 属于图形后端。否则终端后端只能假装实现这些能力。

## 数据不变量

- `cells.len() == cols * rows`。构造时检查乘法和容量溢出。
- patch span 按行列排序、互不重叠，并完全位于目标尺寸内。
- `apply(previous, diff(previous, next)) == next`。
- resize 是显式变化。不同尺寸的 diff 产生完整替换 patch。
- 核心不导入 Crossterm、winit、wgpu 或操作系统 API。
- 正常失败通过 `Result` 返回，不使用 panic 控制流程。

## 被拒绝的方案

### 直接基于 Ratatui 扩展 GPU 后端

拒绝。Ratatui 的 cell 和终端语义会成为事实上的核心模型。GPU 后端能显示画面，但 Punctum 难以自然表达图集资源、游戏更新和不同的合成能力。

Ratatui 可以在未来作为互操作层，而不是 Punctum 的地基。

### 直接基于 Bevy 构建

拒绝作为核心。Bevy 适合完整游戏和 ECS 集成，但它会让窗口、调度、资产和渲染架构先于 Punctum 的网格协议确定。以后可以提供 Bevy plugin，让 Punctum 表面成为纹理或实例来源。

### 从第一天设计统一的万能 Cell

拒绝。Unicode grapheme、终端样式、纹理区域、动画帧和 blend mode 没有稳定的共同结构。首版以 `Surface<T>` 保持核心诚实，portable demo 只定义资源键形式的实验单元。

### 从第一天实现 retained component tree

拒绝。TUI 与游戏 ECS 的生命周期不同。先使用 immediate paint 验证场景和后端，之后再用独立 PEP 决定组件标识、局部状态、焦点和事件冒泡。

### 使用 Web 技术作为首个实现

暂不采用。Canvas/WebGPU 很适合展示和分发，但不能直接验证真实终端行为。Rust 核心以后仍可编译到 WebAssembly，Web presenter 不需要现在进入关键路径。

## 原型与接受条件

### E1：同一场景，两个后端

创建一个 40x24 场景，包含边框、标签、移动角色、前后景颜色和覆盖。场景只产出 `Surface<DemoCell>`，分别由终端和图形 presenter 显示。

接受条件：

- 两端逻辑坐标和遮挡结果一致。
- 核心不包含后端类型。
- 终端和窗口 resize 都能产生完整替换。
- 缺失资源产生明确诊断，不静默替换。

### E2：diff 基准

测试 80x24、160x90、320x180 三种尺寸，以及 1%、5%、20%、100% 更新比例。比较全帧扫描、显式 dirty rect 和整帧提交。

记录帧时间、分配次数、span 数量、终端输出字节和 GPU 上传单元数。全帧扫描先作为正确性基线。只有测量证明必要时，才把 dirty tracking 加进公共 API。

### E3：Unicode 与宽字符

在独立实验中覆盖 combining mark、CJK、emoji、截断、覆盖 continuation 和 resize。

接受条件：

- 不产生半个宽字符。
- 覆盖任一槽位时的行为确定且有测试。
- 终端最终光标位置稳定。
- 如果各终端无法保持一致，核心继续只认识单槽，文本层负责展开与降级。

## 风险

- **共享过度**：用泛型表面和窄 presenter 协议限制共享范围。
- **Unicode 行为依赖终端**：宽度策略留在终端层，并提供降级字形。
- **致密双缓冲在大地图上昂贵**：先测量，再决定 chunk 或 sparse surface。
- **资源映射不一致**：使用显式 theme/tileset manifest，缺失映射返回诊断。
- **GPU 需求污染核心**：核心坐标永久保持整数 cell；变换和像素仅存在于 gfx。
- **名称可用性**：发布前重新检查 `punctum-*` crate 名称和其他软件生态中的冲突。调研时的可用状态不构成长期保证。

## 当前版本基线

这不是依赖锁定，只是 2026-07-10 调研时的生态快照：

- wgpu 30.0.0
- winit 0.30.13
- Crossterm 0.29.0
- Ratatui 0.30.2
- glyphon 0.12.0
- cosmic-text 0.19.0
- Taffy 0.12.1
- Bevy 0.19.0
- macroquad 0.4.15
- egui 0.35.0

真正初始化 workspace 时应重新核对兼容矩阵，并锁定精确版本。

## 参考资料

- [Ratatui rendering concepts](https://ratatui.rs/concepts/rendering/)
- [Crossterm documentation](https://docs.rs/crossterm/latest/crossterm/)
- [wgpu documentation](https://docs.rs/wgpu/latest/wgpu/)
- [winit documentation](https://docs.rs/winit/latest/winit/)
- [glyphon documentation](https://docs.rs/glyphon/latest/glyphon/)
- [cosmic-text documentation](https://docs.rs/cosmic-text/latest/cosmic_text/)
- [Taffy documentation](https://docs.rs/taffy/latest/taffy/)
- [Bevy repository](https://github.com/bevyengine/bevy)
- [macroquad repository](https://github.com/not-fl3/macroquad)
- [egui repository](https://github.com/emilk/egui)
- [crates.io search for `punctum`](https://crates.io/search?q=punctum)
