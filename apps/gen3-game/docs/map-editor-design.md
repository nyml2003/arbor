# Gen3 地图编辑器设计

状态：首个可运行切片已实现，完整编辑工具仍在迭代

使用方法见 [地图编辑器使用教程](map-editor-usage.md)。

## 结论

地图编辑器作为 `gen3-game` Cargo workspace 中的独立可执行文件运行：

```powershell
cargo run -p map-editor --release
```

游戏和编辑器必须使用同一个 `map-render` crate。两者不能各自实现 tile 展开、相机裁剪、atlas 映射和绘制顺序。

编辑器接收已经准备好的 16x16 原子素材。素材提取、切图和去重不属于本方案。

一个组合素材由任意数量的原子素材按顺序叠加而成。第一版不限制叠加层数，不烘焙新 PNG，也不做组合缓存。组合结果保存为扁平配方，并作为新素材继续绘制地图。

## 当前实现

已落地：

- `map-project`：版本化 JSON、三套独立图层、组合素材、验证、Undo/Redo。
- `map-render`：原子素材目录映射、组合层稳定展开、缩放布局和共享 scene plan。
- `map-editor`：独立 winit 可执行文件、MVC 工作台、GPU 文字、原子/组合素材分页、2x 地图画布、绘制、擦除、保存和图层 overlay。
- `game-host`：加载同一份地图 JSON 和 tile 目录，复用 `map-render`，从碰撞与事件图层建立 world。
- `maps/demo-map.json`：可直接打开的 24x16 示例地图，包含单层和双层组合素材。

当前编辑器输入：

- 鼠标左键绘制或点击工作台控件，右键擦除；滚轮切换原子素材页。
- 原子素材和 Material brushes 都提供可点击的上一页/下一页；新组合会自动显示所在页。
- 编辑器以 2x tile span 完整显示 24x16 地图；游戏使用共享 Camera 投影 16x10 可见区域。
- `V` 切回视觉层，`1`/`2` 选择可通行/阻挡碰撞层，`3`/`4` 设置/清除遭遇事件。
- `PageUp`/`PageDown` 切换原子素材。
- `A` 在当前组合上追加选中的原子层，`D` 删除顶层；两者都以 copy-on-write 创建新组合。
- `Ctrl+S` 保存，`Ctrl+Z`/`Ctrl+Y` 撤销和重做；这些操作也有可点击按钮。

矩形填充、吸管、拖动批次合并、层排序、spawn 编辑和原子替换保存仍属于后续编辑工具，不影响当前文件和共享渲染合同。

## 编辑器 MVC

`map-editor` 内部按 MVC 拆分，平台生命周期不属于任何业务层：

```text
winit/wgpu shell (main.rs)
  -> Controller: 输入命中与 EditorIntent
  -> Model: MapProject、选择状态、编辑命令、Undo/Redo
  -> View: 固定工作台、文字、缩略图、图层 overlay
  -> shell: 单次 present 与文件保存
```

- `model.rs` 不依赖 winit、wgpu、glyphon 或文件系统。所有地图修改只能通过 `EditorIntent` 进入。
- `controller.rs` 不修改 `MapProject`，只根据固定布局把指针输入转换为 intent。
- `view.rs` 只读取 Model，并调用共享 `map-render` 生成地图部分；它不能执行编辑命令。
- `layout.rs` 使用 `Row` 和 `Column` 分配工作台区域，并生成唯一的 `WorkbenchLayout`；View 与 Controller 共享同一份矩形。
- `assets.rs` 是 PNG 目录和默认地图的文件系统适配器。
- `text.rs` 是 View 使用的 glyphon GPU 文字实现。
- `main.rs` 只持有窗口、GPU runtime、保存副作用和 MVC 调度。

Model、Controller 和 View 分别有聚焦测试。布局改变不应修改 Model，新增编辑规则不应进入 View。
布局测试会检查全部交互区域都在工作台内且两两不重叠。

## 原始问题

当前世界地图有三个限制：

- `world-domain::Tile` 只有 `Ground`、`Wall` 和 `Grass`。
- `world-application` 在代码中生成固定 16x10 演示地图。
- `game-ui` 根据 tile 类型绘制纯色块，没有读取地图素材。

这些限制已经由 `map-project`、独立碰撞/事件图层和共享 `map-render` 处理。这里保留为设计背景。

## 目标

- 使用已有 16x16 原子素材拼装组合素材。
- 一个组合素材支持任意数量的叠加层。
- 使用组合素材绘制有限宽高的地图。
- 单独编辑通行、阻挡和遭遇语义。
- 编辑器和游戏使用同一个地图渲染实现。
- 编辑器作为独立可执行文件运行。
- 地图、编辑器 overlay 和工具 UI 在一帧内只 present 一次。
- 地图文件可由编辑器保存，并由游戏直接加载。
- 编辑操作支持 Undo 和 Redo。

## 非目标

- 不负责提取、切分或去重原始图片。
- 第一版不支持无限地图。地图宽高仍是明确的有限值。
- 第一版不做 chunk、RLE、实例合并或可见性缓存。
- 第一版不把组合素材烘焙为新 PNG。
- 第一版不自动去重内容相同的组合素材。
- 第一版不支持递归组合素材。
- 第一版不处理跨多个格子的 object stamp。
- 第一版不处理位于角色上方的树冠或前景平面。
- 不创建仓库级 `punctum-scene` 公共框架。

## 依赖结构

新增三个 crate：

```text
apps/gen3-game/crates/
  map-project/
  map-render/
  map-editor/
```

依赖方向固定为：

```text
game-host  -> map-render -> map-project
map-editor -> map-render -> map-project

game-host  -> punctum-wgpu
map-editor -> punctum-wgpu
map-render -> punctum-gpu + punctum-grid

map-project -X-> winit / wgpu / filesystem
map-render  -X-> winit / filesystem
```

`map-project` 持有地图文档、组合素材、编辑命令和验证规则。它不读取文件，不知道 GPU 和窗口。

`map-render` 把地图文档投影为共享地图 draw plan。它知道 atlas resource、相机、viewport 和绘制顺序，不持有窗口或 surface。

`map-editor` 持有编辑器状态、鼠标输入、文件 IO 和工具 UI。它不能复制地图渲染逻辑。

`game-host` 加载地图文件，建立 world application，并把共享地图 draw plan 与角色、文字和其他游戏内容合成。

## 共享渲染链路

游戏链路：

```text
MapProject
  -> map-render
  -> MapScenePlan
  -> 角色和场景图片
  -> plan_composite
  -> 游戏文字和 overlay
  -> GpuRuntime::present_plan_with_overlay
```

编辑器链路：

```text
MapProject
  -> map-render
  -> MapScenePlan
  -> plan_composite
  -> 网格、hover、画笔、选区和工具 UI overlay
  -> GpuRuntime::present_plan_with_overlay
```

`map-render` 的输入至少包含：

```rust
pub struct MapRenderInput<'a> {
    pub project: &'a MapProject,
    pub catalog: &'a AtomicTileCatalog,
    pub camera: MapCamera,
    pub viewport: Viewport,
}
```

输出至少包含：

```rust
pub struct MapScenePlan {
    pub base: Surface<GpuCell>,
    pub tile_images: Vec<GpuImage>,
    pub viewport: Viewport,
}
```

这里的 `Surface<GpuCell>` 是 Punctum 逻辑网格，不是 wgpu surface。`map-render` 仍不能 acquire、submit 或 present。

每个可见地图格按以下流程展开：

```text
VisualCell
  -> CompositeTileId
  -> CompositeTile.layers
  -> AtomicTileId
  -> atlas ResourceId
  -> 按 layers 顺序生成 GpuImage
```

第一版直接为每一层生成一个 draw item。层数越多，draw item 越多。先保证语义正确，再决定是否缓存或烘焙。

共享渲染合同必须覆盖：相同 `MapProject`、`AtomicTileCatalog`、camera 和 viewport，在游戏与编辑器中产生相同的地图 tile resource、目标矩形和绘制顺序。

编辑器 overlay 不属于 `MapScenePlan`，也不能写入地图文件。

## 地图数据模型

### 类型安全 ID

```rust
pub struct AtomicTileId(String);
pub struct CompositeTileId(String);
pub struct MapProjectId(String);
```

三种 ID 不能使用裸字符串互传。

### 运行时原子素材目录

原子素材由外部目录提供。地图项目只引用稳定 ID。`AtomicTileCatalog` 属于 `map-render` 边界，不属于 `map-project`。

```rust
pub struct AtomicTileResource {
    pub id: AtomicTileId,
    pub resource: ResourceId,
}

pub struct AtomicTileCatalog {
    pub tiles: Vec<AtomicTileResource>,
}
```

`ResourceId` 属于运行时 atlas 映射。地图 JSON 只保存 `AtomicTileId`，不能保存 GPU resource number。`map-project` 不 import `punctum-gpu`。

### 组合素材

```rust
pub struct CompositeTile {
    pub id: CompositeTileId,
    pub name: String,
    pub layers: Vec<AtomicTileId>,
}
```

`layers[0]` 最先绘制。后续元素依次覆盖前一层。PNG alpha 决定最终可见像素。

组合素材必须是扁平列表。它不能引用另一个 `CompositeTileId`。

当用户把已有组合素材继续叠加时，编辑器先展开原有原子层，再创建新的组合素材：

```text
grass = [tile-grass]
flower = [tile-grass, tile-shadow, tile-flower]
wet-flower = [tile-grass, tile-shadow, tile-flower, tile-water-light]
```

扁平结构支持任意层数，并避免递归引用和循环依赖。

### 视觉、碰撞和事件图层

```rust
pub enum Collision {
    Walkable,
    Blocked,
}

pub enum MapEventKind {
    Encounter,
}

pub struct VisualCell {
    pub material: Option<CompositeTileId>,
}
```

三种数据使用独立数组保存：

- `visual_cells` 只控制视觉。
- `collision_cells` 只控制通行和阻挡。
- `event_cells` 保存踩入触发的遭遇等事件；空值表示没有事件。

不能根据某个 tile ID 推断碰撞。相同草地图片可以用于普通地面，也可以用于遭遇区域。

### 地图项目

```rust
pub struct MapProject {
    pub id: MapProjectId,
    pub tile_size: TilePixelSize,
    pub width: u16,
    pub height: u16,
    pub materials: Vec<CompositeTile>,
    pub visual_cells: Vec<VisualCell>,
    pub collision_cells: Vec<Collision>,
    pub event_cells: Vec<Option<MapEventKind>>,
    pub player_spawn: TilePosition,
}
```

第一版要求 `tile_size` 等于 16x16。三个图层都使用行优先的稠密数组，索引相同但存储与消费边界独立。

## 文件格式

第一版使用 JSON：

```json
{
  "format_version": "gen3-map-v1",
  "id": "aoba-field",
  "tile_size": { "width": 16, "height": 16 },
  "width": 64,
  "height": 48,
  "materials": [
    {
      "id": "material-grass-flower",
      "layers": [
        "tile-grass",
        "tile-shadow",
        "tile-flower"
      ]
    }
  ],
  "visual_cells": [{ "material": "material-grass-flower" }],
  "collision_cells": ["walkable"],
  "event_cells": ["encounter"],
  "player_spawn": [8, 12]
}
```

`map-project` 负责 wire schema 与领域模型转换。JSON 解析错误不能直接泄漏到 `world-domain`。

保存文件时先写临时文件，再原子替换目标文件。保存失败不能破坏最后一次成功保存的地图。

## 组合素材编辑规则

### 新建

用户从一个原子素材或现有组合素材开始。编辑器在右侧组合面板显示扁平层列表。

### 修改

支持以下操作：

- 添加原子层。
- 删除层。
- 复制层。
- 拖动排序。
- 临时隐藏层。
- 清空当前组合。

临时隐藏只属于编辑器状态，不写入组合素材。

### 保存为新素材

默认使用 copy-on-write。用户修改组合后保存，编辑器创建新的 `CompositeTileId`。原有地图格继续引用旧素材。

### 覆盖素材

覆盖是显式命令。覆盖后，所有引用这个 `CompositeTileId` 的地图格都会显示新结果。

### 重复素材

第一版允许两个组合素材包含相同的 `layers`。不自动合并，也不使用内容 hash 作为 ID。

## 编辑器状态

```rust
pub struct MapEditorState {
    pub project: MapProject,
    pub camera: MapCamera,
    pub tool: EditorTool,
    pub brush: Option<CompositeTileId>,
    pub hovered_cell: Option<TilePosition>,
    pub selection: Option<TileRect>,
    pub composition: CompositionDraft,
    pub dirty: bool,
}
```

工具模式：

```rust
pub enum EditorTool {
    Paint,
    Erase,
    Pick,
    Compose,
    Collision,
    Event,
    Pan,
}
```

`camera`、hover、selection、临时隐藏层和当前工具不写入地图文件。

## 编辑器界面

```text
┌ 素材库 ─────┬──────── 地图画布 ────────┬ 组合素材 ─────┐
│ 原子素材     │ 地图、网格和选区          │ layer 4 花     │
│ 组合素材     │ 缩放、平移和绘制          │ layer 3 阴影   │
│ 搜索和分类   │ 碰撞与遭遇 overlay        │ layer 2 草边   │
│              │                           │ layer 1 草地   │
├──────────────┴───────────────────────────┴───────────────┤
│ 文件状态、坐标、缩放比例、当前工具和诊断                 │
└─────────────────────────────────────────────────────────┘
```

第一版必须包含：

- 新建、打开和保存地图。
- 原子素材与组合素材列表。
- 组合层增加、删除、复制和排序。
- 保存为新素材和显式覆盖素材。
- 画笔、擦除和吸管。
- 矩形填充。
- 通行、阻挡和遭遇语义画笔。
- 地图缩放和平移。
- 网格开关。
- Undo 和 Redo。
- 未保存状态和结构化错误显示。

编辑器控件使用编辑器私有 overlay projection。不要把素材面板、按钮或选区状态放进 `map-render`。

## 输入边界

鼠标和窗口事件只进入 `map-editor` shell。编辑器把平台事件转换为纯编辑意图：

```rust
pub enum MapEditorIntent {
    PaintCell(TilePosition),
    EraseCell(TilePosition),
    PickCell(TilePosition),
    FillRect(TileRect),
    SetCollision(TilePosition, Collision),
    SetEvent(TilePosition, Option<MapEventKind>),
    AddCompositionLayer(AtomicTileId),
    RemoveCompositionLayer(usize),
    MoveCompositionLayer { from: usize, to: usize },
    SaveCompositionAsNew,
    OverwriteComposition(CompositeTileId),
    Undo,
    Redo,
}
```

屏幕坐标到地图格坐标的转换必须使用 `map-render` 的 camera 与 viewport 规则。不能在编辑器中维护第二套坐标公式。

## Undo 和 Redo

每个用户动作转换为可逆命令：

```rust
pub enum MapEditCommand {
    ReplaceCells {
        changes: Vec<CellChange>,
    },
    CreateMaterial {
        material: CompositeTile,
    },
    ReplaceMaterial {
        before: CompositeTile,
        after: CompositeTile,
    },
    RemoveMaterial {
        material: CompositeTile,
    },
}
```

一次鼠标拖动产生一个 `ReplaceCells`，不能为每个经过的格子创建一条独立历史记录。

Undo/Redo 历史属于编辑会话，不写入地图文件。

## 验证规则

`map-project` 加载和保存时必须验证：

- schema version 正确。
- tile size 是 16x16。
- 地图宽高大于零。
- `visual_cells`、`collision_cells` 和 `event_cells` 的长度都等于宽乘高。
- 所有 `AtomicTileId` 都能在素材目录解析。
- 所有 `CompositeTileId` 唯一。
- 所有地图格引用的组合素材存在。
- 组合素材名称非空。
- 组合素材至少包含一个原子层。
- spawn 位于地图内。
- spawn 所在格可通行。
- 所有面积和索引计算使用 checked arithmetic。

第一版不设置组合层数的业务上限。加载器仍需拒绝会导致整数溢出或无法分配内存的输入。

正常失败使用结构化错误：

```rust
pub enum MapProjectError {
    UnsupportedSchema(String),
    InvalidTileSize(TilePixelSize),
    EmptyMap,
    CellCountMismatch { expected: usize, actual: usize },
    UnknownAtomicTile(AtomicTileId),
    DuplicateMaterial(CompositeTileId),
    UnknownMaterial(CompositeTileId),
    EmptyMaterial(CompositeTileId),
    SpawnOutOfBounds(TilePosition),
    SpawnBlocked(TilePosition),
    CapacityOverflow,
}
```

文件读取、写入和替换错误由 `map-editor` adapter 转换为编辑器诊断。

## 与世界领域的关系

`world-domain` 不读取完整地图项目。运行时先把 `MapProject` 编译为世界规则需要的数据：

```text
MapProject.collision_cells + MapProject.event_cells
  -> runtime TileMap / collision / encounter rules
```

视觉数据走另一条路径：

```text
MapProject.visual_cells[].material
  -> map-render
  -> GPU scene plan
```

角色位置、碰撞和遭遇继续由 `world-domain` 决定。`map-render` 不能修改 world state。

当前 `world-domain::Tile` 后续应替换为独立语义类型。不要把 `CompositeTileId` 塞进现有 `Ground/Wall/Grass` 枚举。

## 一帧合成

编辑器沿用现有 `punctum-wgpu` surface 生命周期：

1. `map-render` 生成地图 draw plan。
2. `punctum-gpu` 生成地图 GPU submission plan。
3. 编辑器生成网格、hover、选区和工具 UI overlay plan。
4. `GpuRuntime::present_plan_with_overlay` 在同一个 encoder 中编码地图和 overlay。
5. runtime 只 acquire、submit 和 present 一次。

`map-editor` 不能自行 acquire surface，也不能绕过 `GpuRuntime` 直接 present。

## 实施阶段

### 阶段 1：地图合同

新增 `map-project`。

实现：

- 类型安全 ID。
- `CompositeTile`、`VisualCell`、`Collision`、`MapEventKind` 和 `MapProject`。
- JSON wire schema。
- 验证和结构化错误。
- copy-on-write 组合素材操作。
- 可逆编辑命令和 Undo/Redo。

完成标准：纯内存测试覆盖无限层列表、扁平组合、无效引用、地图尺寸、spawn 和命令逆操作。

### 阶段 2：共享地图渲染

新增 `map-render`。

实现：

- camera 和 viewport。
- 可见格计算。
- 组合素材展开。
- 原子素材到 atlas resource 映射。
- 稳定的层顺序。
- `MapScenePlan`。

修改 `game-host`，让游戏加载一个最小地图项目。现有纯色 `project_world` 逐步退出地图视觉路径。

完成标准：固定地图的 CPU draw plan golden 通过；游戏能使用地图项目运行。

### 阶段 3：最小编辑器

新增 `map-editor` 独立可执行文件。

实现：

- winit 窗口和 `GpuRuntime`。
- 地图画布。
- 素材列表。
- 组合层列表。
- Paint、Erase、Pick 和 Pan。
- 新建、打开和保存。
- Undo 和 Redo。

完成标准：编辑器能创建组合素材、绘制地图、保存文件并重新打开。

### 阶段 4：地图语义

实现：

- Walkable、Blocked 和 Encounter 画笔。
- spawn 编辑。
- 语义 overlay。
- `MapProject` 到 runtime world map 的编译。

完成标准：游戏按编辑结果阻挡移动，并在 Encounter 格触发战斗。

### 阶段 5：共享链路门禁

增加游戏与编辑器的共享合同测试。

完成标准：

- 相同输入生成相同地图 draw plan。
- 编辑器 overlay 不改变地图 draw plan。
- 编辑器保存的文件可由游戏加载。
- 游戏和编辑器都只 present 一次。
- 没有第二套 tile 展开、坐标转换或 atlas 映射实现。

## 测试矩阵

| 范围 | 必须覆盖 |
| --- | --- |
| map-project | schema、ID、尺寸、引用、spawn、容量溢出 |
| 组合素材 | 层顺序、重复层、长层列表、扁平组合、copy-on-write |
| 编辑命令 | 单格、矩形、拖动批次、Undo、Redo |
| map-render | camera、裁剪、viewport、atlas 映射、稳定绘制顺序 |
| map-editor | 打开、保存、dirty 状态、文件失败、鼠标坐标转换 |
| game-host | 地图加载、碰撞、遭遇、角色与地图合成 |
| 共享合同 | 游戏和编辑器的地图 draw plan 完全相同 |

“无限叠加”测试不需要分配无限数据。测试应使用较长层列表，并证明实现没有固定层数分支或 schema 上限。

## 后续优化入口

第一版行为稳定后，才评估以下优化：

- 按组合配方缓存展开结果。
- 对内容相同的组合素材做 hash 去重。
- 把组合素材烘焙到运行时 atlas。
- 只生成可见 chunk 的 draw item。
- 对静态地图做实例批处理。
- 使用 RLE 或 chunk 压缩地图文件。
- 增加多格 object stamp。
- 增加 Background、Actor 和 Foreground 场景平面。

这些优化不能改变 `MapProject` 的视觉顺序、碰撞/事件语义或共享渲染合同。

## 最终完成标准

- `map-editor` 是独立可执行文件。
- 编辑器和游戏使用同一个 `map-render`。
- 原子素材可以按任意层数叠加。
- 叠加结果可以保存为新组合素材。
- 组合素材可以重复用于多个地图格。
- 视觉和碰撞语义分离。
- 编辑器可以保存并重新加载地图。
- 游戏可以直接加载编辑器产物。
- 相同地图在游戏和编辑器中生成相同 draw plan。
- 地图和编辑器 overlay 每帧只 present 一次。
