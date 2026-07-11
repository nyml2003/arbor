use punctum_gpu::{
    GpuAtlas, GpuCell, GpuResource, PixelOffset, PixelRect, PixelSize, ResourceId, Rgba8, Viewport,
};
use punctum_grid::{Patch, Surface, diff};
use punctum_input::KeyEvent;
use punctum_tetris::{
    PieceKind, SURFACE_HEIGHT, SURFACE_WIDTH, TetrisCell, TetrisState, command_for_key, paint,
    transition,
};

const WHITE_RESOURCE: ResourceId = ResourceId(1);
const WHITE_PIXEL: [u8; 4] = [255; 4];
const BORDER_COLOR: Rgba8 = Rgba8::new(92, 102, 112, 255);

pub fn atlas() -> GpuAtlas {
    GpuAtlas::new(
        PixelSize::new(1, 1),
        WHITE_PIXEL.to_vec(),
        &[GpuResource::new(WHITE_RESOURCE, PixelRect::new(0, 0, 1, 1))],
    )
    .expect("the embedded white pixel atlas is valid")
}

pub fn project_cell(cell: TetrisCell) -> GpuCell {
    match cell {
        TetrisCell::Empty => GpuCell::Empty,
        TetrisCell::Border => sprite(BORDER_COLOR),
        TetrisCell::Tetromino(kind) => sprite(piece_color(kind)),
    }
}

pub fn project(state: &TetrisState) -> Surface<GpuCell> {
    let source = paint(state);
    let cells = source.cells().iter().copied().map(project_cell).collect();
    Surface::from_cells(source.size(), cells).expect("projection preserves the surface dimensions")
}

pub struct ProjectedFrame {
    surface: Surface<GpuCell>,
    patch: Option<Patch<GpuCell>>,
}

impl ProjectedFrame {
    pub fn surface(&self) -> &Surface<GpuCell> {
        &self.surface
    }

    pub fn patch(&self) -> Option<&Patch<GpuCell>> {
        self.patch.as_ref()
    }

    pub fn into_surface(self) -> Surface<GpuCell> {
        self.surface
    }
}

pub fn project_frame(previous: Option<&Surface<GpuCell>>, state: &TetrisState) -> ProjectedFrame {
    let surface = project(state);
    let patch = previous.map(|previous| diff(previous, &surface));
    ProjectedFrame { surface, patch }
}

pub fn viewport(target_size: PixelSize) -> Viewport {
    let cell_size = (target_size.width / SURFACE_WIDTH)
        .min(target_size.height / SURFACE_HEIGHT)
        .max(1);
    let board_width = i64::from(SURFACE_WIDTH) * i64::from(cell_size);
    let board_height = i64::from(SURFACE_HEIGHT) * i64::from(cell_size);
    let origin_x = (i64::from(target_size.width) - board_width).div_euclid(2);
    let origin_y = (i64::from(target_size.height) - board_height).div_euclid(2);

    Viewport::new(
        target_size,
        PixelOffset::new(origin_x as i32, origin_y as i32),
        PixelSize::new(cell_size, cell_size),
    )
    .expect("the integer cell size is always at least one pixel")
}

pub fn apply_key(state: &TetrisState, key: &KeyEvent) -> TetrisState {
    command_for_key(key).map_or_else(|| state.clone(), |command| transition(state, command))
}

const fn sprite(tint: Rgba8) -> GpuCell {
    GpuCell::Sprite {
        resource: WHITE_RESOURCE,
        tint,
    }
}

const fn piece_color(kind: PieceKind) -> Rgba8 {
    match kind {
        PieceKind::I => Rgba8::new(0, 224, 224, 255),
        PieceKind::O => Rgba8::new(240, 208, 0, 255),
        PieceKind::T => Rgba8::new(160, 64, 208, 255),
        PieceKind::S => Rgba8::new(48, 192, 80, 255),
        PieceKind::Z => Rgba8::new(224, 64, 64, 255),
        PieceKind::J => Rgba8::new(48, 96, 224, 255),
        PieceKind::L => Rgba8::new(240, 144, 32, 255),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use punctum_gpu::{GpuCell, PixelOffset, PixelRect, PixelSize, ResourceId, Rgba8};
    use punctum_grid::{GridPos, PatchKind};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, PhysicalKeyCode};
    use punctum_tetris::{
        PieceKind, SURFACE_HEIGHT, SURFACE_WIDTH, TetrisCell, TetrisCommand, TetrisState, paint,
        transition,
    };

    use super::{apply_key, atlas, project, project_cell, project_frame, viewport};

    fn state(sequence: &[PieceKind]) -> TetrisState {
        TetrisState::new(sequence.to_vec()).unwrap()
    }

    fn sprite_tint(cell: GpuCell) -> Rgba8 {
        let GpuCell::Sprite { tint, .. } = cell else {
            panic!("expected a visible sprite")
        };
        tint
    }

    #[test]
    fn atlas_is_one_opaque_white_pixel() {
        let atlas = atlas();

        assert_eq!(atlas.size(), PixelSize::new(1, 1));
        assert_eq!(atlas.rgba8(), &[255, 255, 255, 255]);
        assert_eq!(
            atlas.resource(ResourceId(1)),
            Some(PixelRect::new(0, 0, 1, 1))
        );
    }

    #[test]
    fn empty_border_and_all_tetrominoes_have_stable_projection() {
        assert_eq!(project_cell(TetrisCell::Empty), GpuCell::Empty);

        let border = project_cell(TetrisCell::Border);
        assert!(matches!(
            border,
            GpuCell::Sprite {
                resource: ResourceId(1),
                ..
            }
        ));
        assert_eq!(sprite_tint(border).alpha, 255);

        let colors = PieceKind::ALL.map(|kind| {
            let cell = project_cell(TetrisCell::Tetromino(kind));
            assert!(matches!(
                cell,
                GpuCell::Sprite {
                    resource: ResourceId(1),
                    ..
                }
            ));
            sprite_tint(cell)
        });
        assert_eq!(
            colors
                .into_iter()
                .map(Rgba8::to_array)
                .collect::<BTreeSet<_>>()
                .len(),
            7
        );
    }

    #[test]
    fn projection_is_the_cell_for_cell_gpu_view_of_tetris_paint() {
        let state = state(&[PieceKind::T]);
        let painted = paint(&state);
        let projected = project(&state);

        assert_eq!(projected.size(), painted.size());
        for (source, target) in painted.cells().iter().zip(projected.cells()) {
            assert_eq!(*target, project_cell(*source));
        }
        assert_eq!(
            projected.get(GridPos::new(0, 0)),
            Ok(&project_cell(TetrisCell::Border))
        );
    }

    #[test]
    fn viewport_uses_the_largest_integer_cell_size_and_centers_the_board() {
        let exact = viewport(PixelSize::new(SURFACE_WIDTH * 16, SURFACE_HEIGHT * 16));
        assert_eq!(exact.cell_size, PixelSize::new(16, 16));
        assert_eq!(exact.origin, PixelOffset::new(0, 0));

        let wide = viewport(PixelSize::new(SURFACE_WIDTH * 10 + 9, SURFACE_HEIGHT * 10));
        assert_eq!(wide.cell_size, PixelSize::new(10, 10));
        assert_eq!(wide.origin, PixelOffset::new(4, 0));
    }

    #[test]
    fn small_and_minimized_targets_keep_a_valid_clipped_viewport() {
        let small = viewport(PixelSize::new(5, 7));
        assert_eq!(small.cell_size, PixelSize::new(1, 1));
        assert_eq!(small.origin, PixelOffset::new(-4, -8));

        let minimized = viewport(PixelSize::new(0, 0));
        assert_eq!(minimized.target_size, PixelSize::new(0, 0));
        assert_eq!(minimized.cell_size, PixelSize::new(1, 1));
        assert_eq!(minimized.origin, PixelOffset::new(-6, -11));
    }

    #[test]
    fn resize_and_projection_do_not_modify_game_state() {
        let state = transition(&state(&[PieceKind::I]), TetrisCommand::SoftDrop);
        let before = state.clone();

        for size in [
            PixelSize::new(640, 480),
            PixelSize::new(100, 80),
            PixelSize::new(0, 0),
        ] {
            let _ = viewport(size);
            let _ = project(&state);
        }

        assert_eq!(state, before);
    }

    #[test]
    fn input_chain_restarts_game_over_through_shared_command_mapping() {
        let mut game_over = state(&[PieceKind::O]);
        while !game_over.is_game_over() {
            game_over = transition(&game_over, TetrisCommand::HardDrop);
        }
        let restart = KeyEvent {
            physical: Some(PhysicalKeyCode::KeyR),
            logical: LogicalKey::Character("r".into()),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        };

        let restarted = apply_key(&game_over, &restart);

        assert!(!restarted.is_game_over());
        assert_eq!(restarted.active_piece().unwrap().kind(), PieceKind::O);
        assert_ne!(project(&game_over), project(&restarted));
    }

    #[test]
    fn first_frame_is_full_and_later_frames_are_grid_diffs() {
        let before = state(&[PieceKind::T]);
        let first = project_frame(None, &before);
        assert!(first.patch().is_none());

        let previous = first.into_surface();
        let after = transition(&before, TetrisCommand::MoveLeft);
        let second = project_frame(Some(&previous), &after);
        let patch = second.patch().expect("later frames use a patch");

        assert_eq!(patch.kind(), PatchKind::Delta);
        assert!(!patch.spans().is_empty());
        assert_eq!(second.surface(), &project(&after));
    }
}
