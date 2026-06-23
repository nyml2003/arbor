use crate::TeamTemplate;
use crate::pack::DataPack;

pub fn load_gen1_demo_pack() -> DataPack {
    DataPack::from_json_str(include_str!("../../../packs/gen1-demo/data-pack.json"))
        .expect("bundled gen1 demo data pack must be valid")
}

pub fn load_demo_player_team() -> TeamTemplate {
    serde_json::from_str(include_str!("../../../packs/gen1-demo/player-team.json"))
        .expect("bundled player team must be valid")
}

pub fn load_demo_enemy_team() -> TeamTemplate {
    serde_json::from_str(include_str!("../../../packs/gen1-demo/opponent-team.json"))
        .expect("bundled opponent team must be valid")
}
