//! Pure battle-screen projection and keyboard focus state.

#![forbid(unsafe_code)]

use battle_application::{
    Action, BattleObservation, BattlePhase, MoveSlot, Pokemon, Side, TeamSlot,
};
use punctum_gpu::{GpuAtlas, GpuCell, GpuResource, PixelRect, PixelSize, ResourceId, Rgba8};
use punctum_grid::{GridSize, Surface};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use world_application::{
    Direction as WorldDirection, Position, Tile, WorldCommand, WorldObservation,
};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;

const WHITE_RESOURCE: ResourceId = ResourceId(1);
const WHITE_PIXEL: [u8; 4] = [255; 4];

const SKY: Rgba8 = Rgba8::new(145, 205, 210, 255);
const DISTANT_GRASS: Rgba8 = Rgba8::new(104, 164, 112, 255);
const GROUND: Rgba8 = Rgba8::new(83, 132, 79, 255);
const PLATFORM: Rgba8 = Rgba8::new(167, 190, 124, 255);
const PANEL: Rgba8 = Rgba8::new(28, 34, 45, 248);
const PANEL_EDGE: Rgba8 = Rgba8::new(218, 225, 214, 255);
const SELECTED: Rgba8 = Rgba8::new(45, 125, 137, 255);
const HP_GOOD: Rgba8 = Rgba8::new(74, 190, 102, 255);
const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
const PLAYER_DARK: Rgba8 = Rgba8::new(22, 79, 82, 255);
const PLAYER_LIGHT: Rgba8 = Rgba8::new(62, 198, 184, 255);
const ENEMY_DARK: Rgba8 = Rgba8::new(117, 48, 54, 255);
const ENEMY_LIGHT: Rgba8 = Rgba8::new(224, 105, 83, 255);
const CREATURE_EYE: Rgba8 = Rgba8::new(242, 239, 214, 255);
const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
const FAINTED_DARK: Rgba8 = Rgba8::new(68, 72, 78, 255);
const FAINTED_LIGHT: Rgba8 = Rgba8::new(120, 124, 128, 255);
const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);
const MAP_GROUND_LIGHT: Rgba8 = Rgba8::new(157, 202, 132, 255);
const MAP_GRASS: Rgba8 = Rgba8::new(54, 137, 79, 255);
const MAP_GRASS_LIGHT: Rgba8 = Rgba8::new(82, 163, 91, 255);
const MAP_WALL: Rgba8 = Rgba8::new(100, 105, 111, 255);
const MAP_WALL_LIGHT: Rgba8 = Rgba8::new(142, 146, 145, 255);
const PLAYER_BODY: Rgba8 = Rgba8::new(35, 87, 115, 255);
const PLAYER_FACE: Rgba8 = Rgba8::new(245, 210, 117, 255);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleAnimation {
    #[default]
    Idle,
    Acting(Side),
    Hit(Side),
    Fainted(Side),
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
    labels: Vec<TextLabel>,
}

impl GameView {
    pub const fn surface(&self) -> &Surface<GpuCell> {
        &self.surface
    }

    pub fn labels(&self) -> &[TextLabel] {
        &self.labels
    }
}

pub fn atlas() -> GpuAtlas {
    GpuAtlas::new(
        PixelSize::new(1, 1),
        WHITE_PIXEL.to_vec(),
        &[GpuResource::new(WHITE_RESOURCE, PixelRect::new(0, 0, 1, 1))],
    )
    .expect("the embedded battle atlas is valid")
}

pub fn project_battle(
    observation: &BattleObservation,
    actions: &[Action],
    ui: BattleUiState,
    message: &str,
    animation: BattleAnimation,
    display: &BattleDisplayState,
) -> GameView {
    let mut canvas = Canvas::new(SKY);
    canvas.fill(0, 8, CANVAS_WIDTH, 4, DISTANT_GRASS);
    canvas.fill(0, 12, CANVAS_WIDTH, 5, GROUND);
    canvas.ellipse(20, 7, 10, 3, PLATFORM);
    canvas.ellipse(2, 13, 13, 3, PLATFORM);
    draw_enemy(&mut canvas, animation);
    draw_player(&mut canvas, animation);

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
    draw_world_player(&mut canvas, observation.player(), observation.facing());
    canvas.fill(0, 20, CANVAS_WIDTH, 4, PANEL_EDGE);
    canvas.fill(1, 21, CANVAS_WIDTH - 2, 2, PANEL);

    GameView {
        surface: canvas.finish(),
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

fn draw_world_player(canvas: &mut Canvas, position: Position, direction: WorldDirection) {
    let col = u32::from(position.x()) * 2;
    let row = u32::from(position.y()) * 2;
    canvas.fill(col, row, 2, 2, PLAYER_BODY);
    let (face_x, face_y) = match direction {
        WorldDirection::Up => (col, row),
        WorldDirection::Down => (col + 1, row + 1),
        WorldDirection::Left => (col, row + 1),
        WorldDirection::Right => (col + 1, row),
    };
    canvas.set(face_x, face_y, PLAYER_FACE);
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

fn draw_player(canvas: &mut Canvas, animation: BattleAnimation) {
    const MASK: [&str; 7] = [
        "..dd....", ".dLLd...", "dLEELd..", "dLLLLdd.", ".dLLLLd.", "..d..d..", ".dd..dd.",
    ];
    let (col, row) = if animation == BattleAnimation::Acting(Side::One) {
        (6, 9)
    } else {
        (5, 10)
    };
    let (dark, light) = creature_colors(animation, Side::One, PLAYER_DARK, PLAYER_LIGHT);
    draw_mask(canvas, col, row, &MASK, dark, light);
}

fn draw_enemy(canvas: &mut Canvas, animation: BattleAnimation) {
    const MASK: [&str; 7] = [
        ".d....d.", "..d..d..", ".dLLLLd.", "dLLEELLd", "dLLLLLLd", ".dLLLLd.", "..dddd..",
    ];
    let (col, row) = if animation == BattleAnimation::Acting(Side::Two) {
        (21, 5)
    } else {
        (22, 4)
    };
    let (dark, light) = creature_colors(animation, Side::Two, ENEMY_DARK, ENEMY_LIGHT);
    draw_mask(canvas, col, row, &MASK, dark, light);
}

fn creature_colors(
    animation: BattleAnimation,
    side: Side,
    dark: Rgba8,
    light: Rgba8,
) -> (Rgba8, Rgba8) {
    match animation {
        BattleAnimation::Hit(target) if target == side => (HP_LOW, CREATURE_EYE),
        BattleAnimation::Fainted(target) if target == side => (FAINTED_DARK, FAINTED_LIGHT),
        _ => (dark, light),
    }
}

fn draw_mask(canvas: &mut Canvas, col: u32, row: u32, mask: &[&str], dark: Rgba8, light: Rgba8) {
    for (y, line) in mask.iter().enumerate() {
        for (x, pixel) in line.chars().enumerate() {
            let color = match pixel {
                'd' => Some(dark),
                'L' => Some(light),
                'E' => Some(CREATURE_EYE),
                _ => None,
            };
            if let Some(color) = color {
                canvas.set(col + x as u32, row + y as u32, color);
            }
        }
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
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::{Direction, Position, WorldApplication, WorldCommand};

    use super::{
        BattleUiOutcome, BattleUiState, TextRole, move_action, project_world, world_command_for_key,
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
    }
}
