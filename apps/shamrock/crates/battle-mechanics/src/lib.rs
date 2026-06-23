use battle_data::{
    EffortValues, ElementType, IndividualValues, Nature, SpeciesBaseStats, StatusCondition,
    WeatherKind, type_modifier,
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct BattleStats {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DamageInput {
    pub power: u16,
    pub attack: u16,
    pub attack_stage: i8,
    pub defense: u16,
    pub defense_stage: i8,
    pub stab: bool,
    pub move_type: ElementType,
    pub defender_primary: ElementType,
    pub defender_secondary: Option<ElementType>,
    pub weather: Option<WeatherKind>,
    pub variance: u16,
}

pub fn resolve_battle_stats(
    base: SpeciesBaseStats,
    individual_values: Option<IndividualValues>,
    effort_values: Option<EffortValues>,
    nature: Option<Nature>,
    level: Option<u8>,
) -> BattleStats {
    let Some(level) = level else {
        return BattleStats {
            hp: base.hp,
            attack: base.attack,
            defense: base.defense,
            speed: base.speed,
        };
    };

    let ivs = individual_values.unwrap_or_default();
    let evs = effort_values.unwrap_or_default();
    let nature = nature.unwrap_or_default();
    let level = u32::from(level.max(1));
    let apply_nature = |value: u16, boosted: bool, lowered: bool| -> u16 {
        if boosted {
            (u32::from(value) * 110 / 100).max(1) as u16
        } else if lowered {
            (u32::from(value) * 90 / 100).max(1) as u16
        } else {
            value
        }
    };
    let attack = (((u32::from(base.attack) * 2
        + u32::from(ivs.attack)
        + u32::from(evs.attack / 4))
        * level)
        / 100
        + 5)
        .max(1) as u16;
    let defense = (((u32::from(base.defense) * 2
        + u32::from(ivs.defense)
        + u32::from(evs.defense / 4))
        * level)
        / 100
        + 5)
        .max(1) as u16;
    let speed = (((u32::from(base.speed) * 2
        + u32::from(ivs.speed)
        + u32::from(evs.speed / 4))
        * level)
        / 100
        + 5)
        .max(1) as u16;
    BattleStats {
        hp: (((u32::from(base.hp) * 2 + u32::from(ivs.hp) + u32::from(evs.hp / 4)) * level)
            / 100
            + level
            + 10)
            .max(1) as u16,
        attack: apply_nature(attack, nature == Nature::Adamant, nature == Nature::Bold),
        defense: apply_nature(defense, nature == Nature::Bold, false),
        speed: apply_nature(speed, nature == Nature::Timid, nature == Nature::Adamant),
    }
}

pub fn compare_action_order(
    left_priority: i8,
    right_priority: i8,
    left_speed: u16,
    right_speed: u16,
    left_wins_tie: bool,
) -> i32 {
    if left_priority != right_priority {
        return right_priority as i32 - left_priority as i32;
    }

    if left_speed != right_speed {
        return right_speed as i32 - left_speed as i32;
    }

    if left_wins_tie { -1 } else { 1 }
}

pub fn switch_priority() -> i8 {
    6
}

pub fn apply_stage(base: u16, stage: i8) -> u16 {
    if stage >= 0 {
        base.saturating_mul((2 + stage) as u16) / 2
    } else {
        base.saturating_mul(2) / (2 + (-stage) as u16)
    }
}

pub fn effective_speed(base_speed: u16, speed_stage: i8, status: Option<StatusCondition>) -> u16 {
    let staged_speed = apply_stage(base_speed, speed_stage);
    if status == Some(StatusCondition::Paralyzed) {
        (staged_speed / 2).max(1)
    } else {
        staged_speed
    }
}

pub fn weather_damage_multiplier(weather: Option<WeatherKind>, move_type: ElementType) -> (i32, i32) {
    match weather {
        Some(WeatherKind::Sunny) if move_type == ElementType::Fire => (3, 2),
        Some(WeatherKind::Sunny) if move_type == ElementType::Water => (1, 2),
        Some(WeatherKind::Rainy) if move_type == ElementType::Water => (3, 2),
        Some(WeatherKind::Rainy) if move_type == ElementType::Fire => (1, 2),
        _ => (1, 1),
    }
}

pub fn calculate_damage(input: DamageInput) -> u16 {
    let attack = apply_stage(input.attack, input.attack_stage);
    let defense = apply_stage(input.defense, input.defense_stage);

    let mut damage = i32::from(input.power) + i32::from(attack / 4) - i32::from(defense / 8);
    damage = damage.max(1);

    if input.stab {
        damage = damage * 3 / 2;
    }

    damage = damage * i32::from(type_modifier(input.move_type, input.defender_primary)) / 100;
    if let Some(extra_type) = input.defender_secondary {
        damage = damage * i32::from(type_modifier(input.move_type, extra_type)) / 100;
    }

    let (weather_num, weather_den) = weather_damage_multiplier(input.weather, input.move_type);
    damage = damage * weather_num / weather_den;
    damage = (damage * i32::from(input.variance) / 100).max(1);

    damage as u16
}

pub fn residual_status_damage(status: StatusCondition, max_hp: i32) -> Option<u16> {
    match status {
        StatusCondition::Poisoned => Some(((max_hp.max(1) as u16) / 8).max(1)),
        StatusCondition::Paralyzed => None,
    }
}

#[cfg(test)]
mod tests {
    use battle_data::{
        EffortValues, ElementType, IndividualValues, Nature, SpeciesBaseStats, StatusCondition,
        WeatherKind,
    };

    use super::{
        BattleStats, DamageInput, calculate_damage, compare_action_order, effective_speed,
        residual_status_damage, resolve_battle_stats, switch_priority,
    };

    #[test]
    fn resolve_battle_stats_uses_legacy_values_when_level_is_absent() {
        let stats = resolve_battle_stats(
            SpeciesBaseStats {
                hp: 120,
                attack: 55,
                defense: 40,
                speed: 90,
            },
            None,
            None,
            None,
            None,
        );

        assert_eq!(
            stats,
            BattleStats {
                hp: 120,
                attack: 55,
                defense: 40,
                speed: 90,
            }
        );
    }

    #[test]
    fn resolve_battle_stats_applies_level_and_ivs_when_present() {
        let stats = resolve_battle_stats(
            SpeciesBaseStats {
                hp: 35,
                attack: 55,
                defense: 30,
                speed: 90,
            },
            Some(IndividualValues {
                hp: 15,
                attack: 10,
                defense: 12,
                speed: 15,
            }),
            Some(EffortValues {
                hp: 252,
                attack: 252,
                defense: 0,
                speed: 4,
            }),
            Some(Nature::Adamant),
            Some(50),
        );

        assert!(stats.hp > 35);
        assert!(stats.attack > 55);
        assert!(stats.speed > 90 / 2);
    }

    #[test]
    fn resolve_battle_stats_applies_nature_modifiers() {
        let neutral = resolve_battle_stats(
            SpeciesBaseStats {
                hp: 35,
                attack: 55,
                defense: 30,
                speed: 90,
            },
            Some(IndividualValues::default()),
            Some(EffortValues::default()),
            Some(Nature::Neutral),
            Some(50),
        );
        let adamant = resolve_battle_stats(
            SpeciesBaseStats {
                hp: 35,
                attack: 55,
                defense: 30,
                speed: 90,
            },
            Some(IndividualValues::default()),
            Some(EffortValues::default()),
            Some(Nature::Adamant),
            Some(50),
        );
        let timid = resolve_battle_stats(
            SpeciesBaseStats {
                hp: 35,
                attack: 55,
                defense: 30,
                speed: 90,
            },
            Some(IndividualValues::default()),
            Some(EffortValues::default()),
            Some(Nature::Timid),
            Some(50),
        );

        assert!(adamant.attack > neutral.attack);
        assert!(adamant.speed < neutral.speed);
        assert!(timid.speed > neutral.speed);
        assert_eq!(timid.defense, neutral.defense);
    }

    #[test]
    fn switch_priority_stays_above_normal_moves() {
        assert!(switch_priority() > 0);
    }

    #[test]
    fn compare_action_order_prefers_higher_priority_then_speed() {
        assert!(compare_action_order(1, 0, 10, 100, true) < 0);
        assert!(compare_action_order(0, 0, 120, 100, true) < 0);
    }

    #[test]
    fn effective_speed_applies_paralysis_penalty() {
        assert_eq!(effective_speed(100, 0, Some(StatusCondition::Paralyzed)), 50);
    }

    #[test]
    fn damage_calculation_respects_resistance_and_weather() {
        let plain = calculate_damage(DamageInput {
            power: 40,
            attack: 52,
            attack_stage: 0,
            defense: 65,
            defense_stage: 0,
            stab: true,
            move_type: ElementType::Fire,
            defender_primary: ElementType::Water,
            defender_secondary: None,
            weather: None,
            variance: 100,
        });
        let sunny = calculate_damage(DamageInput {
            weather: Some(WeatherKind::Sunny),
            ..DamageInput {
                power: 40,
                attack: 52,
                attack_stage: 0,
                defense: 65,
                defense_stage: 0,
                stab: true,
                move_type: ElementType::Fire,
                defender_primary: ElementType::Water,
                defender_secondary: None,
                weather: None,
                variance: 100,
            }
        });

        assert!(plain > 0);
        assert!(sunny > plain);
    }

    #[test]
    fn residual_status_damage_only_applies_to_poison_for_now() {
        assert_eq!(residual_status_damage(StatusCondition::Poisoned, 120), Some(15));
        assert_eq!(residual_status_damage(StatusCondition::Paralyzed, 120), None);
    }
}
