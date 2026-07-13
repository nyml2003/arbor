use std::{error::Error, fmt, fs, path::PathBuf};

use game_assets::DecodedImage;
use game_host::DemoSpriteManifest;
use game_ui::{atlas_with_battle_sprites, opponent_front_resource, player_back_resource};
use punctum_gpu::{GpuAtlas, ResourceId};

pub fn load_battle_atlas(manifest: &DemoSpriteManifest) -> Result<GpuAtlas, SpriteLoadError> {
    let expected = battle_application::TEAM_SIZE;
    if manifest.player().len() != expected || manifest.opponent().len() != expected {
        return Err(SpriteLoadError::InvalidManifest {
            player: manifest.player().len(),
            opponent: manifest.opponent().len(),
        });
    }

    let mut resources = Vec::with_capacity(expected * 4);
    let mut images = Vec::with_capacity(expected * 4);
    for (slot, form) in manifest.player().iter().enumerate() {
        for frame in 0..2 {
            resources.push(player_back_resource(slot, frame));
            images.push(load_sprite(form.0, SpriteFacing::Back, frame)?);
        }
    }
    for (slot, form) in manifest.opponent().iter().enumerate() {
        for frame in 0..2 {
            resources.push(opponent_front_resource(slot, frame));
            images.push(load_sprite(form.0, SpriteFacing::Front, frame)?);
        }
    }
    let entries = resources
        .into_iter()
        .zip(&images)
        .collect::<Vec<(ResourceId, &DecodedImage)>>();
    atlas_with_battle_sprites(&entries).map_err(|error| SpriteLoadError::Atlas(error.to_string()))
}

#[derive(Clone, Copy)]
enum SpriteFacing {
    Back,
    Front,
}

impl SpriteFacing {
    const fn directory(self) -> &'static str {
        match self {
            Self::Back => "back",
            Self::Front => "front",
        }
    }

    const fn filename_part(self) -> &'static str {
        match self {
            Self::Back => "Back",
            Self::Front => "Front",
        }
    }
}

fn load_sprite(
    pokemon_form_id: u32,
    facing: SpriteFacing,
    frame: usize,
) -> Result<DecodedImage, SpriteLoadError> {
    let path = sprite_root().join(facing.directory()).join(format!(
        "{pokemon_form_id:03}_{}_0_C__frame_{frame}.png",
        facing.filename_part()
    ));
    let bytes = fs::read(&path).map_err(|error| SpriteLoadError::Read {
        path: path.clone(),
        message: error.to_string(),
    })?;
    game_assets::decode_png(&bytes).map_err(|error| SpriteLoadError::Decode {
        path,
        message: error.to_string(),
    })
}

fn sprite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/pokemons/normal")
}

#[derive(Debug)]
pub enum SpriteLoadError {
    InvalidManifest { player: usize, opponent: usize },
    Read { path: PathBuf, message: String },
    Decode { path: PathBuf, message: String },
    Atlas(String),
}

impl fmt::Display for SpriteLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidManifest { player, opponent } => write!(
                formatter,
                "battle sprite manifest requires 6+6 members, received {player}+{opponent}"
            ),
            Self::Read { path, message } => {
                write!(
                    formatter,
                    "failed to read sprite {}: {message}",
                    path.display()
                )
            }
            Self::Decode { path, message } => {
                write!(
                    formatter,
                    "failed to decode sprite {}: {message}",
                    path.display()
                )
            }
            Self::Atlas(message) => {
                write!(formatter, "failed to build battle sprite atlas: {message}")
            }
        }
    }
}

impl Error for SpriteLoadError {}

#[cfg(test)]
mod tests {
    use game_host::DemoGame;
    use game_ui::{opponent_front_resource, player_back_resource};

    use super::load_battle_atlas;

    #[test]
    fn generated_roster_loads_distinct_front_and_back_sprite_resources() {
        for seed in 0..32 {
            let game = DemoGame::new_with_seed(seed).unwrap();
            let manifest = game.sprite_manifest().unwrap();
            let atlas = load_battle_atlas(&manifest).unwrap();

            for slot in 0..battle_application::TEAM_SIZE {
                for frame in 0..2 {
                    assert!(atlas.resource(player_back_resource(slot, frame)).is_some());
                    assert!(
                        atlas
                            .resource(opponent_front_resource(slot, frame))
                            .is_some()
                    );
                }
            }
        }
    }
}
