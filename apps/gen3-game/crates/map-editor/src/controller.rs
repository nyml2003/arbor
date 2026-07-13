use map_project::{Collision, MapEventKind, TilePosition};
use punctum_gpu::Viewport;
use punctum_grid::GridPos;

use crate::{
    layout,
    model::{EditorIntent, EditorModel, EditorTool},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
}

#[derive(Default)]
pub struct EditorController {
    pub hover: Option<TilePosition>,
    cursor: Option<GridPos>,
    pressed: Option<PointerButton>,
    last_painted: Option<TilePosition>,
}

impl EditorController {
    pub fn move_cursor(
        &mut self,
        x: f64,
        y: f64,
        viewport: Viewport,
        model: &EditorModel,
    ) -> Option<EditorIntent> {
        self.cursor = grid_position(x, y, viewport);
        if model.show_help {
            self.hover = None;
            return None;
        }
        self.hover = self
            .cursor
            .and_then(|position| map_position(position, model));
        let position = self.hover?;
        let button = self.pressed?;
        if self.last_painted == Some(position) {
            return None;
        }
        self.last_painted = Some(position);
        Some(EditorIntent::Paint {
            position,
            erase: button == PointerButton::Secondary,
        })
    }

    pub fn press(&mut self, button: PointerButton, model: &EditorModel) -> Option<EditorIntent> {
        let grid = self.cursor?;
        if model.show_help {
            let ui = layout::workbench();
            return (button == PointerButton::Primary && ui.help.contains(grid))
                .then_some(EditorIntent::ToggleHelp);
        }
        if let Some(position) = map_position(grid, model) {
            self.pressed = Some(button);
            self.last_painted = Some(position);
            return Some(EditorIntent::Paint {
                position,
                erase: button == PointerButton::Secondary,
            });
        }
        if button == PointerButton::Secondary {
            return None;
        }
        self.pressed = None;
        self.last_painted = None;
        click_intent(grid, model)
    }

    pub fn release(&mut self, button: PointerButton) {
        if self.pressed == Some(button) {
            self.pressed = None;
            self.last_painted = None;
        }
    }

    pub fn leave(&mut self) {
        self.hover = None;
        self.cursor = None;
        self.pressed = None;
        self.last_painted = None;
    }
}

fn click_intent(position: GridPos, model: &EditorModel) -> Option<EditorIntent> {
    let ui = layout::workbench();
    let asset_page = model.selected_atomic / layout::ASSET_PAGE_SIZE;
    for local in 0..layout::ASSET_PAGE_SIZE {
        if ui.asset_slots[local].contains(position) {
            let index = asset_page * layout::ASSET_PAGE_SIZE + local;
            return (index < model.atomic_ids.len()).then_some(EditorIntent::SelectAtomic(index));
        }
    }
    let material_page = model.selected_material / layout::MATERIAL_PAGE_SIZE;
    for local in 0..layout::MATERIAL_PAGE_SIZE {
        if ui.material_slots[local].contains(position) {
            let index = material_page * layout::MATERIAL_PAGE_SIZE + local;
            return (index < model.project.materials.len())
                .then_some(EditorIntent::SelectMaterial(index));
        }
    }
    if ui.previous_assets.contains(position) {
        let page = asset_page.saturating_sub(1);
        return Some(EditorIntent::SelectAtomic(page * layout::ASSET_PAGE_SIZE));
    }
    if ui.next_assets.contains(position) {
        let maximum_page = model.atomic_ids.len().saturating_sub(1) / layout::ASSET_PAGE_SIZE;
        let page = (asset_page + 1).min(maximum_page);
        return Some(EditorIntent::SelectAtomic(page * layout::ASSET_PAGE_SIZE));
    }
    if ui.previous_materials.contains(position) {
        let page = material_page.saturating_sub(1);
        return Some(EditorIntent::SelectMaterial(
            page * layout::MATERIAL_PAGE_SIZE,
        ));
    }
    if ui.next_materials.contains(position) {
        let maximum_page =
            model.project.materials.len().saturating_sub(1) / layout::MATERIAL_PAGE_SIZE;
        let page = (material_page + 1).min(maximum_page);
        return Some(EditorIntent::SelectMaterial(
            page * layout::MATERIAL_PAGE_SIZE,
        ));
    }
    if ui.add_layer.contains(position) {
        return Some(EditorIntent::AddLayer);
    }
    if ui.remove_layer.contains(position) {
        return Some(EditorIntent::RemoveLayer);
    }
    if ui.delete_material.contains(position) {
        return Some(EditorIntent::DeleteMaterial);
    }
    if ui.save.contains(position) {
        return Some(EditorIntent::Save);
    }
    if ui.undo.contains(position) {
        return Some(EditorIntent::Undo);
    }
    if ui.redo.contains(position) {
        return Some(EditorIntent::Redo);
    }
    if ui.help.contains(position) {
        return Some(EditorIntent::ToggleHelp);
    }
    if ui.visual.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Visual));
    }
    if ui.walkable.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Walkable,
        )));
    }
    if ui.blocked.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Blocked,
        )));
    }
    if ui.encounter.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Event(Some(
            MapEventKind::Encounter,
        ))));
    }
    if ui.clear_event.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Event(None)));
    }
    None
}

fn grid_position(x: f64, y: f64, viewport: Viewport) -> Option<GridPos> {
    let x = x - f64::from(viewport.origin.x);
    let y = y - f64::from(viewport.origin.y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let col = (x / f64::from(viewport.cell_size.width)).floor() as i32;
    let row = (y / f64::from(viewport.cell_size.height)).floor() as i32;
    (col < layout::COLS as i32 && row < layout::ROWS as i32).then_some(GridPos::new(col, row))
}

fn map_position(position: GridPos, model: &EditorModel) -> Option<TilePosition> {
    let col = position.col / layout::MAP_TILE_SPAN as i32;
    let row = position.row / layout::MAP_TILE_SPAN as i32;
    (layout::MAP_RECT.contains(position)
        && col < i32::from(model.project.width)
        && row < i32::from(model.project.height))
    .then(|| TilePosition::new(col as u16, row as u16))
}

#[cfg(test)]
mod tests {
    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};
    use punctum_gpu::{PixelOffset, PixelSize};

    use super::*;

    fn model() -> EditorModel {
        let tile = AtomicTileId::new("tile").unwrap();
        EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                16,
                10,
                Some(CompositeTile::new(
                    CompositeTileId::new("base").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            vec![tile; 20],
        )
    }

    #[test]
    fn canvas_and_palette_clicks_produce_intents_without_mutating_the_model() {
        let model = model();
        let viewport = Viewport::new(
            PixelSize::new(1920, 1040),
            PixelOffset::new(0, 0),
            PixelSize::new(40, 40),
        )
        .unwrap();
        let mut controller = EditorController::default();
        controller.move_cursor(80.0, 120.0, viewport, &model);
        assert!(matches!(
            controller.press(PointerButton::Primary, &model),
            Some(EditorIntent::Paint {
                position: TilePosition(1, 1),
                ..
            })
        ));
        controller.release(PointerButton::Primary);
        controller.move_cursor(50.5 * 40.0, 2.5 * 40.0, viewport, &model);
        assert_eq!(
            controller.press(PointerButton::Primary, &model),
            Some(EditorIntent::SelectAtomic(0))
        );
        assert_eq!(
            controller.move_cursor(40.0, 40.0, viewport, &model),
            None,
            "dragging from a UI control must not paint the canvas"
        );
    }

    #[test]
    fn material_pages_are_clickable() {
        let mut model = model();
        let base = model.project.materials[0].clone();
        for index in 1..8 {
            let mut material = base.clone();
            material.id = CompositeTileId::new(format!("material-{index:04}")).unwrap();
            model.project.materials.push(material);
        }
        let viewport = Viewport::new(
            PixelSize::new(1920, 1040),
            PixelOffset::new(0, 0),
            PixelSize::new(40, 40),
        )
        .unwrap();
        let mut controller = EditorController::default();
        controller.move_cursor(41.5 * 40.0, 33.5 * 40.0, viewport, &model);
        assert_eq!(
            controller.press(PointerButton::Primary, &model),
            Some(EditorIntent::SelectMaterial(5))
        );
    }
}
