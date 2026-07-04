---
id: TEP-0002
title: "渲染层——Cell 网格、diff 算法与终端适配器"
status: Review
created: 2026-07-04
updated: 2026-07-04
author: nyml
area: rendering
affects: [TEP-0003, TEP-0004]
related: [TEP-0001]
---

# TEP-0002: 渲染层——Cell 网格、diff 算法与终端适配器

## 摘要

定义 TUI 框架的渲染管道：VirtualScreen（字符画布）→ diff（脏区计算）→ TerminalBackend（写入终端）。所有核心类型和算法放在 domain/，终端 I/O 适配器放在 infra/。

## 目标

拿到布局引擎给的 widget 坐标和内容后，把字符画到终端上。只画变化的部分。

## Cell 数据结构

```rust
// domain/cell.rs

/// 256 色调色板颜色索引（0-255）
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PaletteColor(pub u8);

impl Default for PaletteColor {
    fn default() -> Self { PaletteColor(7) }  // 默认前景白色
}

/// ANSI 颜色——256 色为必有字段，TrueColor 为可选附加
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct AnsiColor {
    pub palette: PaletteColor,
    pub true_color: Option<Rgb>,  // 终端不支持时直接丢弃此字段
}

impl Default for AnsiColor {
    fn default() -> Self {
        Self { palette: PaletteColor::default(), true_color: None }
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// 字符属性 —— bitflags
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Attrs {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl Default for Attrs {
    fn default() -> Self {
        Self { bold: false, dim: false, italic: false, underline: false, reverse: false }
    }
}

/// 单个字符单元
#[derive(Clone, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,          // UTF-8 scalar value
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: AnsiColor::default(),
            bg: AnsiColor { palette: PaletteColor(0), true_color: None },  // 默认背景黑色
            attrs: Attrs::default(),
        }
    }
}
```

### TrueColor 降级规则

终端不支持 24bit RGB 时，直接丢弃 `true_color` 字段，仅输出 256 色 palette 序列。框架不做 RGB → 256 色自动映射——映射逻辑由上层主题系统负责。Cell 的 `PartialEq` 不比较 `true_color`（仅比较 `palette`），降级后不影响 diff 判断。

### NO_COLOR 处理

`NO_COLOR=1` 环境变量设置后，所有 `fg`/`bg` 自定义颜色被替换为终端默认前景/背景。保留 `bold`/`italic`/`underline`/`reverse` 文本属性——纯文本区分仍然可用。拦截时机：每次从主题取色生成 Cell 前。

## VirtualScreen

```rust
// domain/screen.rs

/// 字符画布——cols × rows 的扁平数组，行优先
pub struct VirtualScreen {
    cells: Vec<Cell>,    // 长度 = cols * rows
    cols: u16,
    rows: u16,
}

impl VirtualScreen {
    /// 创建全空白画布（填充 Cell::default()）
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cells: vec![Cell::default(); cols as usize * rows as usize], cols, rows }
    }

    /// resize 画布。增大区域填充空白 Cell，缩小区域裁剪丢弃
    pub fn resize(&mut self, cols: u16, rows: u16) { /* ... */ }

    /// 越界访问返回 Cell::default() 空白，不 panic
    pub fn cell_at(&self, col: u16, row: u16) -> Cell { /* ... */ }

    /// 越界写入静默忽略，不修改画布
    pub fn cell_at_mut(&mut self, col: u16, row: u16) -> &mut Cell { /* ... */ }

    /// 写字符串到指定位置。超出宽度自动截断，不换行，不越界
    pub fn write_str(&mut self, col: u16, row: u16, text: &str, style: Style) { /* ... */ }
}
```

渲染循环：
1. Widget tree 的 `render(rect: Rect) → VirtualScreen` 为每个 widget 生成局部画布，合并到全局画布
2. `diff(&old, &new) → Vec<DirtyRegion>` 逐行比较
3. `backend.emit(&dirty_regions, &new_screen)` 写入终端
4. `old = new`

## Diff 算法

```rust
// domain/diff.rs

/// 脏区域——单行内的一段连续列区间
#[derive(Clone, PartialEq, Eq)]
pub struct DirtyRegion {
    pub row: u16,
    pub start_col: u16,
    pub end_col: u16,   // exclusive
}

/// 逐行比较两个 VirtualScreen，返回脏区列表
/// 一行内可能有多个独立脏区（零散变化），不做行内合并
pub fn diff(old: &VirtualScreen, new: &VirtualScreen) -> Vec<DirtyRegion> {
    let mut regions = Vec::new();
    let rows = old.rows.min(new.rows);
    let cols = old.cols.min(new.cols);

    for row in 0..rows {
        let old_offset = row as usize * old.cols as usize;
        let new_offset = row as usize * new.cols as usize;
        let old_row = &old.cells[old_offset..old_offset + cols as usize];
        let new_row = &new.cells[new_offset..new_offset + cols as usize];

        // 该行无变化，直接跳过
        if old_row == new_row { continue; }

        // 扫描该行，输出所有不连续脏区间
        let mut in_dirty = false;
        let mut dirty_start = 0u16;
        for (i, (a, b)) in old_row.iter().zip(new_row.iter()).enumerate() {
            if a != b && !in_dirty {
                in_dirty = true;
                dirty_start = i as u16;
            } else if a == b && in_dirty {
                in_dirty = false;
                regions.push(DirtyRegion { row, start_col: dirty_start, end_col: i as u16 });
            }
        }
        if in_dirty {
            regions.push(DirtyRegion { row, start_col: dirty_start, end_col: cols });
        }
    }

    // 旧屏幕多出的行 → 整行清空脏区
    for row in new.rows..old.rows {
        regions.push(DirtyRegion { row, start_col: 0, end_col: old.cols.min(new.cols) });
    }
    // 新屏幕多出的行 → 整行新建脏区
    for row in old.rows..new.rows {
        regions.push(DirtyRegion { row, start_col: 0, end_col: new.cols });
    }

    regions
}
```

不做 Myers diff。终端 Cell 是固定大小的扁平数组，逐行比较 O(rows × cols) = O(14,400)，在 Rust 里 < 100μs。

一行内可能有多个不连续脏区（如行首和行尾各变了一个字符，中间不变）——diff 输出多个独立 `DirtyRegion`。行内合并逻辑后置到 `TerminalBackend::emit` 阶段——writer 可优化光标跳转，diff 层保持简单。

## TerminalBackend trait

```rust
// domain/backend.rs —— 在 domain 层定义

pub trait TerminalBackend {
    /// 进入 raw mode，返回 RAII guard（Drop 时恢复终端全部设置）
    /// 同一进程仅允许一个活跃 guard
    fn enter_raw_mode(&self) -> Box<dyn TerminalGuard>;

    /// 获取终端尺寸（cols, rows）
    fn size(&self) -> (u16, u16);

    /// 批量写入 ANSI escape codes
    /// 输入脏区无序——emit 内部先按 (row, start_col) 排序
    /// 同行相邻脏区间合并，最小化光标移动
    /// 单次 emit 结束统一 flush
    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen);

    /// 隐藏/显示光标
    fn hide_cursor(&mut self);
    fn show_cursor(&mut self);

    /// 进入/退出 alternate screen
    fn enter_alternate_screen(&mut self);
    fn exit_alternate_screen(&mut self);

    /// 清屏
    fn clear(&mut self);

    /// 刷新输出（flush stdout）
    fn flush(&mut self);
}

/// RAII 终端守卫——Drop 时恢复：
/// raw mode、echo、canonical mode、光标状态、alternate screen
pub trait TerminalGuard: Drop {}
```

### emit 职责

1. 脏区按 `(row, start_col)` 升序排序
2. 同行相邻区间（`prev.end_col >= next.start_col`）合并为单区间
3. 复用当前光标位置：同一行内用 `CSI n C`（右移）而非重新定位
4. 批量 queue ANSI 序列，不在循环内频繁 flush
5. 单次 `emit()` 结束统一 flush stdout。不跨 `emit()` 调用累积缓冲——每次渲染循环调用一次 `emit()`，emit 内部 queue → 末尾 flush

### Flush 策略

- **触发时机**：每次 `emit()` 调用末尾统一 flush，不设缓冲区大小阈值。终端屏幕尺寸有限（≤14,400 Cell），单次 emit 的 ANSI 序列总量远低于 stdout 缓冲区（通常 8KB），不设分段 flush
- **不跨 emit 缓冲**：flush 后缓冲区清空。下次 `emit()` 从零开始 queue。避免跨帧缓冲导致的输出延迟
- **异常安全**：`TerminalGuard` Drop 时强制 flush 一次，确保退出 raw mode 前所有输出已写入终端

## CrosstermBackend（infra）

```rust
// infra/crossterm_backend.rs

pub struct CrosstermBackend {
    stdout: Stdout,
    cursor_hidden: bool,
}

impl TerminalBackend for CrosstermBackend {
    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) {
        // 1. 排序 regions（row → col）
        // 2. 合并同行相邻区间
        // 3. 对每个区间：cursor move → write styled text
        // 4. 用 crossterm queue! 宏批量写入
        // 5. 最后 flush
    }
}
```

ANSI 序列生成：CrosstermBackend 内部直接使用 crossterm `queue!`、`Stylize`。domain 层完全不依赖 crossterm 转义码常量——颜色/样式仅通过 Cell 结构表达。未来扩展其他后端（Windows legacy）时独立实现，不污染 domain。

## 模拟后端（测试）

```rust
// infra/simulated_backend.rs

pub struct SimulatedBackend {
    screen: VirtualScreen,
    pub output: Vec<u8>,        // 记录所有 emit 调用的 ANSI 序列
}

impl TerminalBackend for SimulatedBackend {
    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) {
        // 不写真实终端，直接修改内存 screen + 记录 ANSI 序列
    }
}
```

CI 无屏测试——验证 diff + emit 正确性而不需要真实终端。

## 闪烁对策

- **Alternate screen 天然双缓冲**：进入 `CSI ? 1049 h`，渲染循环每次 emit 直接覆盖脏区不先清屏，退出 `CSI ? 1049 l`
- **光标隐藏**：程序生命周期默认隐藏（`CSI ? 25 l`），仅 Input 组件获焦点时临时显示
- **禁止全局 clear()**：全屏刷新仅通过 diff 局部脏区更新，不主动清屏

## 文本测量

```rust
// domain/text.rs

pub fn measure_width(text: &str) -> u16 { /* unicode-width */ }

pub fn expand_tabs(text: &str) -> String { /* \t → 空格补齐至 tab stop */ }

pub fn truncate(text: &str, max_width: u16, strategy: TruncateStrategy) -> String { /* ... */ }

pub fn wrap_lines(text: &str, max_width: u16, strategy: WrapStrategy) -> Vec<String> { /* ... */ }
```

### Tab 展开规则

- Tab 宽度固定 4 字符
- `\t` 替换为空格，补齐至当前列对齐下一个 4 的倍数位置
- 测量/换行/截断前统一展开，画布 Cell 中不存在 `'\t'`

文本测量是纯函数，放 domain/。宽度计算、截断、换行供布局引擎（TEP-0003）与渲染层共用。

## 颜色主题集成

Cell 的 `fg`/`bg` 使用 `AnsiColor { palette: PaletteColor, true_color: Option<Rgb> }`。主题系统（TEP-0005）负责把逻辑颜色（"primary"、"error"、"surface"）映射到 `AnsiColor`。渲染层不管颜色语义——只管把给定颜色写到终端。

终端深浅色主题：v1 不做自动检测。用户通过环境变量 `THEME=light/dark` 或启动参数手动指定。`COLORFGBG`、OSC 查询等自动检测方案兼容性差，远期独立 TEP 评估。

## 完整渲染主循环

对齐 TEP-0003（布局）和 TEP-0004（输入）：

```
1. 输入线程推送 KeyEvent → 主线程批量处理事件
2. Widget 状态变更 → signal.set() → DirtyTracker 标记脏 widget
3. 若 dirty：
   a. 布局引擎 measure_tree + layout_tree（TEP-0003）→ 输出所有 WidgetLayoutInfo
   b. 新建空白 VirtualScreen，遍历 widget 调用 render(content_rect) 填充 Cell
   c. diff(old_screen, new_screen) → 脏区列表
   d. backend.emit(&dirty_regions, &new_screen) → 终端显示
   e. old_screen = new_screen
4. 等待下一轮输入
```

SIGWINCH：resize 触发 `VirtualScreen.resize()`，全树重 layout（TEP-0003），全画布重渲染，diff 自动生成全行脏区。

## 已决议的开放问题

- [x] **ANSI 序列生成**：CrosstermBackend 直接使用 crossterm `queue!`/`Stylize`。domain 层不依赖 crossterm 常量。其他后端独立实现
- [x] **Diff SIMD**：v1 不做，仅保留朴素逐行对比。先上线基础版本，profiling 确认瓶颈后再评估
- [x] **终端深浅色检测**：v1 不做自动检测。用户通过 `THEME` 环境变量或启动参数手动指定。远期独立 TEP

## 附录 A：DirtyRegion 合并算法（Backend emit 阶段）

1. 脏区按 `(row, start_col)` 升序排序
2. 同行相邻区间若 `prev.end_col >= next.start_col`，合并为 `(row, prev.start_col, next.end_col)`
3. 相邻连续行的完整行脏区（start=0, end=cols）可批量处理减少行跳转

## 附录 B：渲染层测试分层

**domain 单元测试（无 IO）：**
- Cell 相等性、Default 空白基线
- diff：尺寸一致、缩小（新旧行列不同）、放大、一行多段零散脏区
- text：tab 展开、宽度测量、Word/Char 换行、End/Middle 截断

**infra 集成测试：**
- `SimulatedBackend` 捕获输出，校验 ANSI 序列、脏区绘制范围
- Alternate screen 切换 + raw mode 守卫 RAII 恢复

## 附录 C：工程实施顺序

1. domain 基础类型：`Cell`/`AnsiColor`/`Rgb`/`Attrs` + Default 实现
2. `VirtualScreen` 画布基础读写、resize、write_str
3. text 工具函数（宽度、tab 展开、截断、换行）
4. diff 逐行对比，覆盖行列增减场景
5. domain `TerminalBackend` trait + `TerminalGuard` 抽象
6. infra `SimulatedBackend`，编写 diff 渲染单元测试
7. infra `CrosstermBackend`，实现 emit 脏区排序合并、ANSI 批量输出
8. Alternate screen、raw mode、光标控制封装
9. 主循环接入渲染管线，对接 TEP-0003 布局、TEP-0004 输入
10. NO_COLOR、主题颜色降级适配

## 参考

- [crossterm](https://github.com/crossterm-rs/crossterm) — 跨平台终端控制
- [unicode-width](https://crates.io/crates/unicode-width) — Unicode UAX #11
- [ANSI Escape Codes](https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797)
