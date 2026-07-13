# 游戏数据导入设计

状态：导入、Emerald 学习面、离线查询和可配置演示队伍已实现
适用项目：`apps/gen3-game`
数据源：PokeAPI `d638fe7791214a8d3c3282e2a3113eea7cfef288`

## 结论

当前代码可以把 `assets/pokeapi-current-data` 中的固定 CSV 快照导入为 JSON，并通过 `game-data::CurrentDataSet` 离线加载和查询。

`game-assets` 仍只处理 PNG 解码和 GPU atlas。游戏数据由独立 crate 负责。`game-host` 已从 `CurrentDataSet` 查询妙蛙种子、撞击和藤鞭，再投影为对战实例。

项目已新增两个 crate：

- `game-data`：持有静态数据模型、类型安全 ID、离线加载和只读查询。
- `game-data-import`：开发期命令，从固定 PokeAPI CSV 快照生成应用资源。

导入器不是 `build.rs`。普通构建不得访问网络，也不得隐式更新数据。生成后的资源使用 `include_bytes!` 编入程序，运行时不读取 CSV，不访问 PokeAPI。

## 目标

当前支持以下数据：

- 宝可梦形态与物种关联。
- 六项种族值。
- 单属性或双属性。
- 招式威力、命中、PP、优先级和伤害类别。
- 宝可梦、招式和属性的简体中文名称。
- 按稳定数值 ID 查询数据。
- 按明确的 `emerald` 版本组查询学习面。
- 校验队伍配置中的招式是否属于对应形态的学习面。
- 记录上游 commit、生成器版本和产物 schema 版本。

当前不支持以下数据：

- 特性、道具和携带物。
- 招式效果、异常状态、多段攻击和反作用力。
- 运行时联网或从用户目录加载 CSV。
- 将当前 PokeAPI 数据宣称为第三世代原始数据。

## 当前代码边界

现有结构有四个需要保留的事实：

1. `battle-domain` 是纯规则核心，不应读取文件或依赖 PokeAPI schema。
2. `Pokemon` 是一次对战中的实例。它包含等级、当前 HP、招式当前 PP 和实例 ID，不是静态图鉴记录。
3. `PokemonId` 和 `MoveId` 当前使用字符串，并服务于对战事件和实例识别。它们不能直接替代 PokeAPI 的形态 ID、物种 ID和招式 ID。
4. `fixtures/battle-rules-v0.1.json` 已声明物种、学习面和具体招式数值属于经过校验的外部输入。

因此，静态数据不能塞进 `battle-domain`，也不能复用 `Pokemon` 作为 CSV 反序列化结构。

## 数据链路

```text
固定 PokeAPI commit
        |
        v
assets/pokeapi-current-data/*.csv
        |
        v
game-data-import
  解析 -> 关联 -> 校验 -> 本地化 -> 稳定排序
        |
        v
assets/data/current-data-set-v2.json
        |
        v
game-data::CurrentDataSet
  include_bytes! -> 校验 schema -> 建立只读索引
        |
        v
game-host / application composition
        |
        v
battle-domain 的 Pokemon、Move 和 Team
```

CSV 是上游原始快照。JSON 是应用使用的生成产物。两者不能混用。

## 目录设计

```text
apps/gen3-game/
  assets/
    pokeapi-current-data/       # 固定 commit 的原始 CSV
    data/
      current-data-set-v2.json  # 生成产物
  fixtures/
    demo-roster-v1.json         # 可编辑的演示队伍
  crates/
    game-data/
      src/
        ids.rs
        model.rs
        query.rs
        wire.rs
        lib.rs
    game-data-import/
      src/
        csv_rows.rs
        import.rs
        diagnostics.rs
        main.rs
```

目录名可以调整。职责边界不应随目录调整而变化。

## 原始数据前置条件

现有快照包含：

- `pokemon.csv`
- `pokemon_stats.csv`
- `pokemon_types.csv`
- `moves.csv`
- `pokemon_species_names.csv`
- `move_names.csv`
- `type_names.csv`

快照已经从同一个 commit 补齐：

- `languages.csv`：通过语言标识找到简体中文 ID，避免硬编码 `local_language_id = 12`。
- `stats.csv`：通过 `identifier` 识别 HP、攻击、防御、特攻、特防和速度，避免硬编码 stat ID。
- `types.csv`：通过 `identifier` 识别属性，避免硬编码 type ID。
- `move_damage_classes.csv`：通过 `identifier` 识别物理、特殊和变化招式。
- `version_groups.csv`：通过 `identifier` 选择版本组。
- `pokemon_move_methods.csv`：通过 `identifier` 识别升级、机器、教学和遗传。
- `pokemon_moves.csv`：按形态和版本组导入学习面。

导入命令缺少这些表时必须失败。不要退回魔法数字映射。

## `game-data` 模型

所有上游 ID 使用数值 newtype。不同 ID 不能互换。

```rust
pub struct PokemonFormId(u32);
pub struct SpeciesId(u32);
pub struct MoveId(u32);
pub struct TypeId(u16);

pub struct CurrentDataSet {
    metadata: DataSetMetadata,
    pokemon: Vec<PokemonRecord>,
    moves: Vec<MoveRecord>,
    types: Vec<TypeRecord>,
}
```

`PokemonRecord` 表示 PokeAPI 的一个 `pokemon.id`，也就是形态记录。它至少包含：

```rust
pub struct PokemonRecord {
    pub id: PokemonFormId,
    pub species_id: SpeciesId,
    pub identifier: String,
    pub is_default: bool,
    pub base_stats: BaseStats,
    pub types: PokemonTypes,
    pub display_name: LocalizedName,
    pub learnset: Vec<LearnsetEntry>,
}
```

`pokemon_species_names.csv` 只提供物种名称。非默认形态在第一阶段可以使用“物种中文名 + 形态 identifier”作为回退展示名，但稳定身份仍使用 `PokemonFormId`。

`MoveRecord` 保留数据事实，不强行满足当前战斗模型：

```rust
pub struct MoveRecord {
    pub id: MoveId,
    pub identifier: String,
    pub display_name: LocalizedName,
    pub move_type: TypeId,
    pub power: Option<u16>,
    pub accuracy: Option<u8>,
    pub pp: Option<u8>,
    pub priority: i8,
    pub damage_class: DamageClass,
}
```

`power`、`accuracy` 和 `pp` 必须保留空值。变化招式没有威力；部分招式不走普通命中检查。导入器不能把空值改成 `0` 或 `100`。固定快照也使用 `0` 表示部分缺失值，导入器把这些零值哨兵规范化为 `None`，原始 CSV 保持不变。

`LearnsetEntry` 保存招式 ID、学习方式、等级和顺序。导入器只保留命令指定版本组的记录，不合并其他版本。

`DataSetMetadata` 至少包含：

- `schema_version`，当前值为 `current-data-set-v2`。
- `source_repository`。
- `source_commit`。
- `generator_version`。
- `locale`，初始值为 `zh-Hans`。
- `version_group`，当前值为 `emerald`。

## 查询接口

`game-data` 暴露只读查询，不暴露内部 map，也不返回 PokeAPI CSV row：

```rust
impl CurrentDataSet {
    pub fn embedded() -> Result<Self, DataLoadError>;
    pub fn pokemon(&self, id: PokemonFormId) -> Option<&PokemonRecord>;
    pub fn move_by_id(&self, id: MoveId) -> Option<&MoveRecord>;
    pub fn type_by_id(&self, id: TypeId) -> Option<&TypeRecord>;
    pub fn learnset(&self, id: PokemonFormId) -> Option<&[LearnsetEntry]>;
    pub fn can_learn(&self, pokemon: PokemonFormId, battle_move: MoveId) -> bool;
    pub fn pokemon_iter(&self) -> impl Iterator<Item = &PokemonRecord>;
    pub fn move_iter(&self) -> impl Iterator<Item = &MoveRecord>;
}
```

`embedded()` 使用 `include_bytes!("../../../assets/data/current-data-set-v2.json")`。它只解析已生成资源。它不读取原始 CSV。

当前使用 JSON 和 `serde_json`。数据量和启动开销有证据表明成为问题后，再改成二进制格式。schema 版本必须在格式变化时更新。

## 导入规则

导入器按以下顺序执行：

1. 读取并校验所有必需 CSV 的表头。
2. 从 `languages.csv` 找到 `zh-Hans`。
3. 从 `stats.csv`、`types.csv`、`move_damage_classes.csv`、`version_groups.csv` 和 `pokemon_move_methods.csv` 建立稳定标识映射。
4. 以 `pokemon.id` 关联种族值和属性。
5. 以 `pokemon.species_id` 关联物种中文名。
6. 以 `moves.id` 关联招式中文名、属性和伤害类别。
7. 只筛选命令指定 `version_group` 的学习面。
8. 关联形态、招式和学习方式。
9. 校验引用、基数和值域。
10. 按数值 ID 和学习面字段稳定排序。
11. 写入临时文件，重新读取并校验。
12. 替换正式产物。

输出必须确定。同一份 CSV、同一版导入器和同一组参数应生成逐字节相同的文件。生成产物不要写当前时间。

## 校验规则

导入失败时不生成部分产物。至少检查：

- 所有引用的物种、属性、招式和语言都存在。
- 每个宝可梦恰好有六项种族值，且 stat 不重复。
- 每个宝可梦有一到两个属性，slot 只能为 1 或 2，slot 不重复。
- ID 唯一且非零。
- 种族值大于零，并能放入目标整数类型。
- 招式命中率为空或处于 `1..=100`。
- 招式 PP 为空或大于零。
- 招式优先级能放入 `i8`。
- 中文名重复行、缺失行和空字符串按明确策略处理。
- metadata 中的 commit 与 `assets/pokeapi-current-data/SOURCE.md` 一致。
- metadata 中的版本组存在，且每条学习面记录引用有效形态、招式和学习方式。
- 同一形态的学习面没有完全重复记录，并按稳定顺序输出。

中文名缺失不是身份错误。查询层可以回退到英文 `identifier`，但导入报告必须列出缺失项。

## 错误模型

导入错误应包含结构化位置，不只返回字符串：

```rust
pub struct ImportDiagnostic {
    pub code: ImportDiagnosticCode,
    pub file: PathBuf,
    pub row: Option<usize>,
    pub field: Option<String>,
    pub entity_id: Option<u32>,
    pub message: String,
}
```

错误码至少区分：

- `MissingInputFile`
- `InvalidHeader`
- `InvalidField`
- `DuplicateId`
- `MissingReference`
- `MissingStat`
- `InvalidTypeSlots`
- `MetadataMismatch`
- `OutputValidationFailed`

`game-data` 的加载错误只处理生成产物，例如 `UnsupportedSchema`、`MalformedData` 和 `DuplicateId`。文件系统错误留在导入器，不进入运行时领域。

## 与战斗领域的边界

`PokemonRecord` 不能直接转换为 `battle-domain::Pokemon`。转换还需要运行时选择：

- 对战实例 ID。
- 等级。
- 个体值、努力值和最终能力值计算策略。
- 当前 HP。
- 最多四个已选招式。
- 每个招式的当前 PP。

当前 `battle-domain::Move` 要求威力大于零，因此变化招式不能直接构造。战斗投影返回结构化的“不支持”错误，不伪造威力。

当前战斗规则按第三世代属性决定物理或特殊分类，PokeAPI 当前数据则包含现代招式伤害类别。`CurrentDataSet` 保留现代数据事实。第三世代战斗投影必须明确使用第三世代规则，不能悄悄覆盖静态记录，也不能把投影结果写回数据集。

当前适配代码位于 `game-host::roster`。它读取 `fixtures/demo-roster-v1.json`，校验版本组、队伍大小、招式数量和学习面，再把静态记录转换为演示对战实例。`game-data` 不依赖 `battle-domain`。后续出现第二种队伍来源时，再把适配代码拆成独立 application 或 adapter crate。

## 命令接口

建议命令：

```powershell
cargo run -p game-data-import -- `
  --source assets/pokeapi-current-data `
  --output assets/data/current-data-set-v2.json `
  --version-group emerald `
  --locale zh-Hans `
  --source-commit d638fe7791214a8d3c3282e2a3113eea7cfef288
```

导入器只接受本地目录。下载和更新上游快照是另一个显式动作，不属于导入命令。

## 实施顺序

### 阶段 1：补齐原始快照（已完成）

从同一 commit 下载四张映射表，更新 `SOURCE.md` 和 `SHA256SUMS`。

完成标准：所有输入有固定来源和哈希，不依赖魔法数字。

### 阶段 2：建立 `game-data`（已完成）

实现 ID、静态记录、wire schema、嵌入加载和内存查询。

完成标准：fixture 能加载；重复 ID、错误 schema 和非法引用会失败。

### 阶段 3：建立 `game-data-import`（已完成）

实现 CSV row、关联、校验、诊断和确定性输出。

完成标准：对固定快照连续运行两次，产物哈希相同；破坏任一引用时，命令返回对应诊断且不覆盖旧产物。

### 阶段 4：接入应用（已完成）

`game-host` 在组合根加载 `CurrentDataSet`，并使用独立 JSON 配置构造双方队伍。

完成标准：切换形态、等级和招式时只修改队伍配置；程序仍能完全离线启动。

### 阶段 5：学习面与队伍配置（已完成）

导入 `emerald` 学习面和学习方式。队伍配置只能选择对应形态可以学习的招式。

### 阶段 6：扩展数据范围

分批加入特性、道具和复杂招式效果。每次扩展都升级 schema 或保持向后兼容，并增加导入校验。

## 测试范围

`game-data`：

- 加载最小合法 fixture。
- 按形态 ID、物种 ID、招式 ID 和属性 ID 查询。
- 拒绝错误 schema、重复 ID 和非法引用。
- 验证中文名回退。

`game-data-import`：

- 六项种族值和双属性正常关联。
- 形态 ID 与全国图鉴物种 ID 不混淆。
- 空威力、空命中和空 PP 保持为空。
- 缺表、坏表头、孤立引用和重复 slot 返回结构化诊断。
- 导入失败不替换已有输出。
- 只导入明确版本组的学习面。
- 输出顺序和哈希确定。

应用接入：

- 固定数据集能构造当前妙蛙种子演示队伍。
- 队伍中的非法招式会被学习面校验拒绝。
- 变化招式和不支持的现代属性不能误入第三世代战斗。
- 运行测试时不需要网络，也不需要原始 CSV。

维护这两个 crate 时运行：

```powershell
cargo fmt --all -- --check
cargo test -p game-data -p game-data-import -p game-host -p game-e2e
cargo clippy -p game-data -p game-data-import -p game-host -p game-e2e --all-targets -- -D warnings
```

## 暂不决定

以下问题等数据量和查询使用方式明确后再决定：

- JSON 是否替换为自定义二进制格式。
- 是否保留多语言名称，还是只生成 `zh-Hans`。
- 是否把战斗投影拆成独立 crate。
- 是否把原始 CSV 移出应用 assets。

这些选择不影响当前两 crate、构建期导入和运行时离线的基本边界。
