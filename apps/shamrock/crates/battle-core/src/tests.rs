use battle_data::{
    BaseStats, DataPack, EffectTarget, ElementType, MoveDef, MoveEffect, MoveId, PokemonTemplate, SpeciesDef, SpeciesId, StatId,
    StatusCondition, TeamTemplate, WeatherKind,
};

use super::{BattleAction, BattleInit, Request, RngState, SideId, WeatherState, initialize_battle, requested_side, step};

fn test_data_pack() -> DataPack {
    DataPack::new(
        "test-pack",
        vec![
            SpeciesDef {
                id: SpeciesId(0),
                name: "Fastmon".to_string(),
                primary_type: ElementType::Electric,
                secondary_type: None,
                stats: BaseStats { hp: 120, attack: 55, defense: 40, speed: 90 },
            },
            SpeciesDef {
                id: SpeciesId(1),
                name: "Midmon".to_string(),
                primary_type: ElementType::Fire,
                secondary_type: None,
                stats: BaseStats { hp: 120, attack: 52, defense: 43, speed: 65 },
            },
            SpeciesDef {
                id: SpeciesId(2),
                name: "Tankmon".to_string(),
                primary_type: ElementType::Water,
                secondary_type: None,
                stats: BaseStats { hp: 150, attack: 48, defense: 65, speed: 43 },
            },
            SpeciesDef {
                id: SpeciesId(3),
                name: "Leafmon".to_string(),
                primary_type: ElementType::Grass,
                secondary_type: None,
                stats: BaseStats { hp: 130, attack: 49, defense: 49, speed: 45 },
            },
        ],
        vec![
            MoveDef {
                id: MoveId(0),
                name: "Tackle".to_string(),
                element_type: ElementType::Normal,
                power: 40,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::Damage,
            },
            MoveDef {
                id: MoveId(1),
                name: "Quick Attack".to_string(),
                element_type: ElementType::Normal,
                power: 40,
                accuracy: 100,
                priority: 1,
                effect: MoveEffect::Damage,
            },
            MoveDef {
                id: MoveId(2),
                name: "Ember".to_string(),
                element_type: ElementType::Fire,
                power: 40,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::Damage,
            },
            MoveDef {
                id: MoveId(3),
                name: "Growl".to_string(),
                element_type: ElementType::Normal,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::ModifyStat { target: EffectTarget::Opponent, stat: StatId::Attack, stages: -1 },
            },
            MoveDef {
                id: MoveId(4),
                name: "Thunder Wave".to_string(),
                element_type: ElementType::Electric,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::ApplyStatus { target: EffectTarget::Opponent, status: StatusCondition::Paralyzed },
            },
            MoveDef {
                id: MoveId(5),
                name: "Poison Powder".to_string(),
                element_type: ElementType::Grass,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::ApplyStatus { target: EffectTarget::Opponent, status: StatusCondition::Poisoned },
            },
            MoveDef {
                id: MoveId(6),
                name: "Recover".to_string(),
                element_type: ElementType::Normal,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::HealPercent { target: EffectTarget::User, percent: 50 },
            },
            MoveDef {
                id: MoveId(7),
                name: "Sunny Day".to_string(),
                element_type: ElementType::Fire,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::SetWeather { weather: WeatherKind::Sunny, turns: 3 },
            },
            MoveDef {
                id: MoveId(8),
                name: "Rain Dance".to_string(),
                element_type: ElementType::Water,
                power: 0,
                accuracy: 100,
                priority: 0,
                effect: MoveEffect::SetWeather { weather: WeatherKind::Rainy, turns: 3 },
            },
            MoveDef {
                id: MoveId(9),
                name: "Roar".to_string(),
                element_type: ElementType::Normal,
                power: 0,
                accuracy: 100,
                priority: 1,
                effect: MoveEffect::ForceSwitch { target: EffectTarget::Opponent },
            },
            MoveDef {
                id: MoveId(10),
                name: "Slam".to_string(),
                element_type: ElementType::Normal,
                power: 55,
                accuracy: 85,
                priority: 0,
                effect: MoveEffect::Damage,
            },
        ],
    )
}

fn single_member_team(nickname: &'static str, species: SpeciesId, moves: [MoveId; 4]) -> TeamTemplate {
    TeamTemplate {
        members: vec![PokemonTemplate {
            nickname: nickname.to_string(),
            species,
            moves,
            level: None,
            individual_values: None,
            effort_values: None,
            nature: None,
        }],
    }
}

fn basic_init() -> BattleInit {
    BattleInit {
        player: single_member_team("Fasty", SpeciesId(0), [MoveId(0), MoveId(1), MoveId(4), MoveId(0)]),
        opponent: single_member_team("Blaze", SpeciesId(1), [MoveId(2), MoveId(3), MoveId(0), MoveId(0)]),
    }
}

#[test]
fn battle_starts_waiting_for_player_choice() {
    let data = test_data_pack();
    let state = initialize_battle(basic_init(), &data).unwrap();
    assert_eq!(requested_side(&state), Request::ChooseAction { side: SideId::Player });
}

#[test]
fn first_choice_only_advances_request() {
    let data = test_data_pack();
    let state = initialize_battle(basic_init(), &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(7), &data).unwrap();

    assert_eq!(result.next_request, Request::ChooseAction { side: SideId::Opponent });
    assert_eq!(result.state.pending[0], Some(BattleAction::UseMove(0)));
}

#[test]
fn turn_resolution_deals_damage_and_advances_turn() {
    let data = test_data_pack();
    let state = initialize_battle(basic_init(), &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(3), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();

    assert_eq!(result.state.turn, 2);
    assert!(result.state.teams[0].party[0].current_hp < result.state.teams[0].party[0].max_hp);
    assert!(result.state.teams[1].party[0].current_hp < result.state.teams[1].party[0].max_hp);
}

#[test]
fn quick_attack_resolves_before_normal_priority_move() {
    let data = test_data_pack();
    let state = initialize_battle(basic_init(), &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(1), RngState::seeded(1), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();

    assert!(matches!(result.log.trace.first(), Some(super::TraceEvent::ChoiceAccepted { .. })));
    assert!(matches!(
        result.log.trace.get(1),
        Some(super::TraceEvent::MoveOrderCalculated { first: SideId::Player, second: SideId::Opponent })
    ));
}

#[test]
fn same_seed_and_actions_produce_same_result() {
    let data = test_data_pack();
    let init = basic_init();
    let first = initialize_battle(init.clone(), &data).unwrap();
    let second = initialize_battle(init, &data).unwrap();

    let first = step(first, SideId::Player, BattleAction::UseMove(0), RngState::seeded(99), &data).unwrap();
    let first = step(first.state, SideId::Opponent, BattleAction::UseMove(0), first.rng, &data).unwrap();

    let second = step(second, SideId::Player, BattleAction::UseMove(0), RngState::seeded(99), &data).unwrap();
    let second = step(second.state, SideId::Opponent, BattleAction::UseMove(0), second.rng, &data).unwrap();

    assert_eq!(first.state, second.state);
    assert_eq!(first.log, second.log);
}

#[test]
fn growl_reduces_following_attack_damage() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Tanky", SpeciesId(2), [MoveId(3), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Blaze", SpeciesId(1), [MoveId(0), MoveId(0), MoveId(0), MoveId(0)]),
    };

    let with_growl = initialize_battle(init.clone(), &data).unwrap();
    let with_growl = step(with_growl, SideId::Player, BattleAction::UseMove(0), RngState::seeded(11), &data).unwrap();
    let with_growl = step(with_growl.state, SideId::Opponent, BattleAction::UseMove(0), with_growl.rng, &data).unwrap();
    assert_eq!(with_growl.state.teams[1].party[0].stages.attack, -1);

    let with_growl = step(with_growl.state, SideId::Player, BattleAction::UseMove(1), with_growl.rng, &data).unwrap();
    let with_growl = step(with_growl.state, SideId::Opponent, BattleAction::UseMove(0), with_growl.rng, &data).unwrap();

    let without_growl = initialize_battle(init, &data).unwrap();
    let without_growl = step(without_growl, SideId::Player, BattleAction::UseMove(1), RngState::seeded(11), &data).unwrap();
    let without_growl = step(without_growl.state, SideId::Opponent, BattleAction::UseMove(0), without_growl.rng, &data).unwrap();
    let without_growl = step(without_growl.state, SideId::Player, BattleAction::UseMove(1), without_growl.rng, &data).unwrap();
    let without_growl = step(without_growl.state, SideId::Opponent, BattleAction::UseMove(0), without_growl.rng, &data).unwrap();

    assert!(with_growl.state.teams[0].party[0].current_hp > without_growl.state.teams[0].party[0].current_hp);
}

#[test]
fn paralysis_changes_turn_order_on_next_turn() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(4), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Fasty", SpeciesId(0), [MoveId(3), MoveId(0), MoveId(0), MoveId(0)]),
    };

    let state = initialize_battle(init, &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(31), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();
    assert_eq!(result.state.teams[1].party[0].status, Some(StatusCondition::Paralyzed));

    let result = step(result.state, SideId::Player, BattleAction::UseMove(1), result.rng, &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(1), result.rng, &data).unwrap();

    assert!(matches!(
        result.log.trace.get(1),
        Some(super::TraceEvent::MoveOrderCalculated { first: SideId::Player, second: SideId::Opponent })
    ));
}

#[test]
fn poison_applies_residual_damage_at_end_of_turn() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Leafy", SpeciesId(3), [MoveId(5), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Tanky", SpeciesId(2), [MoveId(0), MoveId(0), MoveId(0), MoveId(0)]),
    };

    let state = initialize_battle(init, &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(7), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();

    assert_eq!(result.state.teams[1].party[0].status, Some(StatusCondition::Poisoned));
    assert_eq!(result.state.teams[1].party[0].current_hp, 132);
    assert!(result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::ResidualDamage { target: SideId::Opponent, .. })));
}

#[test]
fn recover_heals_up_to_max_hp() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Tanky", SpeciesId(2), [MoveId(6), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Leafy", SpeciesId(3), [MoveId(3), MoveId(0), MoveId(0), MoveId(0)]),
    };

    let mut state = initialize_battle(init, &data).unwrap();
    state.teams[0].party[0].current_hp = 20;

    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(19), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();
    let healed_hp = result.state.teams[0].party[0].current_hp;

    assert!(healed_hp > 20);
    assert!(healed_hp <= result.state.teams[0].party[0].max_hp);
    assert!(result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::Healed { side: SideId::Player, .. })));
}

#[test]
fn sunny_weather_boosts_fire_damage_and_expires() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(7), MoveId(2), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Tanky", SpeciesId(2), [MoveId(3), MoveId(3), MoveId(3), MoveId(3)]),
    };

    let sunny = initialize_battle(init.clone(), &data).unwrap();
    let sunny = step(sunny, SideId::Player, BattleAction::UseMove(0), RngState::seeded(5), &data).unwrap();
    let sunny = step(sunny.state, SideId::Opponent, BattleAction::UseMove(0), sunny.rng, &data).unwrap();
    assert_eq!(sunny.state.weather, Some(WeatherState { kind: WeatherKind::Sunny, remaining_turns: 2 }));
    assert!(sunny
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::WeatherStarted { weather: WeatherKind::Sunny, .. })));

    let sunny = step(sunny.state, SideId::Player, BattleAction::UseMove(1), sunny.rng, &data).unwrap();
    let sunny = step(sunny.state, SideId::Opponent, BattleAction::UseMove(0), sunny.rng, &data).unwrap();
    let sunny_damage = sunny
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    let plain = initialize_battle(init, &data).unwrap();
    let plain = step(plain, SideId::Player, BattleAction::UseMove(1), RngState::seeded(5), &data).unwrap();
    let plain = step(plain.state, SideId::Opponent, BattleAction::UseMove(0), plain.rng, &data).unwrap();
    let plain_damage = plain
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    assert!(sunny_damage > plain_damage);

    let sunny = step(sunny.state, SideId::Player, BattleAction::UseMove(2), sunny.rng, &data).unwrap();
    let sunny = step(sunny.state, SideId::Opponent, BattleAction::UseMove(1), sunny.rng, &data).unwrap();
    assert_eq!(sunny.state.weather, None);
    assert!(sunny
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::WeatherEnded { weather: WeatherKind::Sunny })));
}

#[test]
fn rain_weather_boosts_water_damage() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(8), MoveId(2), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Tanky", SpeciesId(2), [MoveId(3), MoveId(3), MoveId(3), MoveId(3)]),
    };

    let rainy = initialize_battle(init.clone(), &data).unwrap();
    let rainy = step(rainy, SideId::Player, BattleAction::UseMove(0), RngState::seeded(17), &data).unwrap();
    let rainy = step(rainy.state, SideId::Opponent, BattleAction::UseMove(0), rainy.rng, &data).unwrap();
    let rainy = step(rainy.state, SideId::Player, BattleAction::UseMove(1), rainy.rng, &data).unwrap();
    let rainy = step(rainy.state, SideId::Opponent, BattleAction::UseMove(0), rainy.rng, &data).unwrap();
    let rainy_damage = rainy
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    let plain = initialize_battle(init, &data).unwrap();
    let plain = step(plain, SideId::Player, BattleAction::UseMove(1), RngState::seeded(17), &data).unwrap();
    let plain = step(plain.state, SideId::Opponent, BattleAction::UseMove(0), plain.rng, &data).unwrap();
    let plain_damage = plain
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    assert!(rainy_damage < plain_damage);
}

#[test]
fn resisted_type_deals_less_damage_than_neutral_move() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(2), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Tanky", SpeciesId(2), [MoveId(3), MoveId(3), MoveId(3), MoveId(3)]),
    };

    let resisted = initialize_battle(init.clone(), &data).unwrap();
    let resisted = step(resisted, SideId::Player, BattleAction::UseMove(0), RngState::seeded(5), &data).unwrap();
    let resisted = step(resisted.state, SideId::Opponent, BattleAction::UseMove(0), resisted.rng, &data).unwrap();
    let resisted_damage = resisted
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    let neutral = initialize_battle(init, &data).unwrap();
    let neutral = step(neutral, SideId::Player, BattleAction::UseMove(1), RngState::seeded(5), &data).unwrap();
    let neutral = step(neutral.state, SideId::Opponent, BattleAction::UseMove(0), neutral.rng, &data).unwrap();
    let neutral_damage = neutral
        .log
        .domain
        .iter()
        .find_map(|event| match event {
            super::DomainEvent::DamageDealt { side: SideId::Player, amount, .. } => Some(*amount),
            _ => None,
        })
        .unwrap();

    assert!(resisted_damage < neutral_damage);
}

#[test]
fn low_accuracy_move_can_miss_without_dealing_damage() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(10), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: single_member_team("Tanky", SpeciesId(2), [MoveId(3), MoveId(3), MoveId(3), MoveId(3)]),
    };

    let state = initialize_battle(init, &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(17), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();

    assert_eq!(result.state.teams[1].party[0].current_hp, result.state.teams[1].party[0].max_hp);
    assert!(result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::MoveMissed { side: SideId::Player, move_id: MoveId(10) })));
    assert!(result
        .log
        .trace
        .iter()
        .any(|event| matches!(event, super::TraceEvent::AccuracyRolled { side: SideId::Player, roll: 87, needed: 85 })));
}

#[test]
fn roar_forces_target_to_switch_and_cancels_their_action() {
    let data = test_data_pack();
    let init = BattleInit {
        player: single_member_team("Blaze", SpeciesId(1), [MoveId(9), MoveId(0), MoveId(0), MoveId(0)]),
        opponent: TeamTemplate {
            members: vec![
                PokemonTemplate {
                    nickname: "Lead".to_string(),
                    species: SpeciesId(0),
                    moves: [MoveId(0), MoveId(0), MoveId(0), MoveId(0)],
                    level: None,
                    individual_values: None,
                    effort_values: None,
                    nature: None,
                },
                PokemonTemplate {
                    nickname: "Bench".to_string(),
                    species: SpeciesId(2),
                    moves: [MoveId(0), MoveId(0), MoveId(0), MoveId(0)],
                    level: None,
                    individual_values: None,
                    effort_values: None,
                    nature: None,
                },
            ],
        },
    };

    let state = initialize_battle(init, &data).unwrap();
    let result = step(state, SideId::Player, BattleAction::UseMove(0), RngState::seeded(41), &data).unwrap();
    let result = step(result.state, SideId::Opponent, BattleAction::UseMove(0), result.rng, &data).unwrap();

    assert_eq!(result.state.teams[SideId::Opponent.index()].active, 1);
    assert!(result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::ForcedSwitch { side: SideId::Opponent })));
    assert!(result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::PokemonSwitched { side: SideId::Opponent, slot: 1 })));
    assert!(!result
        .log
        .domain
        .iter()
        .any(|event| matches!(event, super::DomainEvent::MoveUsed { side: SideId::Opponent, .. })));
}
