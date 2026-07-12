mod movement;
mod text;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use game_host::{DemoGame, GameScene};
use game_ui::{CANVAS_HEIGHT, CANVAS_WIDTH, WorldAnimation, atlas};
use movement::{Gait, PressedDirections, RUN_STOP_DURATION, WORLD_TICK_INTERVAL, WorldMotion};
use punctum_gpu::{GpuAtlas, GpuClip, PixelOffset, PixelSize, Rgba8, Viewport, plan_composite};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use punctum_wgpu::{GpuRuntime, PresentOutcome, WinitKeyEventSnapshot, normalize_key_event};
use text::BattleTextRenderer;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};
use world_application::{Direction, WorldEvent};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);
const BATTLE_FRAME_INTERVAL: Duration = Duration::from_millis(300);
const TURN_HOLD_DURATION: Duration = Duration::from_millis(90);

struct CreatureGameApp {
    game: DemoGame,
    atlas: GpuAtlas,
    text_renderer: BattleTextRenderer,
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
        Ok(Self {
            game: DemoGame::new()
                .map_err(|error| std::io::Error::other(format!("demo game: {error:?}")))?,
            atlas: atlas(),
            text_renderer: BattleTextRenderer::new(),
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
        let view =
            self.game
                .view_with_presentation(sprite_frame, world_animation, world_pixel_offset);
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
        let result = runtime.present_plan_with_overlay(
            &plan,
            |device, queue, target, encoder, format, size| {
                text_result = renderer.encode(
                    labels, viewport, device, queue, target, encoder, format, size,
                );
            },
        );
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
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
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
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
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

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = CreatureGameApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use punctum_gpu::{PixelOffset, PixelSize};

    use std::time::{Duration, Instant};

    use super::{battle_viewport, earliest_deadline, next_sprite_frame};

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
}
