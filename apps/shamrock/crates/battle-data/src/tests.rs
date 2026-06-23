use crate::{
    ElementType, MoveId, SpeciesId, load_demo_enemy_team, load_demo_player_team, load_gen1_demo_pack,
    type_modifier,
};

#[test]
fn demo_pack_contains_expected_species_and_moves() {
    let pack = load_gen1_demo_pack();
    assert_eq!(pack.species(SpeciesId(0)).name, "Pikachu");
    assert_eq!(pack.move_def(MoveId(3)).name, "Ember");
    assert_eq!(pack.move_def(MoveId(13)).name, "Slam");
}

#[test]
fn demo_teams_have_matching_non_empty_sizes() {
    let player = load_demo_player_team();
    let enemy = load_demo_enemy_team();

    assert!(!player.members.is_empty());
    assert_eq!(player.members.len(), enemy.members.len());
}

#[test]
fn type_chart_handles_core_matchups() {
    assert_eq!(type_modifier(ElementType::Electric, ElementType::Water), 200);
    assert_eq!(type_modifier(ElementType::Electric, ElementType::Grass), 50);
    assert_eq!(type_modifier(ElementType::Water, ElementType::Fire), 200);
    assert_eq!(type_modifier(ElementType::Fire, ElementType::Water), 50);
}
