//! Pure map document, validation, serialization, and reversible editing.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use serde::{Deserialize, Serialize};

pub const FORMAT_VERSION: &str = "gen3-map-v1";

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, MapError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(MapError::EmptyId(stringify!($name)));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(AtomicTileId);
string_id!(CompositeTileId);
string_id!(MapProjectId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilePixelSize {
    pub width: u16,
    pub height: u16,
}

impl TilePixelSize {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TilePosition(pub u16, pub u16);

impl TilePosition {
    pub const fn new(x: u16, y: u16) -> Self {
        Self(x, y)
    }

    pub const fn x(self) -> u16 {
        self.0
    }

    pub const fn y(self) -> u16 {
        self.1
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Collision {
    #[default]
    Walkable,
    Blocked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapEventKind {
    Encounter,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositeTile {
    pub id: CompositeTileId,
    pub layers: Vec<AtomicTileId>,
}

impl CompositeTile {
    pub fn new(id: CompositeTileId, layers: Vec<AtomicTileId>) -> Self {
        Self { id, layers }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualCell {
    pub material: Option<CompositeTileId>,
}

impl VisualCell {
    pub fn new(material: Option<CompositeTileId>) -> Self {
        Self { material }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapCell {
    pub visual: VisualCell,
    pub collision: Collision,
    pub event: Option<MapEventKind>,
}

impl MapCell {
    pub fn new(
        material: Option<CompositeTileId>,
        collision: Collision,
        event: Option<MapEventKind>,
    ) -> Self {
        Self {
            visual: VisualCell::new(material),
            collision,
            event,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapProject {
    pub format_version: String,
    pub id: MapProjectId,
    pub tile_size: TilePixelSize,
    pub width: u16,
    pub height: u16,
    pub materials: Vec<CompositeTile>,
    pub visual_cells: Vec<VisualCell>,
    pub collision_cells: Vec<Collision>,
    pub event_cells: Vec<Option<MapEventKind>>,
    pub player_spawn: TilePosition,
}

impl MapProject {
    pub fn blank(id: MapProjectId, width: u16, height: u16, base: Option<CompositeTile>) -> Self {
        let material = base.as_ref().map(|tile| tile.id.clone());
        let materials = base.into_iter().collect();
        let cell_count = usize::from(width) * usize::from(height);
        Self {
            format_version: FORMAT_VERSION.into(),
            id,
            tile_size: TilePixelSize::new(16, 16),
            width,
            height,
            materials,
            visual_cells: vec![VisualCell::new(material); cell_count],
            collision_cells: vec![Collision::Walkable; cell_count],
            event_cells: vec![None; cell_count],
            player_spawn: TilePosition::new(
                width.saturating_sub(1) / 2,
                height.saturating_sub(1) / 2,
            ),
        }
    }

    pub fn cell(&self, position: TilePosition) -> Option<MapCell> {
        self.cell_index(position).map(|index| MapCell {
            visual: self.visual_cells[index].clone(),
            collision: self.collision_cells[index],
            event: self.event_cells[index],
        })
    }

    pub fn material(&self, id: &CompositeTileId) -> Option<&CompositeTile> {
        self.materials.iter().find(|material| &material.id == id)
    }

    pub fn cell_index(&self, position: TilePosition) -> Option<usize> {
        (position.x() < self.width && position.y() < self.height).then(|| {
            usize::from(position.y()) * usize::from(self.width) + usize::from(position.x())
        })
    }

    pub fn validate(&self, known_tiles: &BTreeSet<AtomicTileId>) -> Result<(), MapError> {
        if self.format_version != FORMAT_VERSION {
            return Err(MapError::UnsupportedFormat(self.format_version.clone()));
        }
        if self.id.as_str().trim().is_empty() {
            return Err(MapError::EmptyId("MapProjectId"));
        }
        if self.tile_size.width == 0 || self.tile_size.height == 0 {
            return Err(MapError::EmptyTileSize(self.tile_size));
        }
        if self.width == 0 || self.height == 0 {
            return Err(MapError::EmptyMap);
        }
        let expected = usize::from(self.width) * usize::from(self.height);
        if self.visual_cells.len() != expected {
            return Err(MapError::CellCount {
                layer: "visual_cells",
                expected,
                actual: self.visual_cells.len(),
            });
        }
        if self.collision_cells.len() != expected {
            return Err(MapError::CellCount {
                layer: "collision_cells",
                expected,
                actual: self.collision_cells.len(),
            });
        }
        if self.event_cells.len() != expected {
            return Err(MapError::CellCount {
                layer: "event_cells",
                expected,
                actual: self.event_cells.len(),
            });
        }
        if self.cell_index(self.player_spawn).is_none() {
            return Err(MapError::SpawnOutOfBounds(self.player_spawn));
        }

        let mut material_ids = BTreeSet::new();
        for material in &self.materials {
            if material.id.as_str().trim().is_empty() {
                return Err(MapError::EmptyId("CompositeTileId"));
            }
            if !material_ids.insert(material.id.clone()) {
                return Err(MapError::DuplicateMaterial(material.id.clone()));
            }
            if material.layers.is_empty() {
                return Err(MapError::EmptyMaterial(material.id.clone()));
            }
            for tile in &material.layers {
                if tile.as_str().trim().is_empty() {
                    return Err(MapError::EmptyId("AtomicTileId"));
                }
                if !known_tiles.contains(tile) {
                    return Err(MapError::UnknownAtomicTile(tile.clone()));
                }
            }
        }
        for cell in &self.visual_cells {
            if let Some(id) = &cell.material
                && !material_ids.contains(id)
            {
                return Err(MapError::UnknownMaterial(id.clone()));
            }
        }
        if self.collision_cells[self.cell_index(self.player_spawn).expect("spawn checked")]
            == Collision::Blocked
        {
            return Err(MapError::SpawnBlocked(self.player_spawn));
        }
        Ok(())
    }

    pub fn from_json(json: &str, known_tiles: &BTreeSet<AtomicTileId>) -> Result<Self, MapError> {
        let project: Self =
            serde_json::from_str(json).map_err(|error| MapError::Json(error.to_string()))?;
        project.validate(known_tiles)?;
        Ok(project)
    }

    pub fn to_json_pretty(&self, known_tiles: &BTreeSet<AtomicTileId>) -> Result<String, MapError> {
        self.validate(known_tiles)?;
        serde_json::to_string_pretty(self).map_err(|error| MapError::Json(error.to_string()))
    }

    fn apply(&mut self, command: &MapEditCommand, forward: bool) -> Result<(), MapError> {
        match command {
            MapEditCommand::ReplaceCells(changes) => {
                for change in changes {
                    let index = self
                        .cell_index(change.position)
                        .ok_or(MapError::CellOutOfBounds(change.position))?;
                    let state = if forward {
                        change.after.clone()
                    } else {
                        change.before.clone()
                    };
                    self.visual_cells[index] = state.visual;
                    self.collision_cells[index] = state.collision;
                    self.event_cells[index] = state.event;
                }
            }
            MapEditCommand::CreateMaterial(material) => {
                if forward {
                    if self.material(&material.id).is_some() {
                        return Err(MapError::DuplicateMaterial(material.id.clone()));
                    }
                    self.materials.push(material.clone());
                } else {
                    self.remove_material(&material.id)?;
                }
            }
            MapEditCommand::ReplaceMaterial { before, after } => {
                let (expected, replacement) = if forward {
                    (before, after)
                } else {
                    (after, before)
                };
                let material = self
                    .materials
                    .iter_mut()
                    .find(|material| material.id == expected.id)
                    .ok_or_else(|| MapError::UnknownMaterial(expected.id.clone()))?;
                *material = replacement.clone();
            }
            MapEditCommand::RemoveMaterial(material) => {
                if forward {
                    self.remove_material(&material.id)?;
                } else {
                    self.materials.push(material.clone());
                }
            }
        }
        Ok(())
    }

    fn remove_material(&mut self, id: &CompositeTileId) -> Result<(), MapError> {
        if self
            .visual_cells
            .iter()
            .any(|cell| cell.material.as_ref() == Some(id))
        {
            return Err(MapError::MaterialInUse(id.clone()));
        }
        let index = self
            .materials
            .iter()
            .position(|material| &material.id == id)
            .ok_or_else(|| MapError::UnknownMaterial(id.clone()))?;
        self.materials.remove(index);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellChange {
    pub position: TilePosition,
    pub before: MapCell,
    pub after: MapCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapEditCommand {
    ReplaceCells(Vec<CellChange>),
    CreateMaterial(CompositeTile),
    ReplaceMaterial {
        before: CompositeTile,
        after: CompositeTile,
    },
    RemoveMaterial(CompositeTile),
}

#[derive(Clone, Debug, Default)]
pub struct EditHistory {
    undo: Vec<MapEditCommand>,
    redo: Vec<MapEditCommand>,
}

impl EditHistory {
    pub fn execute(
        &mut self,
        project: &mut MapProject,
        command: MapEditCommand,
    ) -> Result<(), MapError> {
        project.apply(&command, true)?;
        self.undo.push(command);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self, project: &mut MapProject) -> Result<bool, MapError> {
        let Some(command) = self.undo.pop() else {
            return Ok(false);
        };
        project.apply(&command, false)?;
        self.redo.push(command);
        Ok(true)
    }

    pub fn redo(&mut self, project: &mut MapProject) -> Result<bool, MapError> {
        let Some(command) = self.redo.pop() else {
            return Ok(false);
        };
        project.apply(&command, true)?;
        self.undo.push(command);
        Ok(true)
    }

    pub fn is_dirty(&self) -> bool {
        !self.undo.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapError {
    EmptyId(&'static str),
    UnsupportedFormat(String),
    EmptyTileSize(TilePixelSize),
    EmptyMap,
    CellCount {
        layer: &'static str,
        expected: usize,
        actual: usize,
    },
    CellOutOfBounds(TilePosition),
    SpawnOutOfBounds(TilePosition),
    SpawnBlocked(TilePosition),
    DuplicateMaterial(CompositeTileId),
    UnknownMaterial(CompositeTileId),
    EmptyMaterial(CompositeTileId),
    UnknownAtomicTile(AtomicTileId),
    MaterialInUse(CompositeTileId),
    Json(String),
}

impl fmt::Display for MapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId(kind) => write!(formatter, "{kind} must not be empty"),
            Self::UnsupportedFormat(version) => {
                write!(formatter, "unsupported map format {version}")
            }
            Self::EmptyTileSize(size) => {
                write!(formatter, "tile size must be non-zero, got {size:?}")
            }
            Self::EmptyMap => formatter.write_str("map width and height must be non-zero"),
            Self::CellCount {
                layer,
                expected,
                actual,
            } => write!(
                formatter,
                "map layer {layer} requires {expected} cells, received {actual}"
            ),
            Self::CellOutOfBounds(position) => {
                write!(formatter, "cell {position:?} is outside the map")
            }
            Self::SpawnOutOfBounds(position) => {
                write!(formatter, "spawn {position:?} is outside the map")
            }
            Self::SpawnBlocked(position) => write!(formatter, "spawn {position:?} is blocked"),
            Self::DuplicateMaterial(id) => {
                write!(formatter, "material {id} is defined more than once")
            }
            Self::UnknownMaterial(id) => write!(formatter, "material {id} is not defined"),
            Self::EmptyMaterial(id) => write!(formatter, "material {id} has no layers"),
            Self::UnknownAtomicTile(id) => write!(formatter, "atomic tile {id} is not available"),
            Self::MaterialInUse(id) => {
                write!(formatter, "material {id} is still used by map cells")
            }
            Self::Json(message) => write!(formatter, "invalid map JSON: {message}"),
        }
    }
}

impl Error for MapError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn id(value: &str) -> AtomicTileId {
        AtomicTileId::new(value).unwrap()
    }

    fn fixture() -> (MapProject, BTreeSet<AtomicTileId>) {
        let known = [id("tile-0000"), id("tile-0001")].into_iter().collect();
        let material = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![id("tile-0000"), id("tile-0001")],
        );
        (
            MapProject::blank(MapProjectId::new("demo").unwrap(), 3, 2, Some(material)),
            known,
        )
    }

    #[test]
    fn round_trips_the_versioned_json_document() {
        let (project, known) = fixture();
        let json = project.to_json_pretty(&known).unwrap();
        let decoded = MapProject::from_json(&json, &known).unwrap();
        assert_eq!(decoded, project);
        assert!(json.contains("gen3-map-v1"));
        assert!(json.contains("\"visual_cells\""));
        assert!(json.contains("\"collision_cells\""));
        assert!(json.contains("\"event_cells\""));
    }

    #[test]
    fn accepts_an_unbounded_number_of_flat_layers() {
        let (mut project, mut known) = fixture();
        let layers = (0..4096)
            .map(|index| id(&format!("layer-{index}")))
            .inspect(|tile| {
                known.insert(tile.clone());
            })
            .collect();
        project.materials[0].layers = layers;
        project.validate(&known).unwrap();
    }

    #[test]
    fn rejects_unknown_atomic_and_composite_references() {
        let (mut project, known) = fixture();
        project.materials[0].layers.push(id("missing"));
        assert!(matches!(
            project.validate(&known),
            Err(MapError::UnknownAtomicTile(_))
        ));

        project.materials[0].layers.pop();
        project.visual_cells[0].material = Some(CompositeTileId::new("missing").unwrap());
        assert!(matches!(
            project.validate(&known),
            Err(MapError::UnknownMaterial(_))
        ));
    }

    #[test]
    fn undo_and_redo_restore_cell_edits() {
        let (mut project, _) = fixture();
        let position = TilePosition::new(1, 1);
        let before = project.cell(position).unwrap();
        let after = MapCell::new(None, Collision::Blocked, Some(MapEventKind::Encounter));
        let mut history = EditHistory::default();
        history
            .execute(
                &mut project,
                MapEditCommand::ReplaceCells(vec![CellChange {
                    position,
                    before: before.clone(),
                    after: after.clone(),
                }]),
            )
            .unwrap();
        assert_eq!(project.cell(position), Some(after.clone()));
        assert!(history.undo(&mut project).unwrap());
        assert_eq!(project.cell(position), Some(before.clone()));
        assert!(history.redo(&mut project).unwrap());
        assert_eq!(project.cell(position), Some(after));
    }
}
