//! Pure battle-screen projection and keyboard focus state.

#![forbid(unsafe_code)]

use battle_session::{
    Action, BattleCue, BattleInteraction, BattleObservation, BattleSessionSnapshot, MoveCategory,
    MoveSlot, ObservedBattleOutcome, Participant, Pokemon, PokemonType, TEAM_SIZE, TeamSlot,
    TypeEffectiveness, UsedMove,
};
use game_assets::{AssetError, DecodedImage, build_atlas};
use map_render::{MapCamera, MapScenePlan};
use punctum_gpu::{GpuAtlas, GpuCell, GpuImage, PixelOffset, ResourceId, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use world_application::{Direction as WorldDirection, Position, WorldCommand, WorldObservation};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;

const WHITE_RESOURCE: ResourceId = ResourceId(1);
const CHARACTER_RESOURCE_START: u32 = 6;
const BATTLE_SPRITE_RESOURCE_START: u32 = 30;
const POKEMON_ICON_RESOURCE_START: u32 = BATTLE_SPRITE_RESOURCE_START + TEAM_SIZE as u32 * 4;
const TYPE_ICON_RESOURCE_START: u32 = POKEMON_ICON_RESOURCE_START + TEAM_SIZE as u32 * 2;
const MOVE_CATEGORY_ICON_RESOURCE_START: u32 = TYPE_ICON_RESOURCE_START + 17;
const WHITE_PIXEL: Rgba8 = Rgba8::new(255, 255, 255, 255);
const PLAYER_BACK_FRAME_0_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/back/001_Back_0_C__frame_0.png");
const PLAYER_BACK_FRAME_1_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/back/001_Back_0_C__frame_1.png");
const OPPONENT_FRONT_FRAME_0_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/front/001_Front_0_C__frame_0.png");
const OPPONENT_FRONT_FRAME_1_PNG: &[u8] =
    include_bytes!("../../../assets/pokemons/normal/front/001_Front_0_C__frame_1.png");
const POKEMON_ICON_FRAME_0_PNG: &[u8] = include_bytes!("../../../assets/pokemons/icons/001_0.png");
const POKEMON_ICON_FRAME_1_PNG: &[u8] = include_bytes!("../../../assets/pokemons/icons/001_1.png");
// icon-09 is the removed Generation III ??? type; 18-23 are contest categories.
const TYPE_ICON_PNGS: [&[u8]; 17] = [
    include_bytes!("../../../assets/type-icons/icon-00.png"),
    include_bytes!("../../../assets/type-icons/icon-01.png"),
    include_bytes!("../../../assets/type-icons/icon-02.png"),
    include_bytes!("../../../assets/type-icons/icon-03.png"),
    include_bytes!("../../../assets/type-icons/icon-04.png"),
    include_bytes!("../../../assets/type-icons/icon-05.png"),
    include_bytes!("../../../assets/type-icons/icon-06.png"),
    include_bytes!("../../../assets/type-icons/icon-07.png"),
    include_bytes!("../../../assets/type-icons/icon-08.png"),
    include_bytes!("../../../assets/type-icons/icon-10.png"),
    include_bytes!("../../../assets/type-icons/icon-11.png"),
    include_bytes!("../../../assets/type-icons/icon-12.png"),
    include_bytes!("../../../assets/type-icons/icon-13.png"),
    include_bytes!("../../../assets/type-icons/icon-14.png"),
    include_bytes!("../../../assets/type-icons/icon-15.png"),
    include_bytes!("../../../assets/type-icons/icon-16.png"),
    include_bytes!("../../../assets/type-icons/icon-17.png"),
];
const MOVE_CATEGORY_ICON_PNGS: [&[u8]; 3] = [
    include_bytes!("../../../assets/move-category-icons/physical.png"),
    include_bytes!("../../../assets/move-category-icons/special.png"),
    include_bytes!("../../../assets/move-category-icons/status.png"),
];
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
const HP_MID: Rgba8 = Rgba8::new(226, 177, 66, 255);
const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
const HP_TRACK: Rgba8 = Rgba8::new(68, 74, 82, 255);
const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
const CONSOLE_ERROR: Rgba8 = Rgba8::new(255, 142, 126, 255);
const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleAnimation {
    #[default]
    Idle,
    Acting(Participant),
    Hit(Participant),
    Fainted(Participant),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldAnimation {
    #[default]
    Stand,
    Walk,
    Run,
    RunStopping,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleMenuPage {
    #[default]
    Main,
    Fight,
    Pokemon,
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BattleUiState {
    page: BattleMenuPage,
    selected_index: usize,
    replacement_mode: bool,
    notice: Option<&'static str>,
}

impl BattleUiState {
    pub const fn page(self) -> BattleMenuPage {
        self.page
    }

    pub const fn selected_index(self) -> usize {
        self.selected_index
    }

    pub fn reset(&mut self) {
        self.page = BattleMenuPage::Main;
        self.selected_index = 0;
        self.replacement_mode = false;
        self.notice = None;
    }

    pub fn sync_interaction(&mut self, interaction: &BattleInteraction) {
        match interaction {
            BattleInteraction::ChooseAction(_)
                if self.replacement_mode || self.page == BattleMenuPage::Hidden =>
            {
                self.reset();
            }
            BattleInteraction::ChooseReplacement(prompt) if !self.replacement_mode => {
                self.page = BattleMenuPage::Pokemon;
                self.replacement_mode = true;
                self.notice = None;
                self.selected_index = prompt
                    .legal_actions()
                    .iter()
                    .find_map(|action| match action {
                        Action::Switch(slot) => Some(*slot),
                        _ => None,
                    })
                    .map_or(0, TeamSlot::index);
            }
            BattleInteraction::PlaybackLocked | BattleInteraction::Finished(_) => {
                self.page = BattleMenuPage::Hidden;
                self.notice = None;
            }
            BattleInteraction::ChooseAction(_) | BattleInteraction::ChooseReplacement(_) => {}
        }
    }

    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        interaction: &BattleInteraction,
    ) -> BattleUiOutcome {
        self.sync_interaction(interaction);
        let Some((observation, actions)) = prompt_data(interaction) else {
            return BattleUiOutcome::Ignored;
        };
        if key.phase == KeyPhase::Release {
            return BattleUiOutcome::Ignored;
        }
        let item_count = self.item_count(observation, actions);
        if item_count == 0 {
            return BattleUiOutcome::Ignored;
        }
        self.selected_index = self.selected_index.min(item_count - 1);
        self.notice = None;
        match key.logical {
            LogicalKey::Named(NamedKey::ArrowLeft) => {
                self.selected_index = (self.selected_index + item_count - 1) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::ArrowRight) => {
                self.selected_index = (self.selected_index + 1) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::ArrowUp) => {
                self.selected_index =
                    (self.selected_index + item_count - 2 % item_count) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::ArrowDown) => {
                self.selected_index = (self.selected_index + 2) % item_count;
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::Escape)
                if self.page != BattleMenuPage::Main && !self.replacement_mode =>
            {
                self.reset();
                BattleUiOutcome::Updated
            }
            LogicalKey::Named(NamedKey::Enter) if key.phase == KeyPhase::Press => {
                self.activate(observation, actions)
            }
            _ => BattleUiOutcome::Ignored,
        }
    }

    fn item_count(self, observation: &BattleObservation, actions: &[Action]) -> usize {
        match self.page {
            BattleMenuPage::Main => 4,
            BattleMenuPage::Fight => {
                if actions.contains(&Action::Struggle) {
                    1
                } else {
                    active_pokemon(observation).moves().len()
                }
            }
            BattleMenuPage::Pokemon => observation.own().members().len(),
            BattleMenuPage::Hidden => 0,
        }
    }

    fn activate(&mut self, observation: &BattleObservation, actions: &[Action]) -> BattleUiOutcome {
        match self.page {
            BattleMenuPage::Main => match self.selected_index {
                0 => {
                    self.page = BattleMenuPage::Fight;
                    self.selected_index = 0;
                    BattleUiOutcome::Updated
                }
                1 => {
                    self.page = BattleMenuPage::Pokemon;
                    self.selected_index = observation.own().active_slot().index();
                    BattleUiOutcome::Updated
                }
                2 => {
                    self.notice = Some("包包现在还不能使用。");
                    BattleUiOutcome::Updated
                }
                3 => actions
                    .iter()
                    .copied()
                    .find(|action| *action == Action::Run)
                    .map_or_else(
                        || {
                            self.notice = Some("现在无法逃走。");
                            BattleUiOutcome::Updated
                        },
                        BattleUiOutcome::Submit,
                    ),
                _ => BattleUiOutcome::Ignored,
            },
            BattleMenuPage::Fight => {
                let action = if actions.contains(&Action::Struggle) {
                    Action::Struggle
                } else {
                    Action::UseMove(
                        MoveSlot::new(self.selected_index)
                            .expect("visible move indexes stay within the move limit"),
                    )
                };
                if actions.contains(&action) {
                    BattleUiOutcome::Submit(action)
                } else {
                    self.notice = Some("这个招式的 PP 已用完。");
                    BattleUiOutcome::Updated
                }
            }
            BattleMenuPage::Pokemon => {
                let action = Action::Switch(
                    TeamSlot::new(self.selected_index)
                        .expect("team page indexes stay within the team limit"),
                );
                if actions.contains(&action) {
                    BattleUiOutcome::Submit(action)
                } else if observation.own().active_slot().index() == self.selected_index {
                    self.notice = Some("这只宝可梦正在战斗。");
                    BattleUiOutcome::Updated
                } else {
                    self.notice = Some("这只宝可梦已经无法战斗。");
                    BattleUiOutcome::Updated
                }
            }
            BattleMenuPage::Hidden => BattleUiOutcome::Ignored,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleUiOutcome {
    Updated,
    Submit(Action),
    Ignored,
}

fn prompt_data(interaction: &BattleInteraction) -> Option<(&BattleObservation, &[Action])> {
    match interaction {
        BattleInteraction::ChooseAction(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::ChooseReplacement(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::PlaybackLocked | BattleInteraction::Finished(_) => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Location,
    OpponentName,
    OpponentDetail,
    OpponentHp,
    PlayerName,
    PlayerDetail,
    PlayerHp,
    Action(usize),
    ActionDetail(usize),
    PageTitle,
    TeamMember(usize),
    TeamMemberHp(usize),
    TeamMemberType(usize),
    Message,
    ConsoleQuery,
    ConsoleItem(usize),
    ConsoleDiagnostic,
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

    pub fn replace_world_background(&mut self, scene: &MapScenePlan, camera: MapCamera) {
        assert_eq!(self.surface.size(), scene.base().size());
        self.surface = scene.base().clone();
        for image in &mut self.images {
            image.bounds.origin.col -= camera.col * 2;
            image.bounds.origin.row -= camera.row * 2;
        }
        let mut images = scene.tile_images().to_vec();
        images.append(&mut self.images);
        self.images = images;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandConsoleView {
    pub query: String,
    pub preedit: String,
    pub items: Vec<String>,
    pub selected_index: Option<usize>,
    pub diagnostic: Option<String>,
}

pub fn overlay_command_console(view: &mut GameView, console: &CommandConsoleView) {
    const PANEL_COL: u32 = 1;
    const PANEL_ROW: u32 = 4;
    const PANEL_WIDTH: u32 = 30;
    const PANEL_HEIGHT: u32 = 16;
    const FIRST_ITEM_ROW: u32 = 8;
    const MAX_ITEMS: usize = 8;

    let panel = GridRect::new(
        GridPos::new(PANEL_COL as i32, PANEL_ROW as i32),
        GridSize::new(PANEL_WIDTH, PANEL_HEIGHT),
    );
    view.surface
        .fill_rect(panel, sprite(PANEL_EDGE))
        .expect("the fixed console panel fits the game canvas");
    view.surface
        .fill_rect(
            GridRect::new(
                GridPos::new((PANEL_COL + 1) as i32, (PANEL_ROW + 1) as i32),
                GridSize::new(PANEL_WIDTH - 2, PANEL_HEIGHT - 2),
            ),
            sprite(PANEL),
        )
        .expect("the fixed console body fits the game canvas");

    view.labels.retain(|label| {
        label.row.saturating_add(label.height) <= PANEL_ROW
            || label.row >= PANEL_ROW.saturating_add(PANEL_HEIGHT)
    });
    view.labels.push(label(
        TextRole::ConsoleQuery,
        3,
        6,
        26,
        1,
        &format!("> {}{}", console.query, console.preedit),
        TEXT,
    ));

    let first_visible =
        visible_console_start(console.items.len(), console.selected_index, MAX_ITEMS);
    for (visible_index, (item_index, item)) in console
        .items
        .iter()
        .enumerate()
        .skip(first_visible)
        .take(MAX_ITEMS)
        .enumerate()
    {
        let row = FIRST_ITEM_ROW + visible_index as u32;
        if console.selected_index == Some(item_index) {
            view.surface
                .fill_rect(
                    GridRect::new(GridPos::new(2, row as i32), GridSize::new(28, 1)),
                    sprite(SELECTED),
                )
                .expect("the fixed console selection fits the game canvas");
        }
        view.labels.push(label(
            TextRole::ConsoleItem(item_index),
            3,
            row,
            26,
            1,
            item,
            TEXT,
        ));
    }

    if console.items.is_empty() {
        view.labels.push(label(
            TextRole::ConsoleItem(0),
            3,
            FIRST_ITEM_ROW,
            26,
            1,
            "没有匹配指令",
            MUTED_TEXT,
        ));
    }
    if let Some(diagnostic) = &console.diagnostic {
        view.labels.push(label(
            TextRole::ConsoleDiagnostic,
            3,
            18,
            26,
            1,
            diagnostic,
            CONSOLE_ERROR,
        ));
    }
}

fn visible_console_start(
    item_count: usize,
    selected_index: Option<usize>,
    visible_count: usize,
) -> usize {
    if visible_count == 0 {
        return 0;
    }
    selected_index
        .map_or(0, |selected| {
            selected.saturating_add(1).saturating_sub(visible_count)
        })
        .min(item_count.saturating_sub(visible_count))
}

pub fn atlas() -> GpuAtlas {
    let player_back_frame_0 = decode_embedded_png(PLAYER_BACK_FRAME_0_PNG, "player back frame 0");
    let player_back_frame_1 = decode_embedded_png(PLAYER_BACK_FRAME_1_PNG, "player back frame 1");
    let opponent_front_frame_0 =
        decode_embedded_png(OPPONENT_FRONT_FRAME_0_PNG, "opponent front frame 0");
    let opponent_front_frame_1 =
        decode_embedded_png(OPPONENT_FRONT_FRAME_1_PNG, "opponent front frame 1");
    let pokemon_icon_frame_0 =
        decode_embedded_png(POKEMON_ICON_FRAME_0_PNG, "pokemon icon frame 0");
    let pokemon_icon_frame_1 =
        decode_embedded_png(POKEMON_ICON_FRAME_1_PNG, "pokemon icon frame 1");
    let mut battle_images = Vec::new();
    let mut pokemon_icon_images = Vec::new();
    for slot in 0..TEAM_SIZE {
        battle_images.push((player_back_resource(slot, 0), &player_back_frame_0));
        battle_images.push((player_back_resource(slot, 1), &player_back_frame_1));
        battle_images.push((opponent_front_resource(slot, 0), &opponent_front_frame_0));
        battle_images.push((opponent_front_resource(slot, 1), &opponent_front_frame_1));
        pokemon_icon_images.push((pokemon_icon_resource(slot, 0), &pokemon_icon_frame_0));
        pokemon_icon_images.push((pokemon_icon_resource(slot, 1), &pokemon_icon_frame_1));
    }
    atlas_with_battle_map_and_pokemon_icons(&battle_images, &pokemon_icon_images, &[])
        .expect("the embedded game atlas is valid")
}

pub fn atlas_with_battle_sprites(
    battle_images: &[(ResourceId, &DecodedImage)],
) -> Result<GpuAtlas, AssetError> {
    atlas_with_battle_and_map_sprites(battle_images, &[])
}

pub fn atlas_with_battle_and_map_sprites(
    battle_images: &[(ResourceId, &DecodedImage)],
    map_images: &[(ResourceId, &DecodedImage)],
) -> Result<GpuAtlas, AssetError> {
    atlas_with_battle_map_and_pokemon_icons(battle_images, &[], map_images)
}

pub fn atlas_with_battle_map_and_pokemon_icons(
    battle_images: &[(ResourceId, &DecodedImage)],
    pokemon_icon_images: &[(ResourceId, &DecodedImage)],
    map_images: &[(ResourceId, &DecodedImage)],
) -> Result<GpuAtlas, AssetError> {
    let white = DecodedImage::solid(WHITE_PIXEL);
    let character_images: Vec<_> = CHARACTER_PNGS
        .iter()
        .map(|(name, bytes)| decode_embedded_png(bytes, name))
        .collect();
    let type_icon_images: Vec<_> = TYPE_ICON_PNGS
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            game_assets::decode_png(bytes)
                .unwrap_or_else(|error| panic!("embedded type icon {index} PNG: {error}"))
        })
        .collect();
    let move_category_icon_images: Vec<_> = MOVE_CATEGORY_ICON_PNGS
        .iter()
        .enumerate()
        .map(|(index, bytes)| {
            game_assets::decode_png(bytes)
                .unwrap_or_else(|error| panic!("embedded move category icon {index} PNG: {error}"))
        })
        .collect();
    let mut images = vec![(WHITE_RESOURCE, &white)];
    images.extend(
        character_images
            .iter()
            .enumerate()
            .map(|(index, image)| (ResourceId(CHARACTER_RESOURCE_START + index as u32), image)),
    );
    images.extend_from_slice(battle_images);
    images.extend_from_slice(pokemon_icon_images);
    images.extend(
        type_icon_images
            .iter()
            .enumerate()
            .map(|(index, image)| (ResourceId(TYPE_ICON_RESOURCE_START + index as u32), image)),
    );
    images.extend(
        move_category_icon_images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                (
                    ResourceId(MOVE_CATEGORY_ICON_RESOURCE_START + index as u32),
                    image,
                )
            }),
    );
    images.extend_from_slice(map_images);
    build_atlas(&images)
}

fn decode_embedded_png(bytes: &[u8], name: &str) -> DecodedImage {
    game_assets::decode_png(bytes).unwrap_or_else(|error| panic!("embedded {name} PNG: {error}"))
}

pub fn project_battle(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> GameView {
    let prompt = prompt_data(snapshot.interaction());
    let message = ui
        .notice
        .map(str::to_owned)
        .unwrap_or_else(|| battle_message(snapshot));
    if ui.page == BattleMenuPage::Pokemon
        && let Some((observation, _)) = prompt
    {
        return project_pokemon_page(observation, ui, &message, sprite_frame);
    }

    let scene = snapshot.scene();
    let own = scene.own();
    let opponent = scene.opponent();
    let mut canvas = Canvas::new(SKY);
    canvas.fill(0, 8, CANVAS_WIDTH, 4, DISTANT_GRASS);
    canvas.fill(0, 12, CANVAS_WIDTH, 5, GROUND);
    canvas.ellipse(20, 7, 10, 3, PLATFORM);
    canvas.ellipse(2, 13, 13, 3, PLATFORM);

    draw_status_panel(&mut canvas, 1, 1, opponent.current_hp(), opponent.max_hp());
    draw_status_panel(&mut canvas, 16, 11, own.current_hp(), own.max_hp());
    let actions = prompt.map_or(&[][..], |(_, actions)| actions);
    let observation = prompt.map(|(observation, _)| observation);
    let action_count = match ui.page {
        BattleMenuPage::Main => 4,
        BattleMenuPage::Fight => {
            if actions.contains(&Action::Struggle) {
                1
            } else {
                observation.map_or(0, |observation| active_pokemon(observation).moves().len())
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => 0,
    };
    draw_action_panel(&mut canvas, action_count, ui.selected_index);
    let mut images = battle_images(battle_animation(snapshot.cue()), sprites, sprite_frame);
    images.extend(type_icon_images(
        9,
        3,
        opponent.primary_type(),
        opponent.secondary_type(),
    ));
    images.extend(type_icon_images(
        24,
        13,
        own.primary_type(),
        own.secondary_type(),
    ));

    let mut labels = vec![
        label(TextRole::OpponentName, 2, 2, 10, 1, opponent.name(), TEXT),
        label(
            TextRole::OpponentDetail,
            2,
            3,
            13,
            1,
            &format!("Lv.{}", opponent.level()),
            MUTED_TEXT,
        ),
        label(
            TextRole::OpponentHp,
            2,
            4,
            13,
            1,
            &format!("HP {}/{}", opponent.current_hp(), opponent.max_hp()),
            MUTED_TEXT,
        ),
        label(TextRole::PlayerName, 17, 12, 10, 1, own.name(), TEXT),
        label(
            TextRole::PlayerDetail,
            17,
            13,
            13,
            1,
            &format!("Lv.{}", own.level()),
            MUTED_TEXT,
        ),
        label(
            TextRole::PlayerHp,
            17,
            14,
            13,
            1,
            &format!("HP {}/{}", own.current_hp(), own.max_hp()),
            MUTED_TEXT,
        ),
    ];
    match ui.page {
        BattleMenuPage::Main => {
            for (index, content) in ["战斗", "宝可梦", "包包", "逃走"].into_iter().enumerate()
            {
                let col = 2 + (index as u32 % 2) * 15;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    13,
                    1,
                    content,
                    TEXT,
                ));
            }
        }
        BattleMenuPage::Fight if actions.contains(&Action::Struggle) => {
            labels.push(label(TextRole::Action(0), 2, 18, 13, 1, "挣扎", TEXT));
            images.push(type_icon_image(2, 19, PokemonType::Normal));
            images.push(move_category_icon_image(4, 19, MoveCategory::Physical));
            labels.push(label(
                TextRole::ActionDetail(0),
                7,
                19,
                8,
                1,
                "威50 PP--",
                MUTED_TEXT,
            ));
        }
        BattleMenuPage::Fight => {
            let moves = observation
                .map(active_pokemon)
                .map_or(&[][..], |pokemon| pokemon.moves());
            for (index, battle_move) in moves.iter().enumerate().take(4) {
                let col = 2 + (index as u32 % 2) * 15;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    13,
                    1,
                    battle_move.name(),
                    TEXT,
                ));
                images.push(type_icon_image(col, row + 1, battle_move.move_type()));
                images.push(move_category_icon_image(
                    col + 2,
                    row + 1,
                    battle_move.category(),
                ));
                labels.push(label(
                    TextRole::ActionDetail(index),
                    col + 5,
                    row + 1,
                    8,
                    1,
                    &format!(
                        "威{} PP{}/{}",
                        battle_move.power(),
                        battle_move.current_pp(),
                        battle_move.max_pp()
                    ),
                    MUTED_TEXT,
                ));
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => {}
    }
    labels.push(label(TextRole::Message, 2, 22, 28, 1, &message, MUTED_TEXT));

    GameView {
        surface: canvas.finish(),
        images,
        labels,
    }
}

fn battle_animation(cue: Option<&BattleCue>) -> BattleAnimation {
    match cue {
        Some(BattleCue::MoveUsed { participant, .. }) => BattleAnimation::Acting(*participant),
        Some(BattleCue::DamageApplied { participant, .. })
        | Some(BattleCue::Critical { participant }) => BattleAnimation::Hit(*participant),
        Some(BattleCue::Fainted { participant }) => BattleAnimation::Fainted(*participant),
        _ => BattleAnimation::Idle,
    }
}

fn battle_message(snapshot: &BattleSessionSnapshot) -> String {
    let scene = snapshot.scene();
    match snapshot.cue() {
        Some(BattleCue::TurnStarted { turn }) => format!("第 {turn} 回合"),
        Some(BattleCue::Switched { participant }) => {
            format!("{} 上场了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::MoveUsed {
            participant,
            used_move,
        }) => format!(
            "{} 使用了 {}！",
            combatant_name(scene, *participant),
            used_move_name(used_move)
        ),
        Some(BattleCue::DamageApplied {
            participant,
            amount,
        }) => format!(
            "{} 受到 {} 点伤害。",
            combatant_name(scene, *participant),
            amount
        ),
        Some(BattleCue::Missed { .. }) => "攻击没有命中。".into(),
        Some(BattleCue::Critical { .. }) => "会心一击！".into(),
        Some(BattleCue::Effectiveness { effectiveness, .. }) => {
            effectiveness_message(*effectiveness).into()
        }
        Some(BattleCue::Fainted { participant }) => {
            format!("{} 倒下了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::ReplacementRequired { .. }) => "请选择下一只宝可梦".into(),
        Some(BattleCue::BattleFinished { outcome }) => outcome_message(*outcome).into(),
        None => match snapshot.interaction() {
            BattleInteraction::ChooseAction(_) => "请选择行动".into(),
            BattleInteraction::ChooseReplacement(_) => "请选择下一只宝可梦".into(),
            BattleInteraction::PlaybackLocked => String::new(),
            BattleInteraction::Finished(prompt) => outcome_message(prompt.outcome()).into(),
        },
    }
}

fn combatant_name(scene: &battle_session::BattleScene, participant: Participant) -> &str {
    match participant {
        Participant::Own => scene.own().name(),
        Participant::Opponent => scene.opponent().name(),
    }
}

fn used_move_name(used_move: &UsedMove) -> &str {
    match used_move {
        UsedMove::Move { name, .. } => name,
        UsedMove::Struggle => "挣扎",
    }
}

fn outcome_message(outcome: ObservedBattleOutcome) -> &'static str {
    match outcome {
        ObservedBattleOutcome::Winner(Participant::Own) => "你赢了！",
        ObservedBattleOutcome::Winner(Participant::Opponent) => "对手赢了。",
        ObservedBattleOutcome::Escaped(Participant::Own) => "成功逃走了！",
        ObservedBattleOutcome::Escaped(Participant::Opponent) => "对手逃走了。",
        ObservedBattleOutcome::Draw => "战斗平局。",
    }
}

fn effectiveness_message(effectiveness: TypeEffectiveness) -> &'static str {
    match effectiveness {
        TypeEffectiveness::Immune => "没有效果。",
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => "效果不太好……",
        TypeEffectiveness::Normal => "命中了。",
        TypeEffectiveness::Double | TypeEffectiveness::Quadruple => "效果绝佳！",
    }
}

fn project_pokemon_page(
    observation: &BattleObservation,
    ui: BattleUiState,
    message: &str,
    sprite_frame: usize,
) -> GameView {
    let mut canvas = Canvas::new(PANEL);
    canvas.fill(0, 0, CANVAS_WIDTH, 2, PANEL_EDGE);
    canvas.fill(1, 1, CANVAS_WIDTH - 2, 1, SELECTED);
    let mut labels = vec![label(TextRole::PageTitle, 2, 1, 28, 1, "宝可梦", TEXT)];
    let mut images = Vec::with_capacity(observation.own().members().len() * 3);
    for (index, pokemon) in observation.own().members().iter().enumerate() {
        let col = 1 + (index as u32 % 2) * 15;
        let row = 3 + (index as u32 / 2) * 6;
        let selected = index == ui.selected_index;
        draw_team_card(&mut canvas, col, row, selected, pokemon);
        images.push(pokemon_icon_image(
            col,
            row,
            index,
            pokemon.is_fainted(),
            sprite_frame,
        ));
        images.extend(type_icon_images(
            col + 9,
            row + 2,
            pokemon.primary_type(),
            pokemon.secondary_type(),
        ));
        let active = index == observation.own().active_slot().index();
        labels.push(label(
            TextRole::TeamMember(index),
            col + 4,
            row + 1,
            9,
            1,
            &if active {
                format!("{} 上场", pokemon.name())
            } else {
                pokemon.name().to_owned()
            },
            if pokemon.is_fainted() {
                MUTED_TEXT
            } else {
                TEXT
            },
        ));
        labels.push(label(
            TextRole::TeamMemberType(index),
            col + 4,
            row + 2,
            9,
            1,
            &format!("Lv.{}", pokemon.level()),
            MUTED_TEXT,
        ));
        labels.push(label(
            TextRole::TeamMemberHp(index),
            col + 4,
            row + 3,
            9,
            1,
            &if pokemon.is_fainted() {
                "无法战斗".into()
            } else {
                format!("HP {}/{}", pokemon.current_hp(), pokemon.max_hp())
            },
            if pokemon.is_fainted() {
                HP_LOW
            } else {
                MUTED_TEXT
            },
        ));
    }
    labels.push(label(TextRole::Message, 2, 22, 28, 1, message, MUTED_TEXT));
    GameView {
        surface: canvas.finish(),
        images,
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

pub fn project_world(observation: &WorldObservation) -> GameView {
    project_world_animated(observation, WorldAnimation::Stand, 0)
}

pub fn project_world_animated(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> GameView {
    project_world_presented(observation, animation, sprite_frame, PixelOffset::new(0, 0))
}

pub fn project_world_presented(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> GameView {
    GameView {
        surface: Canvas::new(MAP_GROUND).finish(),
        images: vec![world_player_image(
            observation.player(),
            observation.facing(),
            animation,
            sprite_frame,
            pixel_offset,
        )],
        labels: Vec::new(),
    }
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
        WorldAnimation::Walk => match sprite_frame % 4 {
            0 => 1,
            1 | 3 => 0,
            _ => 2,
        },
        WorldAnimation::Run => match sprite_frame % 4 {
            0 => 4,
            1 | 3 => 3,
            _ => 5,
        },
        WorldAnimation::RunStopping => 3,
    };
    ResourceId(CHARACTER_RESOURCE_START + direction_index * 6 + frame_offset as u32)
}

fn active_pokemon(observation: &BattleObservation) -> &Pokemon {
    &observation.own().members()[observation.own().active_slot().index()]
}

fn draw_status_panel(canvas: &mut Canvas, col: u32, row: u32, hp: u32, max_hp: u32) {
    canvas.fill(col, row, 15, 5, PANEL_EDGE);
    canvas.fill(col + 1, row + 1, 13, 3, PANEL);
    draw_hp_bar(canvas, col + 1, row + 4, 13, hp, max_hp);
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

fn draw_team_card(canvas: &mut Canvas, col: u32, row: u32, selected: bool, pokemon: &Pokemon) {
    canvas.fill(
        col,
        row,
        14,
        6,
        if selected { SELECTED } else { PANEL_EDGE },
    );
    canvas.fill(col + 1, row + 1, 12, 4, PANEL);
    draw_hp_bar(
        canvas,
        col + 4,
        row + 4,
        9,
        pokemon.current_hp(),
        pokemon.max_hp(),
    );
}

fn pokemon_icon_image(
    col: u32,
    row: u32,
    slot: usize,
    fainted: bool,
    sprite_frame: usize,
) -> GpuImage {
    GpuImage::new(
        GridRect::new(
            GridPos::new((col + 1) as i32, (row + 1) as i32),
            GridSize::new(3, 3),
        ),
        pokemon_icon_resource(slot, sprite_frame),
        if fainted {
            Rgba8::new(112, 112, 112, 255)
        } else {
            Rgba8::new(255, 255, 255, 255)
        },
        10,
    )
}

fn draw_hp_bar(canvas: &mut Canvas, col: u32, row: u32, width: u32, hp: u32, max_hp: u32) {
    canvas.fill(col, row, width, 1, HP_TRACK);
    let filled = hp.saturating_mul(width).checked_div(max_hp).unwrap_or(0);
    let color = if hp.saturating_mul(4) <= max_hp {
        HP_LOW
    } else if hp.saturating_mul(2) <= max_hp {
        HP_MID
    } else {
        HP_GOOD
    };
    canvas.fill(col, row, filled.min(width), 1, color);
}

fn type_icon_images(
    col: u32,
    row: u32,
    primary: PokemonType,
    secondary: Option<PokemonType>,
) -> Vec<GpuImage> {
    let mut images = vec![type_icon_image(col, row, primary)];
    if let Some(secondary) = secondary {
        images.push(type_icon_image(col + 2, row, secondary));
    }
    images
}

fn type_icon_image(col: u32, row: u32, pokemon_type: PokemonType) -> GpuImage {
    GpuImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        type_icon_resource(pokemon_type),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

const fn type_icon_resource(pokemon_type: PokemonType) -> ResourceId {
    let index = match pokemon_type {
        PokemonType::Normal => 0,
        PokemonType::Fighting => 1,
        PokemonType::Flying => 2,
        PokemonType::Poison => 3,
        PokemonType::Ground => 4,
        PokemonType::Rock => 5,
        PokemonType::Bug => 6,
        PokemonType::Ghost => 7,
        PokemonType::Steel => 8,
        PokemonType::Fire => 9,
        PokemonType::Water => 10,
        PokemonType::Grass => 11,
        PokemonType::Electric => 12,
        PokemonType::Psychic => 13,
        PokemonType::Ice => 14,
        PokemonType::Dragon => 15,
        PokemonType::Dark => 16,
    };
    ResourceId(TYPE_ICON_RESOURCE_START + index)
}

fn move_category_icon_image(col: u32, row: u32, category: MoveCategory) -> GpuImage {
    GpuImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        move_category_icon_resource(category),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

const fn move_category_icon_resource(category: MoveCategory) -> ResourceId {
    let index = match category {
        MoveCategory::Physical => 0,
        MoveCategory::Special => 1,
        MoveCategory::Status => 2,
    };
    ResourceId(MOVE_CATEGORY_ICON_RESOURCE_START + index)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleSpriteResources {
    own: [ResourceId; 2],
    opponent: [ResourceId; 2],
}

impl BattleSpriteResources {
    pub const fn for_slots(own_slot: usize, opponent_slot: usize) -> Self {
        Self {
            own: [
                player_back_resource(own_slot, 0),
                player_back_resource(own_slot, 1),
            ],
            opponent: [
                opponent_front_resource(opponent_slot, 0),
                opponent_front_resource(opponent_slot, 1),
            ],
        }
    }
}

pub const fn player_back_resource(slot: usize, frame: usize) -> ResourceId {
    ResourceId(BATTLE_SPRITE_RESOURCE_START + slot as u32 * 2 + (frame % 2) as u32)
}

pub const fn opponent_front_resource(slot: usize, frame: usize) -> ResourceId {
    ResourceId(
        BATTLE_SPRITE_RESOURCE_START + TEAM_SIZE as u32 * 2 + slot as u32 * 2 + (frame % 2) as u32,
    )
}

pub const fn pokemon_icon_resource(slot: usize, frame: usize) -> ResourceId {
    ResourceId(POKEMON_ICON_RESOURCE_START + slot as u32 * 2 + (frame % 2) as u32)
}

fn battle_images(
    animation: BattleAnimation,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Vec<GpuImage> {
    let player_origin = if animation == BattleAnimation::Acting(Participant::Own) {
        GridPos::new(6, 9)
    } else {
        GridPos::new(5, 10)
    };
    let opponent_origin = if animation == BattleAnimation::Acting(Participant::Opponent) {
        GridPos::new(21, 5)
    } else {
        GridPos::new(22, 4)
    };

    vec![
        GpuImage::new(
            GridRect::new(player_origin, GridSize::new(8, 8)),
            sprites.own[sprite_frame % 2],
            creature_tint(animation, Participant::Own),
            10,
        ),
        GpuImage::new(
            GridRect::new(opponent_origin, GridSize::new(8, 8)),
            sprites.opponent[sprite_frame % 2],
            creature_tint(animation, Participant::Opponent),
            10,
        ),
    ]
}

fn creature_tint(animation: BattleAnimation, participant: Participant) -> Rgba8 {
    match animation {
        BattleAnimation::Hit(target) if target == participant => Rgba8::new(255, 112, 112, 255),
        BattleAnimation::Fainted(target) if target == participant => Rgba8::new(112, 112, 112, 255),
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

#[cfg(test)]
mod tests {
    use battle_application::{
        Accuracy, BattleApplication, BattleStats, Move, MoveCategory, MoveId, Pokemon, PokemonId,
        PokemonType, TEAM_SIZE, Team,
    };
    use battle_session::{
        Action, BattleCoordinator, BattleObservation, BattleSession, BattleSessionSnapshot,
        OpponentPolicy,
    };
    use punctum_gpu::ResourceId;
    use punctum_grid::{GridPos, GridSize};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::{Direction, Position, WorldApplication, WorldCommand};

    use super::{
        BattleMenuPage, BattleSpriteResources, BattleUiOutcome, BattleUiState, CommandConsoleView,
        TextRole, WorldAnimation, move_action, overlay_command_console, project_battle,
        project_world, switch_action, world_character_resource, world_command_for_key,
    };

    fn key(name: NamedKey) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::Unidentified),
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    struct FirstActionPolicy;

    impl OpponentPolicy for FirstActionPolicy {
        fn choose_action(
            &mut self,
            _observation: &BattleObservation,
            legal_actions: &[Action],
        ) -> Option<Action> {
            legal_actions.first().copied()
        }
    }

    fn battle_fixture() -> BattleSessionSnapshot {
        fn team(prefix: &str, move_type: PokemonType) -> Team {
            let members = (0..TEAM_SIZE)
                .map(|index| {
                    let battle_move = Move::new(
                        MoveId::new(format!("{prefix}-move-{index}")).unwrap(),
                        if index == 0 { "撞击" } else { "电光一闪" },
                        move_type,
                        40,
                        Accuracy::AlwaysHit,
                        35,
                        35,
                        0,
                    )
                    .unwrap();
                    Pokemon::new(
                        PokemonId::new(format!("{prefix}-{index}")).unwrap(),
                        format!("{prefix}{index}"),
                        24,
                        move_type,
                        (index == 0).then_some(PokemonType::Poison),
                        80 + index as u32,
                        80 + index as u32,
                        BattleStats::new(50, 50, 50, 50, 50).unwrap(),
                        vec![battle_move],
                    )
                    .unwrap()
                })
                .collect();
            Team::new(members).unwrap()
        }

        let application = BattleApplication::new(
            team("己方", PokemonType::Grass),
            team("对手", PokemonType::Fire),
            42,
        )
        .unwrap();
        BattleSession::new(BattleCoordinator::new(application, FirstActionPolicy)).snapshot()
    }

    #[test]
    fn main_menu_routes_to_fight_pokemon_bag_and_run() {
        let snapshot = battle_fixture();
        let interaction = snapshot.interaction();
        let mut ui = BattleUiState::default();

        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.page(), BattleMenuPage::Fight);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(move_action(0))
        );

        ui.reset();
        assert_eq!(
            ui.handle_key(&key(NamedKey::ArrowRight), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.page(), BattleMenuPage::Pokemon);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        ui.handle_key(&key(NamedKey::ArrowRight), interaction);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(switch_action(1))
        );

        ui.reset();
        ui.handle_key(&key(NamedKey::ArrowRight), interaction);
        ui.handle_key(&key(NamedKey::ArrowRight), interaction);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.page(), BattleMenuPage::Main);

        ui.reset();
        ui.handle_key(&key(NamedKey::ArrowLeft), interaction);
        assert_eq!(ui.selected_index(), 3);
        assert_eq!(
            ui.handle_key(&key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(Action::Run)
        );
    }

    #[test]
    fn battle_projection_shows_status_and_move_details() {
        let snapshot = battle_fixture();
        let mut ui = BattleUiState::default();
        let main = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
        let commands = main
            .labels()
            .iter()
            .filter(|label| matches!(label.role, TextRole::Action(_)))
            .map(|label| label.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(commands, ["战斗", "宝可梦", "包包", "逃走"]);
        assert!(
            main.labels()
                .iter()
                .any(|label| { label.role == TextRole::PlayerDetail && label.content == "Lv.24" })
        );
        assert!(
            main.images()
                .iter()
                .any(|image| image.resource == super::type_icon_resource(PokemonType::Grass))
        );
        assert!(
            main.images()
                .iter()
                .any(|image| image.resource == super::type_icon_resource(PokemonType::Poison))
        );
        assert!(
            main.labels()
                .iter()
                .any(|label| { label.role == TextRole::PlayerHp && label.content == "HP 80/80" })
        );

        ui.handle_key(&key(NamedKey::Enter), snapshot.interaction());
        let fight = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
        assert!(fight.labels().iter().any(|label| {
            label.role == TextRole::ActionDetail(0) && label.content == "威40 PP35/35"
        }));
        assert!(fight.images().iter().any(|image| {
            image.resource == super::type_icon_resource(PokemonType::Grass)
                && image.bounds.origin == GridPos::new(2, 19)
        }));
        assert!(fight.images().iter().any(|image| {
            image.resource == super::move_category_icon_resource(MoveCategory::Special)
                && image.bounds.origin == GridPos::new(4, 19)
        }));
    }

    #[test]
    fn pokemon_selection_uses_animated_team_icons() {
        let snapshot = battle_fixture();
        let mut ui = BattleUiState::default();
        ui.handle_key(&key(NamedKey::ArrowRight), snapshot.interaction());
        ui.handle_key(&key(NamedKey::Enter), snapshot.interaction());

        let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);

        assert!(
            view.labels()
                .iter()
                .any(|label| { label.role == TextRole::PageTitle && label.content == "宝可梦" })
        );
        assert_eq!(
            view.labels()
                .iter()
                .filter(|label| matches!(label.role, TextRole::TeamMember(_)))
                .count(),
            TEAM_SIZE
        );
        assert_eq!(view.images().len(), TEAM_SIZE * 2 + 1);
        for slot in 0..TEAM_SIZE {
            assert!(
                view.images()
                    .iter()
                    .any(|image| image.resource == super::pokemon_icon_resource(slot, 0))
            );
        }
        assert!(view.images().iter().any(|image| {
            image.resource == super::type_icon_resource(PokemonType::Poison)
                && image.bounds.origin == GridPos::new(12, 5)
        }));

        let animated = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1);
        assert!(
            animated
                .images()
                .iter()
                .any(|image| image.resource == super::pokemon_icon_resource(0, 1))
        );
        assert!(view.images().iter().any(|image| {
            image.resource == super::pokemon_icon_resource(0, 0)
                && image.bounds.origin == GridPos::new(2, 4)
        }));
    }

    #[test]
    fn type_icons_are_embedded_in_battle_type_order() {
        let atlas = super::atlas();
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
        for pokemon_type in types {
            assert!(
                atlas
                    .resource(super::type_icon_resource(pokemon_type))
                    .is_some()
            );
        }
        assert_eq!(
            super::type_icon_resource(PokemonType::Fire),
            ResourceId(super::TYPE_ICON_RESOURCE_START + 9)
        );
        for category in [
            MoveCategory::Physical,
            MoveCategory::Special,
            MoveCategory::Status,
        ] {
            assert!(
                atlas
                    .resource(super::move_category_icon_resource(category))
                    .is_some()
            );
        }
    }

    #[test]
    fn world_projection_and_keyboard_input_share_the_integer_grid() {
        let world = WorldApplication::demo().unwrap();
        let view = project_world(&world.observe());

        assert_eq!(
            world_command_for_key(&key(NamedKey::ArrowRight)),
            Some(WorldCommand::Move(Direction::Right))
        );
        assert_eq!(world.observe().player(), Position::new(3, 6));
        assert!(view.labels().is_empty());
        assert_eq!(view.surface().size(), GridSize::new(32, 24));
        assert_eq!(view.images().len(), 1);
        assert_eq!(
            view.images()[0].resource,
            world_character_resource(Direction::Down, WorldAnimation::Stand, 0)
        );
        assert_eq!(
            world_character_resource(Direction::Up, WorldAnimation::Run, 2),
            ResourceId(29)
        );
        assert_eq!(
            world_character_resource(Direction::Up, WorldAnimation::RunStopping, 99),
            ResourceId(27)
        );
        assert_eq!(
            (0..4)
                .map(|frame| world_character_resource(Direction::Left, WorldAnimation::Walk, frame))
                .collect::<Vec<_>>(),
            vec![
                ResourceId(13),
                ResourceId(12),
                ResourceId(14),
                ResourceId(12)
            ]
        );
        assert_eq!(
            (0..4)
                .map(|frame| {
                    world_character_resource(Direction::Right, WorldAnimation::Run, frame)
                })
                .collect::<Vec<_>>(),
            vec![
                ResourceId(22),
                ResourceId(21),
                ResourceId(23),
                ResourceId(21)
            ]
        );
    }

    #[test]
    fn command_console_overlays_grid_background_and_keeps_text_as_labels() {
        let world = WorldApplication::demo().unwrap();
        let mut view = project_world(&world.observe());
        overlay_command_console(
            &mut view,
            &CommandConsoleView {
                query: "move".into(),
                preedit: "中".into(),
                items: vec!["/battle/move/one use".into(), "/battle/move/two use".into()],
                selected_index: Some(1),
                diagnostic: Some("action rejected".into()),
            },
        );

        assert!(
            view.labels().iter().any(|label| {
                label.role == TextRole::ConsoleQuery && label.content == "> move中"
            })
        );
        assert!(view.labels().iter().any(|label| {
            label.role == TextRole::ConsoleItem(1) && label.content == "/battle/move/two use"
        }));
        assert!(
            view.labels()
                .iter()
                .any(|label| label.role == TextRole::ConsoleDiagnostic)
        );
        assert!(view.surface().get(GridPos::new(2, 9)).is_ok());
    }
}
