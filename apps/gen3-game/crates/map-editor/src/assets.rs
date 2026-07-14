use std::{error::Error, path::PathBuf};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{read_optional_text, read_tile_sources};
use game_native_target::NativeAssets;
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_project::{AtomicTileId, MapProject};
use map_render::AtomicTileCatalog;
use punctum_gpu::Rgba8;

pub struct EditorAssets {
    pub native: NativeAssets,
    pub catalog: AtomicTileCatalog,
    pub ids: Vec<AtomicTileId>,
}

pub fn load_assets() -> Result<EditorAssets, Box<dyn Error>> {
    let assets = build_tile_assets(read_tile_sources(&tile_root())?)?;
    let mut images = vec![(
        AssetKey::new("solid/white").expect("the white asset key is valid"),
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    images.extend(assets.images);
    let native = NativeAssets::new(images)?;
    Ok(EditorAssets {
        native,
        catalog: assets.catalog,
        ids: assets.ids,
    })
}

pub fn load_project(
    path: &std::path::Path,
    ids: &[AtomicTileId],
) -> Result<MapProject, Box<dyn Error>> {
    let json = read_optional_text(path)?;
    Ok(project_from_json_or_default(json.as_deref(), ids)?)
}

pub fn default_project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../maps/demo-map.json")
}

fn tile_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/maps/25_47179/tiles")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_real_tile_catalog_and_default_project() {
        let assets = load_assets().unwrap();
        assert!(assets.ids.len() > 200);
        let project = project_from_json_or_default(None, &assets.ids).unwrap();
        assert_eq!((project.width, project.height), (24, 16));
    }
}
