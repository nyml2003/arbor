mod console;
mod map;
mod movement;
mod sprites;
mod text;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use console::{ConsoleIntent, ConsoleOutcome, ConsoleState, GameConsole};
use game_host::{DemoGame, GameScene};
use game_ui::{
    CANVAS_HEIGHT, CANVAS_WIDTH, CommandConsoleView, WorldAnimation, overlay_command_console,
};
use map::load_map;
use map_project::MapProject;
use map_render::{AtomicTileCatalog, MapCamera, MapGridLayout, MapRenderInput, project_map};
use movement::{Gait, PressedDirections, RUN_STOP_DURATION, WORLD_TICK_INTERVAL, WorldMotion};
use punctum_gpu::{GpuAtlas, GpuClip, PixelOffset, PixelSize, Rgba8, Viewport, plan_composite};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, PhysicalKeyCode, TextEvent};
use punctum_wgpu::{
    GpuRuntime, PresentOutcome, WinitCommittedTextSnapshot, WinitKeyEventSnapshot,
    normalize_committed_text, normalize_key_event,
};
use sprites::load_battle_atlas;
use text::BattleTextRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{Ime, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};
use world_application::{Direction, WorldEvent};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);
const BATTLE_FRAME_INTERVAL: Duration = Duration::from_millis(300);
const TURN_HOLD_DURATION: Duration = Duration::from_millis(90);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostMode {
    Gameplay,
    Console,
}

struct CreatureGameApp {
    mode: HostMode,
    game: DemoGame,
    map_project: MapProject,
    map_catalog: AtomicTileCatalog,
    console: GameConsole,
    console_state: ConsoleState,
    atlas: GpuAtlas,
    text_renderer: BattleTextRenderer,
    ime_preedit: String,
    ime_composing: bool,
    modifiers: ModifiersState,
    next_playback: Option<Instant>,
    sprite_frame: usize,
    next_sprite_frame: Option<Instant>,
    pressed_directions: PressedDirections,
    world_motion: Option<WorldMotion>,
    next_world_tick: Option<Instant>,
    turn_hold_ends: Option<Instant>,
    run_stop_ends: Option<Instant>,
    window: Option<Arc<Window>>,
    runtime: Option<GpuRuntime<'static>>,
}

impl CreatureGameApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let loaded_map = load_map()?;
        let world = world_application::WorldApplication::from_map_project(&loaded_map.project)
            .map_err(|error| std::io::Error::other(format!("map world: {error:?}")))?;
        let game = DemoGame::new_random_with_world(world)
            .map_err(|error| std::io::Error::other(format!("demo game: {error:?}")))?;
        let sprite_manifest = game
            .sprite_manifest()
            .map_err(|error| std::io::Error::other(format!("demo sprite manifest: {error:?}")))?;
        let atlas = load_battle_atlas(&sprite_manifest, &loaded_map.images)?;
        Ok(Self {
            mode: HostMode::Gameplay,
            game,
            map_project: loaded_map.project,
            map_catalog: loaded_map.catalog,
            console: GameConsole::new(),
            console_state: ConsoleState::default(),
            atlas,
            text_renderer: BattleTextRenderer::new(),
            ime_preedit: String::new(),
            ime_composing: false,
            modifiers: ModifiersState::empty(),
            next_playback: None,
            sprite_frame: 0,
            next_sprite_frame: None,
            pressed_directions: PressedDirections::default(),
            world_motion: None,
            next_world_tick: None,
            turn_hold_ends: None,
            run_stop_ends: None,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("宝可梦：还没想好名字")
                    .with_inner_size(LogicalSize::new(960.0, 720.0)),
            )?,
        );
        let size = pixel_size(window.inner_size());
        let instance = wgpu::Instance::default();
        let runtime = pollster::block_on(GpuRuntime::new(
            &instance,
            window.clone(),
            size,
            &self.atlas,
            CLEAR_COLOR,
        ))?;
        window.set_ime_allowed(false);
        window.request_redraw();
        self.window = Some(window);
        self.runtime = Some(runtime);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(surface_size) = self.runtime.as_ref().map(GpuRuntime::surface_size) else {
            return;
        };
        let viewport = battle_viewport(surface_size);
        let (world_animation, sprite_frame, world_pixel_offset) =
            self.presentation(Instant::now(), viewport.cell_size);
        let world_scene = self.game.scene() == GameScene::World;
        let player_pixel_offset = if world_scene {
            PixelOffset::new(0, 0)
        } else {
            world_pixel_offset
        };
        let mut view =
            self.game
                .view_with_presentation(sprite_frame, world_animation, player_pixel_offset);
        if world_scene {
            let camera = world_camera(self.game.world_position());
            let scene = match project_map(MapRenderInput {
                project: &self.map_project,
                catalog: &self.map_catalog,
                camera,
                pixel_offset: invert_pixel_offset(world_pixel_offset),
                viewport,
                layout: MapGridLayout::new(
                    punctum_grid::GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT),
                    punctum_grid::GridSize::new(2, 2),
                ),
            }) {
                Ok(scene) => scene,
                Err(error) => {
                    eprintln!("map projection failed: {error}");
                    event_loop.exit();
                    return;
                }
            };
            view.replace_world_background(&scene, camera);
        }
        if self.mode == HostMode::Console {
            overlay_command_console(&mut view, &self.console_view());
        }
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let plan = match plan_composite(
            view.surface(),
            view.images(),
            &self.atlas,
            u32::MAX,
            viewport,
            GpuClip::Surface,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("game GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let labels = view.labels();
        let renderer = &mut self.text_renderer;
        let mut text_result = Ok(());
        let result = if labels.is_empty() {
            runtime.present_plan(&plan)
        } else {
            runtime.present_plan_with_overlay(
                &plan,
                |device, queue, target, encoder, format, size| {
                    text_result = renderer.encode(
                        labels, viewport, device, queue, target, encoder, format, size,
                    );
                },
            )
        };
        if let Err(error) = text_result {
            eprintln!("game text rendering failed: {error}");
            event_loop.exit();
            return;
        }
        match result {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(
                PresentOutcome::Presented
                | PresentOutcome::PresentedAndReconfigured
                | PresentOutcome::SkippedMinimized
                | PresentOutcome::SkippedTimeout
                | PresentOutcome::SkippedOccluded,
            ) => {}
            Err(error) => {
                eprintln!("game presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let text = match normalize_committed_text(WinitCommittedTextSnapshot::new(
            event.text.map(|text| text.to_string()),
        )) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("ignored invalid committed text: {error}");
                None
            }
        };
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        if self.mode == HostMode::Console && self.ime_composing && !is_console_toggle(&key) {
            return;
        }
        if is_console_toggle(&key) {
            if key.phase == KeyPhase::Press {
                self.toggle_console();
            }
            return;
        }
        if self.mode == HostMode::Console {
            self.handle_console_input(&key, text.as_ref());
            return;
        }
        if self.game.scene() == GameScene::World
            && let Some(direction) = direction_for_key(&key)
        {
            self.handle_world_direction(direction, key.phase, Instant::now());
            return;
        }
        match self.game.handle_key(&key) {
            Ok(true) => {
                let now = Instant::now();
                self.sync_battle_sprite_timer(now);
                if self.game.has_pending_playback() {
                    self.next_playback = Some(now + Duration::from_millis(600));
                }
                self.request_redraw();
            }
            Ok(false) => {}
            Err(error) => eprintln!("game command rejected: {error:?}"),
        }
    }

    fn toggle_console(&mut self) {
        match self.mode {
            HostMode::Gameplay => {
                let legal_actions = self.game.legal_player_actions();
                let outcome = self
                    .console
                    .handle(&mut self.console_state, ConsoleIntent::Open(legal_actions));
                debug_assert_eq!(outcome, ConsoleOutcome::Updated);
                self.mode = HostMode::Console;
            }
            HostMode::Console => {
                let outcome = self
                    .console
                    .handle(&mut self.console_state, ConsoleIntent::Close);
                debug_assert_eq!(outcome, ConsoleOutcome::Closed);
                self.mode = HostMode::Gameplay;
            }
        }
        self.sync_ime_allowed();
        self.request_redraw();
    }

    fn handle_console_input(&mut self, key: &KeyEvent, text: Option<&TextEvent>) {
        let intent = console_intent_for_key(key).or_else(|| {
            (key.phase != KeyPhase::Release)
                .then(|| text.map(|text| ConsoleIntent::InsertText(text.text().to_owned())))
                .flatten()
        });
        let Some(intent) = intent else {
            return;
        };
        match self.console.handle(&mut self.console_state, intent) {
            ConsoleOutcome::ActionQueued => self.commit_console_action(),
            ConsoleOutcome::Closed => {
                self.mode = HostMode::Gameplay;
                self.sync_ime_allowed();
                self.request_redraw();
            }
            ConsoleOutcome::Updated | ConsoleOutcome::NoSelection | ConsoleOutcome::Failed => {
                self.request_redraw();
            }
            ConsoleOutcome::Ignored => {}
        }
    }

    fn commit_console_action(&mut self) {
        let Some(action) = self.console.take_queued_action() else {
            self.console
                .execution_failed(&mut self.console_state, "Ramus 没有生成战斗 action");
            self.request_redraw();
            return;
        };
        match self.game.submit_player_action(action) {
            Ok(()) => {
                self.console.execution_succeeded(&mut self.console_state);
                self.mode = HostMode::Gameplay;
                let now = Instant::now();
                self.sync_battle_sprite_timer(now);
                self.next_playback = self
                    .game
                    .has_pending_playback()
                    .then_some(now + Duration::from_millis(600));
                self.sync_ime_allowed();
            }
            Err(error) => self.console.execution_failed(
                &mut self.console_state,
                format!("战斗 action 被拒绝: {error:?}"),
            ),
        }
        self.request_redraw();
    }

    fn handle_ime_event(&mut self, event: Ime) {
        match event {
            Ime::Enabled => {}
            Ime::Preedit(text, _) => {
                self.ime_composing = !text.is_empty();
                self.ime_preedit = text;
                self.request_redraw();
            }
            Ime::Commit(text) => {
                self.ime_composing = false;
                self.ime_preedit.clear();
                if text.is_empty() || self.mode != HostMode::Console {
                    self.request_redraw();
                    return;
                }
                let text = TextEvent::new(text).expect("non-empty IME commit is valid");
                if self.console.handle(
                    &mut self.console_state,
                    ConsoleIntent::InsertText(text.text().to_owned()),
                ) == ConsoleOutcome::Updated
                {
                    self.request_redraw();
                }
            }
            Ime::Disabled => {
                self.ime_composing = false;
                self.ime_preedit.clear();
                self.request_redraw();
            }
        }
    }

    fn sync_ime_allowed(&mut self) {
        let allowed = self.mode == HostMode::Console;
        if !allowed {
            self.ime_composing = false;
            self.ime_preedit.clear();
        }
        if let Some(window) = &self.window {
            window.set_ime_allowed(allowed);
        }
    }

    fn console_view(&self) -> CommandConsoleView {
        CommandConsoleView {
            query: self.console_state.query().to_owned(),
            preedit: self.ime_preedit.clone(),
            items: self
                .console_state
                .items()
                .iter()
                .map(|item| item.invocation.clone())
                .collect(),
            selected_index: self.console_state.selected_index(),
            diagnostic: self.console_state.diagnostic().map(str::to_owned),
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn handle_world_direction(&mut self, direction: Direction, phase: KeyPhase, now: Instant) {
        match phase {
            KeyPhase::Press => {
                self.pressed_directions.press(direction);
                self.run_stop_ends = None;
                self.settle_if_direction_changed(now);
                self.try_start_world_step(now);
            }
            KeyPhase::Repeat => {}
            KeyPhase::Release => {
                self.pressed_directions.release(direction);
                if self.pressed_directions.active().is_none() {
                    self.turn_hold_ends = None;
                }
                self.settle_if_direction_changed(now);
                if self.world_motion.is_none() {
                    self.try_start_world_step(now);
                    self.request_redraw();
                }
            }
        }
    }

    fn settle_if_direction_changed(&mut self, now: Instant) {
        if let Some(motion) = &mut self.world_motion
            && self.pressed_directions.active() != Some(motion.direction())
        {
            motion.settle(now);
        }
    }

    fn try_start_world_step(&mut self, now: Instant) {
        if self.game.scene() != GameScene::World || self.world_motion.is_some() {
            return;
        }
        let Some(direction) = self.pressed_directions.active() else {
            return;
        };
        let gait = if self.modifiers.shift_key() {
            Gait::Run
        } else {
            Gait::Walk
        };
        match self.game.step_world(direction) {
            Ok(WorldEvent::Turned { .. }) => {
                self.turn_hold_ends = Some(now + TURN_HOLD_DURATION);
                self.request_redraw();
            }
            Ok(WorldEvent::Moved { .. }) => {
                self.turn_hold_ends = None;
                self.world_motion = Some(WorldMotion::new(direction, gait, now));
                self.next_world_tick = Some(now);
                self.run_stop_ends = None;
                self.request_redraw();
            }
            Ok(WorldEvent::Blocked { .. }) => self.request_redraw(),
            Ok(WorldEvent::EncounterTriggered { .. }) => {
                self.clear_world_input();
                self.sync_battle_sprite_timer(now);
                self.request_redraw();
            }
            Err(error) => eprintln!("world movement rejected: {error:?}"),
        }
    }

    fn clear_world_input(&mut self) {
        self.pressed_directions.clear();
        self.world_motion = None;
        self.next_world_tick = None;
        self.turn_hold_ends = None;
        self.run_stop_ends = None;
    }

    fn sync_battle_sprite_timer(&mut self, now: Instant) {
        if self.game.scene() == GameScene::Battle {
            self.next_sprite_frame
                .get_or_insert(now + BATTLE_FRAME_INTERVAL);
        } else {
            self.sprite_frame = 0;
            self.next_sprite_frame = None;
        }
    }

    fn presentation(
        &self,
        now: Instant,
        cell_size: PixelSize,
    ) -> (WorldAnimation, usize, PixelOffset) {
        if self.game.scene() == GameScene::Battle {
            return (
                WorldAnimation::Stand,
                self.sprite_frame,
                PixelOffset::new(0, 0),
            );
        }
        if let Some(motion) = self.world_motion {
            return (
                motion.gait().animation(),
                motion.sprite_frame(now),
                motion.pixel_offset(now, cell_size),
            );
        }
        let animation = if self.run_stop_ends.is_some() {
            WorldAnimation::RunStopping
        } else {
            WorldAnimation::Stand
        };
        (animation, 0, PixelOffset::new(0, 0))
    }
}

impl ApplicationHandler for CreatureGameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("game initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Focused(false) => {
                self.pressed_directions.clear();
                self.turn_hold_ends = None;
                if let Some(motion) = &mut self.world_motion {
                    motion.settle(Instant::now());
                } else {
                    self.run_stop_ends = None;
                    self.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::Ime(event) => self.handle_ime_event(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.mode == HostMode::Console {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
            return;
        }
        let now = Instant::now();
        if self.next_playback.is_some_and(|deadline| now >= deadline) {
            self.game.advance_playback();
            self.request_redraw();
            self.next_playback = self
                .game
                .has_pending_playback()
                .then_some(now + Duration::from_millis(600));
        }
        if self
            .next_sprite_frame
            .is_some_and(|deadline| now >= deadline)
        {
            self.sprite_frame = next_sprite_frame(self.sprite_frame);
            self.next_sprite_frame = Some(now + BATTLE_FRAME_INTERVAL);
            self.request_redraw();
        }
        if self.turn_hold_ends.is_some_and(|deadline| now >= deadline) {
            self.turn_hold_ends = None;
            self.try_start_world_step(now);
        }
        if self
            .world_motion
            .is_some_and(|motion| motion.is_complete(now))
        {
            let gait = self.world_motion.take().map(WorldMotion::gait);
            self.next_world_tick = None;
            if self.pressed_directions.active().is_some() {
                self.try_start_world_step(now);
            } else {
                self.run_stop_ends = (gait == Some(Gait::Run)).then_some(now + RUN_STOP_DURATION);
                self.request_redraw();
            }
        } else if self.next_world_tick.is_some_and(|deadline| now >= deadline) {
            self.next_world_tick = Some(now + WORLD_TICK_INTERVAL);
            self.request_redraw();
        }
        if self.run_stop_ends.is_some_and(|deadline| now >= deadline) {
            self.run_stop_ends = None;
            self.request_redraw();
        }
        if let Some(deadline) = earliest_deadline(&[
            self.next_playback,
            self.next_sprite_frame,
            self.next_world_tick,
            self.turn_hold_ends,
            self.run_stop_ends,
        ]) {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}

fn direction_for_key(key: &KeyEvent) -> Option<Direction> {
    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(Direction::Up),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(Direction::Down),
        LogicalKey::Named(NamedKey::ArrowLeft) => Some(Direction::Left),
        LogicalKey::Named(NamedKey::ArrowRight) => Some(Direction::Right),
        _ => None,
    }
}

fn is_console_toggle(key: &KeyEvent) -> bool {
    key.modifiers.control
        && (key.physical == Some(PhysicalKeyCode::KeyP)
            || matches!(&key.logical, LogicalKey::Character(character) if character.eq_ignore_ascii_case("p")))
}

fn console_intent_for_key(key: &KeyEvent) -> Option<ConsoleIntent> {
    if key.phase == KeyPhase::Release {
        return None;
    }
    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(ConsoleIntent::Previous),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(ConsoleIntent::Next),
        LogicalKey::Named(NamedKey::Backspace) => Some(ConsoleIntent::Backspace),
        LogicalKey::Named(NamedKey::Enter) if key.phase == KeyPhase::Press => {
            Some(ConsoleIntent::Execute)
        }
        LogicalKey::Named(NamedKey::Escape) if key.phase == KeyPhase::Press => {
            Some(ConsoleIntent::Close)
        }
        _ => None,
    }
}

const fn next_sprite_frame(current: usize) -> usize {
    current.wrapping_add(1)
}

fn earliest_deadline(deadlines: &[Option<Instant>]) -> Option<Instant> {
    deadlines.iter().flatten().min().copied()
}

fn battle_viewport(target_size: PixelSize) -> Viewport {
    let cell_size = (target_size.width / CANVAS_WIDTH)
        .min(target_size.height / CANVAS_HEIGHT)
        .max(1);
    let width = i64::from(CANVAS_WIDTH) * i64::from(cell_size);
    let height = i64::from(CANVAS_HEIGHT) * i64::from(cell_size);
    Viewport::new(
        target_size,
        PixelOffset::new(
            ((i64::from(target_size.width) - width) / 2) as i32,
            ((i64::from(target_size.height) - height) / 2) as i32,
        ),
        PixelSize::new(cell_size, cell_size),
    )
    .expect("the battle viewport always has a positive integer cell size")
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn world_camera(player: world_application::Position) -> MapCamera {
    const VIEW_COLS: u16 = 16;
    const VIEW_ROWS: u16 = 12;
    MapCamera::new(
        i32::from(player.x()) - i32::from(VIEW_COLS / 2),
        i32::from(player.y()) - i32::from(VIEW_ROWS / 2),
    )
}

const fn invert_pixel_offset(offset: PixelOffset) -> PixelOffset {
    PixelOffset::new(-offset.x, -offset.y)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = CreatureGameApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use punctum_gpu::{PixelOffset, PixelSize};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};

    use std::time::{Duration, Instant};

    use super::{
        ConsoleIntent, CreatureGameApp, MapCamera, battle_viewport, console_intent_for_key,
        earliest_deadline, invert_pixel_offset, is_console_toggle, next_sprite_frame, world_camera,
    };
    use world_application::Position;

    fn key(logical: LogicalKey, physical: PhysicalKeyCode, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            physical: Some(physical),
            logical,
            modifiers,
            phase: KeyPhase::Press,
        }
    }

    #[test]
    fn battle_viewport_uses_integer_scaling_and_centers_the_canvas() {
        let viewport = battle_viewport(PixelSize::new(960, 720));
        assert_eq!(viewport.cell_size, PixelSize::new(30, 30));
        assert_eq!(viewport.origin, PixelOffset::new(0, 0));

        let wide = battle_viewport(PixelSize::new(1000, 720));
        assert_eq!(wide.cell_size, PixelSize::new(30, 30));
        assert_eq!(wide.origin, PixelOffset::new(20, 0));
    }

    #[test]
    fn world_camera_keeps_the_player_centered_even_near_map_edges() {
        assert_eq!(world_camera(Position::new(3, 6)), MapCamera::new(-5, 0));
        assert_eq!(world_camera(Position::new(20, 14)), MapCamera::new(12, 8));
        assert_eq!(
            invert_pixel_offset(PixelOffset::new(-60, 30)),
            PixelOffset::new(60, -30)
        );
    }

    #[test]
    fn complete_game_atlas_fits_wgpu_texture_limits() {
        let app = CreatureGameApp::new().unwrap();
        let size = app.atlas.size();
        assert!(size.width <= 8_192, "atlas width was {}", size.width);
        assert!(size.height <= 8_192, "atlas height was {}", size.height);
    }

    #[test]
    fn sprite_frames_wrap_and_deadlines_share_one_event_loop_wait() {
        assert_eq!(next_sprite_frame(0), 1);
        assert_eq!(next_sprite_frame(1), 2);

        let now = Instant::now();
        let early = now + Duration::from_millis(100);
        let late = now + Duration::from_millis(300);
        assert_eq!(
            earliest_deadline(&[Some(late), Some(early), None]),
            Some(early)
        );
        assert_eq!(earliest_deadline(&[None, Some(late), None]), Some(late));
    }

    #[test]
    fn console_toggle_and_navigation_use_canonical_input_events() {
        let toggle = key(
            LogicalKey::Character("p".into()),
            PhysicalKeyCode::KeyP,
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        );
        assert!(is_console_toggle(&toggle));

        let down = key(
            LogicalKey::Named(NamedKey::ArrowDown),
            PhysicalKeyCode::ArrowDown,
            Modifiers::default(),
        );
        assert_eq!(console_intent_for_key(&down), Some(ConsoleIntent::Next));
    }
}
