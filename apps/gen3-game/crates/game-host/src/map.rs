use std::{collections::BTreeSet, error::Error, fmt, fs, path::PathBuf};

use game_assets::{DecodedImage, decode_png};
use map_project::{
    AtomicTileId, Collision, CompositeTile, CompositeTileId, MapEventKind, MapProject,
    MapProjectId, TilePosition,
};
use map_render::{AtomicTileCatalog, AtomicTileResource};
use punctum_gpu::{PixelSize, ResourceId};

const MAP_RESOURCE_START: u32 = 1_000;

pub struct LoadedMap {
    pub project: MapProject,
    pub catalog: AtomicTileCatalog,
    pub images: Vec<(ResourceId, DecodedImage)>,
}

pub fn load_map() -> Result<LoadedMap, MapLoadError> {
    let root = tile_root();
    let mut paths = fs::read_dir(&root)
        .map_err(|error| MapLoadError::Read(root.clone(), error.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
        .collect::<Vec<_>>();
    paths.sort();

    let mut ids = Vec::with_capacity(paths.len());
    let mut resources = Vec::with_capacity(paths.len());
    let mut images = Vec::with_capacity(paths.len());
    for (index, path) in paths.iter().enumerate() {
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| MapLoadError::InvalidFilename(path.clone()))?;
        let id =
            AtomicTileId::new(name).map_err(|error| MapLoadError::Project(error.to_string()))?;
        let resource = ResourceId(MAP_RESOURCE_START + index as u32);
        let bytes =
            fs::read(path).map_err(|error| MapLoadError::Read(path.clone(), error.to_string()))?;
        let image = decode_png(&bytes)
            .map_err(|error| MapLoadError::Decode(path.clone(), error.to_string()))?;
        if image.size() != PixelSize::new(16, 16) {
            return Err(MapLoadError::WrongSize(path.clone(), image.size()));
        }
        ids.push(id.clone());
        resources.push(AtomicTileResource { id, resource });
        images.push((resource, image));
    }
    if ids.is_empty() {
        return Err(MapLoadError::EmptyTiles(root));
    }

    let known = ids.iter().cloned().collect::<BTreeSet<_>>();
    let path = project_path();
    let project = if path.is_file() {
        let json = fs::read_to_string(&path)
            .map_err(|error| MapLoadError::Read(path.clone(), error.to_string()))?;
        MapProject::from_json(&json, &known)
            .map_err(|error| MapLoadError::Project(error.to_string()))?
    } else {
        default_project(&ids).map_err(|error| MapLoadError::Project(error.to_string()))?
    };
    let catalog = AtomicTileCatalog::new(resources)
        .map_err(|error| MapLoadError::Catalog(error.to_string()))?;
    Ok(LoadedMap {
        project,
        catalog,
        images,
    })
}

fn default_project(ids: &[AtomicTileId]) -> Result<MapProject, map_project::MapError> {
    let tile = |name: &str| {
        ids.iter()
            .find(|id| id.as_str() == name)
            .or_else(|| ids.first())
            .cloned()
            .expect("map loader rejects an empty tile set")
    };
    let ground_id = CompositeTileId::new("material-0000")?;
    let flower_id = CompositeTileId::new("material-0001")?;
    let grass_id = CompositeTileId::new("material-0002")?;
    let rock_id = CompositeTileId::new("material-0003")?;
    let border_id = CompositeTileId::new("material-0004")?;
    let mut project = MapProject::blank(
        MapProjectId::new("demo-map")?,
        24,
        16,
        Some(CompositeTile::new(
            ground_id.clone(),
            vec![tile("tile-0102")],
        )),
    );
    project.materials.extend([
        CompositeTile::new(flower_id.clone(), vec![tile("tile-0101")]),
        CompositeTile::new(grass_id.clone(), vec![tile("tile-0102"), tile("tile-0110")]),
        CompositeTile::new(rock_id.clone(), vec![tile("tile-0102"), tile("tile-0223")]),
        CompositeTile::new(border_id.clone(), vec![tile("tile-0251")]),
    ]);
    project.player_spawn = TilePosition::new(3, 6);
    for y in 0..project.height {
        for x in 0..project.width {
            let border = x == 0 || y == 0 || x + 1 == project.width || y + 1 == project.height;
            let grass = ((6..=10).contains(&x) && (2..=7).contains(&y))
                || ((15..=20).contains(&x) && (8..=13).contains(&y));
            let rocks = matches!(
                (x, y),
                (3, 3) | (4, 3) | (12, 5) | (12, 6) | (18, 4) | (19, 4)
            );
            let index = usize::from(y * project.width + x);
            let (material, collision, event) = if border {
                (Some(border_id.clone()), Collision::Blocked, None)
            } else if rocks {
                (Some(rock_id.clone()), Collision::Blocked, None)
            } else if grass {
                (
                    Some(grass_id.clone()),
                    Collision::Walkable,
                    Some(MapEventKind::Encounter),
                )
            } else if (x + y) % 7 == 0 {
                (Some(flower_id.clone()), Collision::Walkable, None)
            } else {
                (Some(ground_id.clone()), Collision::Walkable, None)
            };
            project.visual_cells[index].material = material;
            project.collision_cells[index] = collision;
            project.event_cells[index] = event;
        }
    }
    Ok(project)
}

fn tile_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/maps/25_47179/tiles")
}

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../maps/demo-map.json")
}

#[derive(Debug)]
pub enum MapLoadError {
    Read(PathBuf, String),
    Decode(PathBuf, String),
    InvalidFilename(PathBuf),
    WrongSize(PathBuf, PixelSize),
    EmptyTiles(PathBuf),
    Project(String),
    Catalog(String),
}

impl fmt::Display for MapLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(path, message) => {
                write!(formatter, "cannot read {}: {message}", path.display())
            }
            Self::Decode(path, message) => {
                write!(formatter, "cannot decode {}: {message}", path.display())
            }
            Self::InvalidFilename(path) => {
                write!(formatter, "tile filename is not UTF-8: {}", path.display())
            }
            Self::WrongSize(path, size) => write!(
                formatter,
                "tile {} is {size:?}, expected 16x16",
                path.display()
            ),
            Self::EmptyTiles(path) => {
                write!(formatter, "tile directory {} is empty", path.display())
            }
            Self::Project(message) => write!(formatter, "invalid map project: {message}"),
            Self::Catalog(message) => write!(formatter, "invalid tile catalog: {message}"),
        }
    }
}

impl Error for MapLoadError {}
