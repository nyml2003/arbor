//! Pure battle-screen projection and keyboard focus state.

#![forbid(unsafe_code)]

use battle_application::{
    Action, BattleObservation, BattlePhase, MoveSlot, Pokemon, Side, TeamSlot,
};
use game_assets::{DecodedImage, build_atlas};
use punctum_gpu::{GpuAtlas, GpuCell, GpuImage, PixelOffset, ResourceId, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use world_application::{
    Direction as WorldDirection, Position, Tile, WorldCommand, WorldObservation,
};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;

const WHITE_RESOURCE: ResourceId = ResourceId(1);
const PLAYER_BACK_FRAME_0_RESOURCE: ResourceId = ResourceId(2);
const PLAYER_BACK_FRAME_1_RESOURCE: ResourceId = ResourceId(3);
const OPPONENT_FRONT_FRAME_0_RESOURCE: ResourceId = ResourceId(4);
const OPPONENT_FRONT_FRAME_1_RESOURCE: ResourceId = ResourceId(5);
const CHARACTER_RESOURCE_START: u32 = 6;
const WHITE_PIXEL: Rgba8 = Rgba8::new(255, 255, 255, 255);
const PLAYER_BACK_FRAME_0_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/back/001_Back_0_C__frame_0.png");
const PLAYER_BACK_FRAME_1_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/back/001_Back_0_C__frame_1.png");
const OPPONENT_FRONT_FRAME_0_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/front/001_Front_0_C__frame_0.png");
const OPPONENT_FRONT_FRAME_1_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/front/001_Front_0_C__frame_1.png");
const CHARACTER_PNGS: [(&str, &[u8]); 24] = [
    (
        "down stand",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_stand.png"),
    ),
    (
        "down walk 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_walk_2.png"),
    ),
    (
        "down walk 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_walk_3.png"),
    ),
    (
        "down run 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_run_1.png"),
    ),
    (
        "down run 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_run_2.png"),
    ),
    (
        "down run 3",
        include_bytes!("../../../assets/characters/red/actions/group-00/down_runn_3.png"),
    ),
    (
        "left stand",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_stand.png"),
    ),
    (
        "left walk 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_walk_1.png"),
    ),
    (
        "left walk 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_walk_2.png"),
    ),
    (
        "left run 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_run_1.png"),
    ),
    (
        "left run 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_run_2.png"),
    ),
    (
        "left run 3",
        include_bytes!("../../../assets/characters/red/actions/group-00/left_run_3.png"),
    ),
    (
        "right stand",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_stand.png"),
    ),
    (
        "right walk 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_walk_1.png"),
    ),
    (
        "right walk 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_walk_2.png"),
    ),
    (
        "right run 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_run_1.png"),
    ),
    (
        "right run 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_run_2.png"),
    ),
    (
        "right run 3",
        include_bytes!("../../../assets/characters/red/actions/group-00/right_run_3.png"),
    ),
    (
        "up stand",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_stand.png"),
    ),
    (
        "up walk 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_walk_1.png"),
    ),
    (
        "up walk 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_walk_2.png"),
    ),
    (
        "up run 1",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_run_1.png"),
    ),
    (
        "up run 2",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_run_2.png"),
    ),
    (
        "up run 3",
        include_bytes!("../../../assets/characters/red/actions/group-00/up_run_3.png"),
    ),
];

const SKY: Rgba8 = Rgba8::new(145, 205, 210, 255);
const DISTANT_GRASS: Rgba8 = Rgba8::new(104, 164, 112, 255);
const GROUND: Rgba8 = Rgba8::new(83, 132, 79, 255);
const PLATFORM: Rgba8 = Rgba8::new(167, 190, 124, 255);
const PANEL: Rgba8 = Rgba8::new(28, 34, 45, 248);
const PANEL_EDGE: Rgba8 = Rgba8::new(218, 225, 214, 255);
const SELECTED: Rgba8 = Rgba8::new(45, 125, 137, 255);
const HP_GOOD: Rgba8 = Rgba8::new(74, 190, 102, 255);
const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);
const MAP_GROUND_LIGHT: Rgba8 = Rgba8::new(157, 202, 132, 255);
const MAP_GRASS: Rgba8 = Rgba8::new(54, 137, 79, 255);
const MAP_GRASS_LIGHT: Rgba8 = Rgba8::new(82, 163, 91, 255);
const MAP_WALL: Rgba8 = Rgba8::new(100, 105, 111, 255);
const MAP_WALL_LIGHT: Rgba8 = Rgba8::new(142, 146, 145, 255);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleAnimation {
    #[default]
    Idle,
    Acting(Side),
    Hit(Side),
    Fainted(Side),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldAnimation {
    #[default]
    Stand,
    Walk,
    Run,
    RunStopping,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleDisplayState {
    pub own_name: String,
    pub own_hp: u32,
    pub own_max_hp: u32,
    pub opponent_name: String,
    pub opponent_hp: u32,
    pub opponent_max_hp: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BattleUiState {
    selected_index: usize,
}

impl BattleUiState {
    pub const fn selected_index(self) -> usize {
        self.selected_index
    }

    pub fn reconcile(&mut self, actions: &[Action]) {
        if actions.is_empty() {
            self.selected_index = 0;
        } else {
            self.selected_index = self.selected_index.min(actions.len() - 1);
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent, actions: &[Action]) -> BattleUiOutcome {
        if key.phase == KeyPhase::Release || actions.is_empty() {
            return BattleUiOutcome::Ignored;
        }
        self.reconcile(actions);
        match key.logical {
            LogicalKey::Named(NamedKey::ArrowLeft) | LogicalKey::Named(NamedKey::ArrowUp) => {
                self.selected_index = self
                    .selected_index
                    .checked_sub(1)
                    .unwrap_or(actions.len() - 1);
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::ArrowRight) | LogicalKey::Named(NamedKey::ArrowDown) => {
                self.selected_index = (self.selected_index + 1) % actions.len();
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::Enter) if key.phase == KeyPhase::Press => {
                BattleUiOutcome::Submit(actions[self.selected_index])
            }
            _ => BattleUiOutcome::Ignored,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleUiOutcome {
    Updated,
    Submit(Action),
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Location,
    OpponentName,
    OpponentHp,
    PlayerName,
    PlayerHp,
    Action(usize),
    Message,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLabel {
    pub role: TextRole,
    pub col: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
    pub content: String,
    pub color: Rgba8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameView {
    surface: Surface<GpuCell>,
    images: Vec<GpuImage>,
    labels: Vec<TextLabel>,
}

impl GameView {
    pub const fn surface(&self) -> &Surface<GpuCell> {
        &self.surface
    }

    pub fn labels(&self) -> &[TextLabel] {
        &self.labels
    }

    pub fn images(&self) -> &[GpuImage] {
        &self.images
    }
}

pub fn atlas() -> GpuAtlas {
    let white = DecodedImage::solid(WHITE_PIXEL);
    let player_back_frame_0 = decode_embedded_png(PLAYER_BACK_FRAME_0_PNG, "player back frame 0");
    let player_back_frame_1 = decode_embedded_png(PLAYER_BACK_FRAME_1_PNG, "player back frame 1");
    let opponent_front_frame_0 =
        decode_embedded_png(OPPONENT_FRONT_FRAME_0_PNG, "opponent front frame 0");
    let opponent_front_frame_1 =
        decode_embedded_png(OPPONENT_FRONT_FRAME_1_PNG, "opponent front frame 1");
    let character_images: Vec<_> = CHARACTER_PNGS
        .iter()
        .map(|(name, bytes)| decode_embedded_png(bytes, name))
        .collect();
    let mut images = vec![
        (WHITE_RESOURCE, &white),
        (PLAYER_BACK_FRAME_0_RESOURCE, &player_back_frame_0),
        (PLAYER_BACK_FRAME_1_RESOURCE, &player_back_frame_1),
        (OPPONENT_FRONT_FRAME_0_RESOURCE, &opponent_front_frame_0),
        (OPPONENT_FRONT_FRAME_1_RESOURCE, &opponent_front_frame_1),
    ];
    images.extend(
        character_images
            .iter()
            .enumerate()
            .map(|(index, image)| (ResourceId(CHARACTER_RESOURCE_START + index as u32), image)),
    );
    build_atlas(&images).expect("the embedded game atlas is valid")
}

fn decode_embedded_png(bytes: &[u8], name: &str) -> DecodedImage {
    game_assets::decode_png(bytes).unwrap_or_else(|error| panic!("embedded {name} PNG: {error}"))
}

pub fn project_battle(
    observation: &BattleObservation,
    actions: &[Action],
    ui: BattleUiState,
    message: &str,
    animation: BattleAnimation,
    display: &BattleDisplayState,
    sprite_frame: usize,
) -> GameView {
    let mut canvas = Canvas::new(SKY);
    canvas.fill(0, 8, CANVAS_WIDTH, 4, DISTANT_GRASS);
    canvas.fill(0, 12, CANVAS_WIDTH, 5, GROUND);
    canvas.ellipse(20, 7, 10, 3, PLATFORM);
    canvas.ellipse(2, 13, 13, 3, PLATFORM);

    let own = observation.own().members()[observation.own().active_slot().index()].clone();
    draw_status_panel(
        &mut canvas,
        1,
        1,
        display.opponent_hp,
        display.opponent_max_hp,
    );
    draw_status_panel(&mut canvas, 18, 11, display.own_hp, display.own_max_hp);
    draw_action_panel(&mut canvas, actions.len(), ui.selected_index);

    let mut labels = vec![
        label(
            TextRole::OpponentName,
            2,
            2,
            12,
            1,
            &display.opponent_name,
            TEXT,
        ),
        label(
            TextRole::OpponentHp,
            2,
            3,
            12,
            1,
            &format!("生命 {}/{}", display.opponent_hp, display.opponent_max_hp),
            MUTED_TEXT,
        ),
        label(TextRole::PlayerName, 19, 12, 12, 1, &display.own_name, TEXT),
        label(
            TextRole::PlayerHp,
            19,
            13,
            12,
            1,
            &format!("生命 {}/{}", display.own_hp, display.own_max_hp),
            MUTED_TEXT,
        ),
    ];
    for (index, action) in actions.iter().copied().enumerate().take(4) {
        let col = 2 + (index as u32 % 2) * 15;
        let row = 18 + (index as u32 / 2) * 2;
        labels.push(label(
            TextRole::Action(index),
            col,
            row,
            13,
            1,
            &action_label(action, observation, &own),
            TEXT,
        ));
    }
    labels.push(label(TextRole::Message, 2, 22, 28, 1, message, MUTED_TEXT));

    GameView {
        surface: canvas.finish(),
        images: battle_images(animation, sprite_frame),
        labels,
    }
}

pub fn world_command_for_key(key: &KeyEvent) -> Option<WorldCommand> {
    if key.phase == KeyPhase::Release {
        return None;
    }
    let direction = match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => WorldDirection::Up,
        LogicalKey::Named(NamedKey::ArrowDown) => WorldDirection::Down,
        LogicalKey::Named(NamedKey::ArrowLeft) => WorldDirection::Left,
        LogicalKey::Named(NamedKey::ArrowRight) => WorldDirection::Right,
        _ => return None,
    };
    Some(WorldCommand::Move(direction))
}

pub fn project_world(observation: &WorldObservation, message: &str) -> GameView {
    project_world_animated(observation, message, WorldAnimation::Stand, 0)
}

pub fn project_world_animated(
    observation: &WorldObservation,
    message: &str,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> GameView {
    project_world_presented(
        observation,
        message,
        animation,
        sprite_frame,
        PixelOffset::new(0, 0),
    )
}

pub fn project_world_presented(
    observation: &WorldObservation,
    message: &str,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> GameView {
    const TILE_SIZE: u32 = 2;
    let mut canvas = Canvas::new(MAP_GROUND);
    for y in 0..observation.height() {
        for x in 0..observation.width() {
            let position = Position::new(x, y);
            let tile = observation
                .tile(position)
                .expect("observed map coordinates are in bounds");
            draw_world_tile(
                &mut canvas,
                u32::from(x) * TILE_SIZE,
                u32::from(y) * TILE_SIZE,
                tile,
            );
        }
    }
    canvas.fill(0, 20, CANVAS_WIDTH, 4, PANEL_EDGE);
    canvas.fill(1, 21, CANVAS_WIDTH - 2, 2, PANEL);

    GameView {
        surface: canvas.finish(),
        images: vec![world_player_image(
            observation.player(),
            observation.facing(),
            animation,
            sprite_frame,
            pixel_offset,
        )],
        labels: vec![
            label(TextRole::Location, 2, 21, 10, 1, "青叶原野", TEXT),
            label(TextRole::Message, 12, 21, 18, 1, message, MUTED_TEXT),
        ],
    }
}

fn draw_world_tile(canvas: &mut Canvas, col: u32, row: u32, tile: Tile) {
    let (base, accent) = match tile {
        Tile::Ground => (MAP_GROUND, MAP_GROUND_LIGHT),
        Tile::Grass => (MAP_GRASS, MAP_GRASS_LIGHT),
        Tile::Wall => (MAP_WALL, MAP_WALL_LIGHT),
    };
    canvas.fill(col, row, 2, 2, base);
    canvas.set(col + 1, row, accent);
}

fn world_player_image(
    position: Position,
    direction: WorldDirection,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> GpuImage {
    GpuImage::new(
        GridRect::new(
            GridPos::new(i32::from(position.x()) * 2, i32::from(position.y()) * 2),
            GridSize::new(2, 2),
        ),
        world_character_resource(direction, animation, sprite_frame),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
    .with_pixel_offset(pixel_offset)
}

const fn world_character_resource(
    direction: WorldDirection,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> ResourceId {
    let direction_index = match direction {
        WorldDirection::Down => 0,
        WorldDirection::Left => 1,
        WorldDirection::Right => 2,
        WorldDirection::Up => 3,
    };
    let frame_offset = match animation {
        WorldAnimation::Stand => 0,
        WorldAnimation::Walk => 1 + sprite_frame % 2,
        WorldAnimation::Run => 4 + sprite_frame % 2,
        WorldAnimation::RunStopping => 3,
    };
    ResourceId(CHARACTER_RESOURCE_START + direction_index * 6 + frame_offset as u32)
}

fn action_label(action: Action, observation: &BattleObservation, own: &Pokemon) -> String {
    match action {
        Action::UseMove(slot) => own.moves().get(slot.index()).map_or_else(
            || "未知招式".into(),
            |battle_move| battle_move.name().into(),
        ),
        Action::Switch(slot) => {
            format!("换上 {}", observation.own().members()[slot.index()].name())
        }
        Action::Struggle => "挣扎".into(),
    }
}

fn draw_status_panel(canvas: &mut Canvas, col: u32, row: u32, hp: u32, max_hp: u32) {
    canvas.fill(col, row, 13, 4, PANEL_EDGE);
    canvas.fill(col + 1, row + 1, 11, 2, PANEL);
    let bar_width = hp.saturating_mul(10).checked_div(max_hp).unwrap_or(0);
    let color = if hp.saturating_mul(4) <= max_hp {
        HP_LOW
    } else {
        HP_GOOD
    };
    canvas.fill(col + 1, row + 3, bar_width, 1, color);
}

fn draw_action_panel(canvas: &mut Canvas, action_count: usize, selected: usize) {
    canvas.fill(0, 17, CANVAS_WIDTH, 7, PANEL_EDGE);
    canvas.fill(1, 18, CANVAS_WIDTH - 2, 5, PANEL);
    for index in 0..action_count.min(4) {
        if index == selected {
            let col = 1 + (index as u32 % 2) * 15;
            let row = 18 + (index as u32 / 2) * 2;
            canvas.fill(col, row, 15, 2, SELECTED);
        }
    }
}

fn battle_images(animation: BattleAnimation, sprite_frame: usize) -> Vec<GpuImage> {
    let player_origin = if animation == BattleAnimation::Acting(Side::One) {
        GridPos::new(6, 9)
    } else {
        GridPos::new(5, 10)
    };
    let opponent_origin = if animation == BattleAnimation::Acting(Side::Two) {
        GridPos::new(21, 5)
    } else {
        GridPos::new(22, 4)
    };

    vec![
        GpuImage::new(
            GridRect::new(player_origin, GridSize::new(8, 8)),
            player_back_resource(sprite_frame),
            creature_tint(animation, Side::One),
            10,
        ),
        GpuImage::new(
            GridRect::new(opponent_origin, GridSize::new(8, 8)),
            opponent_front_resource(sprite_frame),
            creature_tint(animation, Side::Two),
            10,
        ),
    ]
}

const fn player_back_resource(sprite_frame: usize) -> ResourceId {
    if sprite_frame.is_multiple_of(2) {
        PLAYER_BACK_FRAME_0_RESOURCE
    } else {
        PLAYER_BACK_FRAME_1_RESOURCE
    }
}

const fn opponent_front_resource(sprite_frame: usize) -> ResourceId {
    if sprite_frame.is_multiple_of(2) {
        OPPONENT_FRONT_FRAME_0_RESOURCE
    } else {
        OPPONENT_FRONT_FRAME_1_RESOURCE
    }
}

fn creature_tint(animation: BattleAnimation, side: Side) -> Rgba8 {
    match animation {
        BattleAnimation::Hit(target) if target == side => Rgba8::new(255, 112, 112, 255),
        BattleAnimation::Fainted(target) if target == side => Rgba8::new(112, 112, 112, 255),
        _ => Rgba8::new(255, 255, 255, 255),
    }
}

fn label(
    role: TextRole,
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    content: &str,
    color: Rgba8,
) -> TextLabel {
    TextLabel {
        role,
        col,
        row,
        width,
        height,
        content: content.into(),
        color,
    }
}

struct Canvas {
    cells: Vec<GpuCell>,
}

impl Canvas {
    fn new(color: Rgba8) -> Self {
        Self {
            cells: vec![sprite(color); (CANVAS_WIDTH * CANVAS_HEIGHT) as usize],
        }
    }

    fn set(&mut self, col: u32, row: u32, color: Rgba8) {
        if col < CANVAS_WIDTH && row < CANVAS_HEIGHT {
            self.cells[(row * CANVAS_WIDTH + col) as usize] = sprite(color);
        }
    }

    fn fill(&mut self, col: u32, row: u32, width: u32, height: u32, color: Rgba8) {
        for y in row..row.saturating_add(height).min(CANVAS_HEIGHT) {
            for x in col..col.saturating_add(width).min(CANVAS_WIDTH) {
                self.set(x, y, color);
            }
        }
    }

    fn ellipse(&mut self, col: u32, row: u32, width: u32, height: u32, color: Rgba8) {
        let center_x = i64::from(width.saturating_sub(1));
        let center_y = i64::from(height.saturating_sub(1));
        for y in 0..height {
            for x in 0..width {
                let dx = i64::from(x) * 2 - center_x;
                let dy = i64::from(y) * 2 - center_y;
                if dx * dx * i64::from(height * height) + dy * dy * i64::from(width * width)
                    <= i64::from(width * width * height * height)
                {
                    self.set(col + x, row + y, color);
                }
            }
        }
    }

    fn finish(self) -> Surface<GpuCell> {
        Surface::from_cells(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), self.cells)
            .expect("the fixed battle canvas dimensions are valid")
    }
}

const fn sprite(tint: Rgba8) -> GpuCell {
    GpuCell::Sprite {
        resource: WHITE_RESOURCE,
        tint,
    }
}

pub fn move_action(index: usize) -> Action {
    Action::UseMove(MoveSlot::new(index).expect("demo move indexes stay in range"))
}

pub fn switch_action(index: usize) -> Action {
    Action::Switch(TeamSlot::new(index).expect("demo team indexes stay in range"))
}

pub fn phase_message(phase: BattlePhase) -> &'static str {
    match phase {
        BattlePhase::Turn => "请选择行动",
        BattlePhase::ForcedReplacement(_) => "请选择下一只精灵",
        BattlePhase::Finished(_) => "战斗结束",
    }
}

#[cfg(test)]
mod tests {
    use punctum_gpu::ResourceId;
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::{Direction, Position, WorldApplication, WorldCommand};

    use super::{
        BattleUiOutcome, BattleUiState, TextRole, WorldAnimation, move_action, project_world,
        world_character_resource, world_command_for_key,
    };

    fn key(name: NamedKey) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::Unidentified),
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    #[test]
    fn keyboard_focus_wraps_and_enter_submits_the_selected_legal_action() {
        let actions = [move_action(0), move_action(1), move_action(2)];
        let mut ui = BattleUiState::default();

        assert_eq!(
            ui.handle_key(&key(NamedKey::ArrowLeft), &actions),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.selected_index(), 2);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), &actions),
            BattleUiOutcome::Submit(actions[2])
        );
    }

    #[test]
    fn world_projection_and_keyboard_input_share_the_integer_grid() {
        let world = WorldApplication::demo().unwrap();
        let view = project_world(&world.observe(), "风吹过草地。");

        assert_eq!(
            world_command_for_key(&key(NamedKey::ArrowRight)),
            Some(WorldCommand::Move(Direction::Right))
        );
        assert_eq!(world.observe().player(), Position::new(3, 6));
        assert!(
            view.labels().iter().any(|label| {
                label.role == TextRole::Location && label.content == "青叶原野"
            })
        );
        assert_eq!(view.images().len(), 1);
        assert_eq!(
            view.images()[0].resource,
            world_character_resource(Direction::Down, WorldAnimation::Stand, 0)
        );
        assert_eq!(
            world_character_resource(Direction::Up, WorldAnimation::Run, 2),
            ResourceId(28)
        );
        assert_eq!(
            world_character_resource(Direction::Up, WorldAnimation::RunStopping, 99),
            ResourceId(27)
        );
    }
}
