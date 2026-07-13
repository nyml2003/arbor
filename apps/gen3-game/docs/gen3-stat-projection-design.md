# 第三世代能力值投影设计

状态：设计完成，尚未接入运行时

## 结论

当前 `game-host::roster` 直接把 PokeAPI 种族值传给 `battle-domain::Pokemon`：

- HP 种族值直接作为 `max_hp` 和 `current_hp`。
- 攻击、防御、特攻、特防和速度种族值直接构造 `BattleStats`。

这不是宝可梦的实际能力值。实际能力值还取决于等级、个体值、努力值和性格。

接入实际能力值一定会改变现有演示行为。即使所有个体值和努力值都是 0，50 级能力值也不等于种族值。因此不能在保持现有数值、伤害和回合顺序不变的前提下直接切换。

实现应分为两个阶段：

1. 先新增纯计算模型和测试，不修改 host 的能力值来源。
2. 再单独批准 roster 接入，更新受影响的应用测试和演示基线。

## 目标

- 精确实现第三世代整数能力值公式。
- 区分种族值、个体值、努力值和最终能力值。
- 使用类型安全结构，避免数组下标和裸整数混传。
- 校验第三世代 IV、EV 和等级约束。
- 保持 `battle-domain::Pokemon` 只接收已经计算好的能力值。
- 第一阶段保持当前 host 和 E2E 行为不变。

## 非目标

- 本阶段不修改伤害公式。
- 本阶段不实现能力阶级变化。
- 本阶段不实现经验值和升级流程。
- 本阶段不自动生成竞技配点。
- 本阶段不把 IV、EV 写入 PokeAPI 静态数据集。
- 本阶段不切换现有随机演示队伍的实际能力值。

## 数据边界

四类数据必须分开：

| 数据 | 所属位置 | 生命周期 |
| --- | --- | --- |
| 种族值 | `game-data::PokemonRecord` | 固定上游数据 |
| 等级 | 队伍成员配置或生成结果 | 单个宝可梦实例 |
| IV、EV、性格 | 训练参数 | 单个宝可梦实例 |
| 最终 HP 和五项战斗能力值 | 公式计算结果 | 对战实例 |

`game-data` 不应保存某个实例的 IV、EV 或最终能力值。`battle-domain` 不应读取 CSV，也不应知道 PokeAPI ID。

## 建议位置

能力值公式属于第三世代战斗规则。建议在 `battle-domain` 中新增独立模块：

```text
crates/battle-domain/src/
  stats.rs
```

这个模块只做纯计算，不依赖 `game-data`。调用方把六项种族值转换为输入结构，再获取最终能力值。

保持现有 `BattleStats::new` 和 `Pokemon::new` API 不变。第一阶段只增加新 API，不替换旧调用点。

## 类型模型

```rust
pub struct StatBlock<T> {
    pub hp: T,
    pub attack: T,
    pub defense: T,
    pub special_attack: T,
    pub special_defense: T,
    pub speed: T,
}

pub struct IndividualValue(u8);
pub struct EffortValue(u8);

pub struct TrainingValues {
    pub ivs: StatBlock<IndividualValue>,
    pub evs: StatBlock<EffortValue>,
    pub nature: Nature,
}

pub struct CalculatedStats {
    pub max_hp: u32,
    pub battle: BattleStats,
}
```

`StatBlock<T>` 可以减少六项字段的重复，但不能暴露任意索引访问作为主要 API。字段名应保留在类型系统和错误中。

## 值域规则

### 等级

- 合法范围：`1..=100`。

### 个体值

- 每项合法范围：`0..=31`。
- 六项相互独立。

### 努力值

第三世代按原始 EV 保存范围建模：

- 每项合法范围：`0..=255`。
- 六项总和不能超过 510。
- 公式贡献为 `floor(EV / 4)`。
- 因此每项最多贡献 63 点基础计算值。

不要在核心模型里把单项上限写成现代常用的 252。252 是为了避免不能被 4 整除的浪费，不是第三世代存储上限。

### 性格

第三世代实际能力值包含性格修正。即使当前需求先关注 IV 和 EV，也必须明确性格策略。

第一版应支持：

- `Neutral`：所有非 HP 能力值乘以 `1.0`。
- `Raised(stat) / Lowered(stat)`：一个非 HP 能力值乘以 `1.1`，另一个乘以 `0.9`。

HP 不受性格影响。性格不能提高或降低 HP。

如果暂时不接入完整 25 种性格名称，roster 可以显式使用 `Neutral`。不能省略性格字段后让调用方猜默认值。

## 第三世代公式

所有除法都使用整数向下取整。

HP：

```text
base_part = 2 * base + iv + floor(ev / 4)
max_hp = floor(base_part * level / 100) + level + 10
```

其他五项能力值：

```text
base_part = 2 * base + iv + floor(ev / 4)
before_nature = floor(base_part * level / 100) + 5
final_stat = floor(before_nature * nature_numerator / 100)
```

性格倍率使用整数分子：

- 降低：90。
- 中性：100。
- 提高：110。

不要使用浮点数。计算中间值使用 `u32`，避免未来扩展时出现窄整数溢出。

## 示例

妙蛙种子的种族值是：

```text
HP 45 / 攻击 49 / 防御 49 / 特攻 65 / 特防 65 / 速度 45
```

50 级、全 31 IV、全 0 EV、中性性格的最终能力值应为：

```text
HP 120 / 攻击 69 / 防御 69 / 特攻 85 / 特防 85 / 速度 65
```

当前演示使用的是：

```text
HP 45 / 攻击 49 / 防御 49 / 特攻 65 / 特防 65 / 速度 45
```

这个差异说明切换公式后不能沿用旧的 host 数值断言和对战节奏基线。

## 错误模型

正常输入错误返回结构化错误：

```rust
pub enum StatProjectionError {
    InvalidLevel { value: u8 },
    InvalidIndividualValue { stat: StatName, value: u8 },
    InvalidEffortValue { stat: StatName, value: u16 },
    EffortTotalExceeded { total: u16, max: u16 },
    InvalidNature { raised: StatName, lowered: StatName },
    ZeroBaseStat { stat: StatName },
}
```

虽然单项 EV 的公开构造器可以使用 `u16` 接收输入，但合法值验证后内部存储为 `u8`。这样错误可以准确报告 256 等越界输入，而不是在调用前截断。

## roster 接入方式

真正接入时，为每个 `RosterMember` 增加训练参数：

```rust
struct RosterMember {
    pokemon_form_id: PokemonFormId,
    level: u8,
    move_ids: Vec<MoveId>,
    training: TrainingValues,
}
```

构造顺序：

1. 从 `CurrentDataSet` 查询 `PokemonRecord`。
2. 获取六项种族值。
3. 用等级、IV、EV 和性格计算 `CalculatedStats`。
4. 将 `max_hp` 同时作为初始 HP。
5. 将五项非 HP 结果传给 `BattleStats`。
6. 按现有流程校验属性和招式学习面。

随机演示队伍需要明确训练参数生成策略。建议第一版固定为：

```text
全 31 IV / 全 0 EV / Neutral
```

它可复现、容易验证，也不会引入配点策略。后续需要竞技配点时，再单独设计 EV 模板。

## 分阶段实施

### 阶段 A：纯模型，不改变行为

- 在 `battle-domain` 新增 `StatBlock`、IV、EV、性格和计算函数。
- 导出新 API。
- 增加公式向量、边界和错误测试。
- 不修改 `game-host::roster`。
- 不修改现有 `Pokemon`、`BattleStats` 构造调用。

完成标准：新测试通过，现有全部测试输出保持不变。

### 阶段 B：roster 显式选择训练策略

- 给 `RosterMember` 增加 `TrainingValues`。
- 随机队伍使用固定训练策略。
- 用计算结果替换当前直接使用种族值的逻辑。
- 更新 host 和 E2E 预期。

完成标准：同一 seed 仍生成相同队伍和训练参数；最终能力值符合公式向量；战斗可确定重放。

阶段 B 是行为变更，必须单独执行和验收。

## 测试范围

纯计算测试至少覆盖：

- 1 级和 100 级边界。
- IV 0 和 31。
- EV 0、252 和 255。
- EV 总和 510 和 511。
- EV 除以 4 的向下取整。
- HP 公式与非 HP 公式差异。
- 中性、提高和降低性格。
- 妙蛙种子示例向量。
- 中间计算不使用浮点数。

roster 接入测试至少覆盖：

- 固定训练策略可复现。
- 最终 HP 和五项能力值来自计算器。
- 非法 IV、EV 或性格不能构造队伍。
- 切换能力值后世界到战斗的 E2E 流程仍能结束。
- 对战 seed 相同仍产生相同事件序列。

## 决策门槛

开始阶段 B 前需要明确：

1. 演示队伍是否采用全 31 IV、全 0 EV、中性性格。
2. 是否接受 HP、伤害、速度顺序和战斗回合数变化。
3. 是否需要同时显示等级和训练参数。
4. 旧演示数值是否需要保留为单独的 legacy fixture。

这四项未确认前，只实施阶段 A。
