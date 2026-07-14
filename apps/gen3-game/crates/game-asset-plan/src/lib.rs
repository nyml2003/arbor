//! Pure planning and assembly of the game's native assets.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use battle_application::{MoveCategory, PokemonType, TEAM_SIZE};
use game_assets::{AssetKey, DecodedImage, decode_png};
use game_native_plan::NativeAssets;
use game_session::DemoSpriteManifest;
use game_ui::WorldAnimation;
use game_view::{
    move_category_icon_asset, opponent_front_asset, pill_ui_asset, player_back_asset,
    pokemon_icon_asset, rounded_ui_asset, type_icon_asset, world_character_asset,
};
use punctum_gpu::{PixelSize, Rgba8};
use world_application::Direction;

const CHARACTER_FILES: [[&str; 6]; 4] = [
    [
        "down_stand.png",
        "down_walk_2.png",
        "down_walk_3.png",
        "down_run_1.png",
        "down_run_2.png",
        "down_runn_3.png",
    ],
    [
        "left_stand.png",
        "left_walk_1.png",
        "left_walk_2.png",
        "left_run_1.png",
        "left_run_2.png",
        "left_run_3.png",
    ],
    [
        "right_stand.png",
        "right_walk_1.png",
        "right_walk_2.png",
        "right_run_1.png",
        "right_run_2.png",
        "right_run_3.png",
    ],
    [
        "up_stand.png",
        "up_walk_1.png",
        "up_walk_2.png",
        "up_run_1.png",
        "up_run_2.png",
        "up_run_3.png",
    ],
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetRequest {
    pub key: AssetKey,
    pub relative_path: String,
    pub expected_size: Option<PixelSize>,
}

pub struct AssetBytes {
    pub request: AssetRequest,
    pub bytes: Vec<u8>,
}

pub fn asset_requests(manifest: &DemoSpriteManifest) -> Vec<AssetRequest> {
    debug_assert_eq!(manifest.player().len(), TEAM_SIZE);
    debug_assert_eq!(manifest.opponent().len(), TEAM_SIZE);
    let mut requests = character_requests();
    for (slot, form) in manifest.player().iter().enumerate() {
        for frame in 0..2 {
            requests.push(AssetRequest {
                key: player_back_asset(slot, frame),
                relative_path: format!(
                    "pokemons/normal/back/{:03}_Back_0_C__frame_{frame}.png",
                    form.0
                ),
                expected_size: None,
            });
            requests.push(AssetRequest {
                key: pokemon_icon_asset(slot, frame),
                relative_path: format!("pokemons/icons/{:03}_{}.png", form.0, frame % 2),
                expected_size: Some(PixelSize::new(32, 32)),
            });
        }
    }
    for (slot, form) in manifest.opponent().iter().enumerate() {
        for frame in 0..2 {
            requests.push(AssetRequest {
                key: opponent_front_asset(slot, frame),
                relative_path: format!(
                    "pokemons/normal/front/{:03}_Front_0_C__frame_{frame}.png",
                    form.0
                ),
                expected_size: None,
            });
        }
    }
    requests.extend(type_icon_requests());
    requests.extend(move_category_requests());
    requests
}

fn character_requests() -> Vec<AssetRequest> {
    let directions = [
        Direction::Down,
        Direction::Left,
        Direction::Right,
        Direction::Up,
    ];
    let mut requests = Vec::with_capacity(24);
    for (direction_index, files) in CHARACTER_FILES.iter().enumerate() {
        for (frame, file) in files.iter().enumerate() {
            requests.push(AssetRequest {
                key: world_character_asset(
                    directions[direction_index],
                    frame_animation(frame),
                    frame_index(frame),
                ),
                relative_path: format!("characters/red/actions/group-00/{file}"),
                expected_size: None,
            });
        }
    }
    requests
}

fn frame_animation(frame: usize) -> WorldAnimation {
    match frame {
        0 => WorldAnimation::Stand,
        1 | 2 => WorldAnimation::Walk,
        _ => WorldAnimation::Run,
    }
}

const fn frame_index(frame: usize) -> usize {
    match frame {
        0 => 0,
        1 => 0,
        2 => 2,
        3 => 1,
        4 => 0,
        _ => 2,
    }
}

fn type_icon_requests() -> Vec<AssetRequest> {
    let types = [
        PokemonType::Normal,
        PokemonType::Fighting,
        PokemonType::Flying,
        PokemonType::Poison,
        PokemonType::Ground,
        PokemonType::Rock,
        PokemonType::Bug,
        PokemonType::Ghost,
        PokemonType::Steel,
        PokemonType::Fire,
        PokemonType::Water,
        PokemonType::Grass,
        PokemonType::Electric,
        PokemonType::Psychic,
        PokemonType::Ice,
        PokemonType::Dragon,
        PokemonType::Dark,
    ];
    let icon_numbers = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 16, 17];
    types
        .into_iter()
        .zip(icon_numbers)
        .map(|(pokemon_type, number)| AssetRequest {
            key: type_icon_asset(pokemon_type),
            relative_path: format!("type-icons/icon-{number:02}.png"),
            expected_size: None,
        })
        .collect()
}

fn move_category_requests() -> Vec<AssetRequest> {
    [
        (MoveCategory::Physical, "physical.png"),
        (MoveCategory::Special, "special.png"),
        (MoveCategory::Status, "status.png"),
    ]
    .into_iter()
    .map(|(category, file)| AssetRequest {
        key: move_category_icon_asset(category),
        relative_path: format!("move-category-icons/{file}"),
        expected_size: None,
    })
    .collect()
}

pub fn assemble_assets(
    sources: Vec<AssetBytes>,
    map_images: Vec<(AssetKey, DecodedImage)>,
) -> Result<NativeAssets, GameAssetError> {
    let mut images = vec![(
        AssetKey::new("solid/white").expect("the white asset key is valid"),
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    images.push((rounded_ui_asset(), rounded_mask(64, 64, 6)));
    images.push((pill_ui_asset(), rounded_mask(128, 64, 32)));
    for source in sources {
        let image = decode_png(&source.bytes).map_err(|error| GameAssetError::Decode {
            path: source.request.relative_path.clone(),
            message: error.to_string(),
        })?;
        if source
            .request
            .expected_size
            .is_some_and(|expected| image.size() != expected)
        {
            return Err(GameAssetError::WrongSize {
                path: source.request.relative_path,
                expected: source.request.expected_size.expect("the size was checked"),
                actual: image.size(),
            });
        }
        images.push((source.request.key, image));
    }
    images.extend(map_images);
    NativeAssets::new(images).map_err(|error| GameAssetError::Assets(error.to_string()))
}

fn rounded_mask(width: u32, height: u32, radius: u32) -> DecodedImage {
    let mut rgba8 = Vec::with_capacity((width * height * 4) as usize);
    let radius = radius as f32;
    let half_width = width as f32 / 2.0;
    let half_height = height as f32 / 2.0;
    let inner_x = half_width - radius;
    let inner_y = half_height - radius;
    for y in 0..height {
        for x in 0..width {
            let dx = ((x as f32 + 0.5) - half_width).abs() - inner_x;
            let dy = ((y as f32 + 0.5) - half_height).abs() - inner_y;
            let outside = dx.max(0.0).hypot(dy.max(0.0));
            let inside = dx.max(dy).min(0.0);
            let distance = outside + inside - radius;
            let alpha = ((0.5 - distance).clamp(0.0, 1.0) * 255.0).round() as u8;
            rgba8.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    DecodedImage::from_rgba8(PixelSize::new(width, height), rgba8)
        .expect("generated UI masks have a complete RGBA8 payload")
}

#[derive(Debug)]
pub enum GameAssetError {
    Decode {
        path: String,
        message: String,
    },
    WrongSize {
        path: String,
        expected: PixelSize,
        actual: PixelSize,
    },
    Assets(String),
}

impl fmt::Display for GameAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode { path, message } => {
                write!(formatter, "failed to decode sprite {path}: {message}")
            }
            Self::WrongSize {
                path,
                expected,
                actual,
            } => write!(
                formatter,
                "sprite {path} must be {}x{} pixels, received {}x{}",
                expected.width, expected.height, actual.width, actual.height
            ),
            Self::Assets(message) => write!(formatter, "failed to build game assets: {message}"),
        }
    }
}

impl Error for GameAssetError {}

#[cfg(test)]
mod tests {
    use game_data::CurrentDataSet;
    use game_session::GameSession;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    use super::*;

    fn manifest() -> DemoSpriteManifest {
        GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17)
            .unwrap()
            .sprite_manifest()
            .unwrap()
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                &vec![255; (width * height * 4) as usize],
                width,
                height,
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    #[test]
    fn manifest_expands_to_stable_file_and_key_requests() {
        let requests = asset_requests(&manifest());
        assert_eq!(requests.len(), 80);
        assert_eq!(
            requests[0].relative_path,
            "characters/red/actions/group-00/down_stand.png"
        );
        assert_eq!(requests[0].key.as_str(), "character/0/0");
        assert!(requests.iter().any(|request| {
            request.relative_path == "move-category-icons/status.png"
                && request.key.as_str() == "battle/move-category/status"
        }));
    }

    #[test]
    fn assembly_decodes_and_validates_declared_sizes() {
        let requests = asset_requests(&manifest());
        let sources = requests
            .into_iter()
            .map(|request| AssetBytes {
                bytes: png(
                    request.expected_size.map_or(1, |size| size.width),
                    request.expected_size.map_or(1, |size| size.height),
                ),
                request,
            })
            .collect();
        let assets = assemble_assets(sources, Vec::new()).unwrap();
        assert!(assets.atlas_size().width > 0);
        assert!(assets.resource(&rounded_ui_asset()).is_some());
        assert!(assets.resource(&pill_ui_asset()).is_some());

        let errors = [
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        key: AssetKey::new("bad/png").unwrap(),
                        relative_path: "bad.png".into(),
                        expected_size: None,
                    },
                    bytes: Vec::new(),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        key: AssetKey::new("bad/size").unwrap(),
                        relative_path: "bad-size.png".into(),
                        expected_size: Some(PixelSize::new(32, 32)),
                    },
                    bytes: png(1, 1),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
            assemble_assets(
                vec![AssetBytes {
                    request: AssetRequest {
                        key: AssetKey::new("solid/white").unwrap(),
                        relative_path: "duplicate.png".into(),
                        expected_size: None,
                    },
                    bytes: png(1, 1),
                }],
                Vec::new(),
            )
            .err()
            .unwrap(),
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn generated_ui_masks_have_transparent_corners_and_opaque_centers() {
        for mask in [rounded_mask(64, 64, 6), rounded_mask(128, 64, 32)] {
            let center = ((mask.size().height / 2 * mask.size().width + mask.size().width / 2) * 4
                + 3) as usize;
            assert_eq!(mask.rgba8()[3], 0);
            assert_eq!(mask.rgba8()[center], 255);
        }
    }
}
