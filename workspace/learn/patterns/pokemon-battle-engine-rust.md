# 模式：Rust 回合制对战引擎（tdd-demo）

## 一句话

用 Rust 的 struct + enum 构建一个回合制对战引擎——Creature/Move/BattleState 领域模型 + 类型克制系统 + 速度决定先后的回合解析。

## 核心架构

```
BattleState（对局状态）
  ├── player: Creature
  ├── enemy: Creature
  └── log: Vec<String>          ← 事件日志

Creature（生物）
  ├── name, hp, speed
  ├── pokemon_type: PokemonType
  └── moves: Vec<Move>

Move（招式）
  ├── name, power, pp
  └── move_type: PokemonType

回合解析：
  Command → apply_turn() → perform_player_move / perform_enemy_move
                               │
                         resolve_move_use() → type_effectiveness() → calculate_damage()
```

## 关键设计

### 1. 领域类型的 Builder 模式

```rust
impl Creature {
    pub fn with_moves(name, hp, moves) -> Self { ... }
    pub fn with_moves_and_speed(name, hp, speed, moves) -> Self { ... }
    pub fn with_typed_moves_and_speed(name, hp, speed, pokemon_type, moves) -> Self { ... }
}
```

三个构造函数，从简到繁。每个 Builder 链式调用更完整的版本，最终收敛到一个全参数的构造。没有用 Rust 的 builder pattern crate——三个关联函数就够。

### 2. 枚举表示状态空间

```rust
pub enum PokemonType { Normal, Electric, Fire, Water, Ground }

enum Effectiveness { NoEffect, NotVeryEffective, Normal, SuperEffective }

pub enum Command { UseMove(usize), Status, Help, Quit }
```

Command 不是字符串匹配——parse_command 把用户输入映射为枚举变体。`UseMove(usize)` 携带招式索引，类型系统保证了拿到 Command 时已经有合法的索引。

### 3. 回合解析：速度决定先后

```rust
fn use_move(state: &mut BattleState, index: usize) -> String {
    let player_first = state.player.speed >= state.enemy.speed;
    if player_first {
        perform_player_move(state, index, &mut events);
        if !state.enemy.is_defeated() { perform_enemy_move(state, &mut events); }
    } else {
        perform_enemy_move(state, &mut events);
        if !state.player.is_defeated() { perform_player_move(state, index, &mut events); }
    }
}
```

先手判断后的事件顺序：速度快的一方先动。每个动作后检查对方是否已战败——战败方不能行动。这不是一个"你先全部执行完，我再执行"的简单顺序，而是交错执行的。

### 4. 类型克制系统

```rust
fn type_effectiveness(attack: PokemonType, defender: PokemonType) -> Effectiveness {
    match (attack, defender) {
        (Electric, Water)  => SuperEffective,
        (Water, Fire)      => SuperEffective,
        (Fire, Water)      => NotVeryEffective,
        (Electric, Ground) => NoEffect,
        _                  => Normal,
    }
}

fn calculate_damage(power: i32, effectiveness: Effectiveness) -> i32 {
    match effectiveness {
        NoEffect          => 0,
        NotVeryEffective  => (power.max(1) / 2).max(1),
        Normal            => power.max(1),
        SuperEffective    => power.max(1) * 2,
    }
}
```

类型克制是一个纯函数——输入两个类型，输出倍率。没有副作用，没有状态。`damage.min(1)` 保证最小伤害为 1（除了 NoEffect 为 0）。

### 5. 事件日志

```rust
pub struct BattleState {
    pub log: Vec<String>,  // ← 每回合追加一条事件描述
}

// 每条日志包含完整回合描述：
// "Pikachu used Thunderbolt for 40 damage! It's super effective!"
```

不是事后生成的回放——日志是事件发生时同步写入的。`Vec<String>` 是最简单的日志结构，不需要时间戳或结构化字段。

### 6. Command 解析：多对一映射

```rust
pub fn parse_command(input: &str) -> Option<Command> {
    match input.trim().to_ascii_lowercase().as_str() {
        "attack" | "a" | "move1" | "1" => Some(Command::UseMove(0)),
        "move2" | "2"                  => Some(Command::UseMove(1)),
        // ...
        "quit" | "q" | "exit"          => Some(Command::Quit),
        _                              => None,
    }
}
```

多个用户输入映射到同一个 `Command`。解析失败返回 `None` 而不是 panic——调用方自己处理"未知命令"的提示。

## 反模式警示

### ❌ 用字符串做内部状态

不要 `if move_name == "thunderbolt"` 满地都是。把用户输入解析为枚举，后续逻辑只 match 枚举。

### ❌ 回合逻辑和 UI 耦合

`apply_turn` 返回 `String` 而不是直接打印。调用方可以把返回值输出到终端、存日志、或渲染到 GUI——引擎不知道输出目标。

## 来源

- tdd-demo 源码（`src/lib.rs`、`Cargo.toml`）
- 2026-06-07 agent 阅读后提炼
