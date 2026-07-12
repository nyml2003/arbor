mod ollama_planner;
mod palette_overlay;
mod ramus_palette;
mod view;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use ollama_planner::{
    CompletionDisposition, OllamaConfig, OllamaTransport, PlannerCompletion, PlannerError,
    PlannerJob, PlannerSession, PlannerView, PlannerWorker, RequestId,
};
use palette_overlay::{PaletteOverlayRenderer, PlannerNotice, plan_palette_overlay};
use punctum_gpu::{GpuAtlas, GpuCell, GpuClip, PixelSize, Rgba8, plan_patch, plan_surface};
use punctum_grid::Surface;
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, PhysicalKeyCode, TextEvent};
use punctum_tetris::{PieceKind, TetrisCommand, TetrisState, transition};
use punctum_wgpu::{
    GpuRuntime, PresentOutcome, WinitCommittedTextSnapshot, WinitKeyEventSnapshot,
    normalize_committed_text, normalize_key_event,
};
use ramus_palette::{CommandQueue, PaletteIntent, PaletteOutcome, PaletteState, RamusPalette};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

use view::{apply_key, atlas, project_frame, viewport};

const TICK_INTERVAL: Duration = Duration::from_millis(450);
const CLEAR_COLOR: Rgba8 = Rgba8::new(18, 20, 24, 255);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HostMode {
    Gameplay,
    Palette,
}

struct HostModel {
    mode: HostMode,
    state: TetrisState,
    palette_state: PaletteState,
    palette: RamusPalette,
    command_queue: CommandQueue,
    planner: PlannerSession,
    next_tick: Instant,
}

impl HostModel {
    fn new(now: Instant) -> Result<Self, Box<dyn Error>> {
        let command_queue = CommandQueue::default();
        Ok(Self {
            mode: HostMode::Gameplay,
            state: TetrisState::new(PieceKind::ALL.to_vec())?,
            palette_state: PaletteState::default(),
            palette: RamusPalette::new(Arc::clone(&command_queue)),
            command_queue,
            planner: PlannerSession::default(),
            next_tick: now + TICK_INTERVAL,
        })
    }

    fn handle_keyboard_input(
        &mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
        now: Instant,
    ) -> bool {
        if is_palette_toggle(key) {
            if key.phase == KeyPhase::Press {
                self.toggle_palette(now);
                return true;
            }
            return false;
        }

        match self.mode {
            HostMode::Gameplay => self.handle_gameplay_key(key),
            HostMode::Palette => {
                if let Some(intent) = palette_intent_for_key(key) {
                    return self.apply_palette_intent(intent, now);
                }
                if key.phase != KeyPhase::Release
                    && let Some(text) = text
                {
                    return self.apply_palette_intent(
                        PaletteIntent::InsertText(text.text().to_owned()),
                        now,
                    );
                }
                false
            }
        }
    }

    fn handle_tick(&mut self, now: Instant) -> bool {
        if self.mode == HostMode::Palette {
            return false;
        }

        let mut changed = false;
        while now >= self.next_tick {
            changed |= self.apply_command(TetrisCommand::Tick);
            self.next_tick += TICK_INTERVAL;
        }
        changed
    }

    fn next_deadline(&self) -> Option<Instant> {
        (self.mode == HostMode::Gameplay).then_some(self.next_tick)
    }

    fn begin_planner(&mut self) -> Result<PlannerJob, PlannerError> {
        self.planner.begin(
            self.palette_state.query(),
            self.palette.discover_invocations(),
        )
    }

    fn planner_submit_failed(&mut self, id: RequestId, error: PlannerError) {
        self.planner.submit_failed(id, error);
    }

    fn complete_planner(&mut self, completion: PlannerCompletion, now: Instant) -> bool {
        match self.planner.complete(completion) {
            CompletionDisposition::Candidate(invocation) => {
                match self.palette.execute_invocation(&invocation) {
                    Ok(()) => {
                        self.drain_commands();
                        let outcome = self
                            .palette
                            .handle(&mut self.palette_state, PaletteIntent::Close);
                        debug_assert_eq!(outcome, PaletteOutcome::Closed);
                        self.resume_gameplay(now);
                    }
                    Err(diagnostic) => {
                        self.planner
                            .record_failure(&diagnostic.code, &diagnostic.message);
                    }
                }
                true
            }
            CompletionDisposition::Failed => true,
            CompletionDisposition::Ignored => false,
        }
    }

    fn planner_notice(&self) -> Option<PlannerNotice<'_>> {
        match self.planner.view() {
            PlannerView::Idle => None,
            PlannerView::Pending => Some(PlannerNotice::Pending),
            PlannerView::Failed(message) => Some(PlannerNotice::Failed(message)),
        }
    }

    fn toggle_palette(&mut self, now: Instant) {
        match self.mode {
            HostMode::Gameplay => {
                self.planner.clear_failure();
                let outcome = self
                    .palette
                    .handle(&mut self.palette_state, PaletteIntent::Open);
                debug_assert_eq!(outcome, PaletteOutcome::Updated);
                self.mode = HostMode::Palette;
            }
            HostMode::Palette => {
                let outcome = self
                    .palette
                    .handle(&mut self.palette_state, PaletteIntent::Close);
                debug_assert_eq!(outcome, PaletteOutcome::Closed);
                self.resume_gameplay(now);
            }
        }
    }

    fn handle_gameplay_key(&mut self, key: &KeyEvent) -> bool {
        let next = apply_key(&self.state, key);
        let changed = next != self.state;
        self.state = next;
        changed
    }

    fn apply_palette_intent(&mut self, intent: PaletteIntent, now: Instant) -> bool {
        if matches!(
            intent,
            PaletteIntent::InsertText(_) | PaletteIntent::Backspace
        ) {
            self.planner.detach();
            self.planner.clear_failure();
        }
        match self.palette.handle(&mut self.palette_state, intent) {
            PaletteOutcome::Executed => {
                self.drain_commands();
                self.resume_gameplay(now);
                true
            }
            PaletteOutcome::Closed => {
                self.resume_gameplay(now);
                true
            }
            PaletteOutcome::Updated | PaletteOutcome::NoSelection | PaletteOutcome::Failed => true,
            PaletteOutcome::Ignored => false,
        }
    }

    fn drain_commands(&mut self) {
        let commands = self
            .command_queue
            .lock()
            .expect("a successful provider execution leaves the command queue available")
            .drain(..)
            .collect::<Vec<_>>();
        for command in commands {
            self.apply_command(command);
        }
    }

    fn apply_command(&mut self, command: TetrisCommand) -> bool {
        let next = transition(&self.state, command);
        let changed = next != self.state;
        self.state = next;
        changed
    }

    fn resume_gameplay(&mut self, now: Instant) {
        self.planner.detach();
        self.mode = HostMode::Gameplay;
        self.next_tick = now + TICK_INTERVAL;
    }
}

fn is_palette_toggle(key: &KeyEvent) -> bool {
    key.modifiers.control
        && (key.physical == Some(PhysicalKeyCode::KeyP)
            || matches!(&key.logical, LogicalKey::Character(character) if character.eq_ignore_ascii_case("p")))
}

fn is_planner_submit(key: &KeyEvent) -> bool {
    key.modifiers.control
        && (key.physical == Some(PhysicalKeyCode::Enter)
            || matches!(key.logical, LogicalKey::Named(NamedKey::Enter)))
}

fn palette_intent_for_key(key: &KeyEvent) -> Option<PaletteIntent> {
    if key.phase == KeyPhase::Release {
        return None;
    }

    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(PaletteIntent::Previous),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(PaletteIntent::Next),
        LogicalKey::Named(NamedKey::Backspace) => Some(PaletteIntent::Backspace),
        LogicalKey::Named(NamedKey::Enter)
            if key.phase == KeyPhase::Press && !key.modifiers.control =>
        {
            Some(PaletteIntent::Execute)
        }
        LogicalKey::Named(NamedKey::Escape) if key.phase == KeyPhase::Press => {
            Some(PaletteIntent::Close)
        }
        _ => None,
    }
}

struct TetrisGpu {
    atlas: GpuAtlas,
    host: HostModel,
    previous: Option<Surface<GpuCell>>,
    overlay_renderer: PaletteOverlayRenderer,
    planner_worker: PlannerWorker,
    modifiers: ModifiersState,
    window: Option<Arc<Window>>,
    runtime: Option<GpuRuntime<'static>>,
}

impl TetrisGpu {
    fn new(planner_worker: PlannerWorker) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            atlas: atlas(),
            host: HostModel::new(Instant::now())?,
            previous: None,
            overlay_renderer: PaletteOverlayRenderer::new(),
            planner_worker,
            modifiers: ModifiersState::empty(),
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let attributes = Window::default_attributes()
            .with_title("Punctum Tetris GPU")
            .with_inner_size(LogicalSize::new(480.0, 704.0));
        let window = Arc::new(event_loop.create_window(attributes)?);
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
        self.runtime = Some(runtime);
        self.window = Some(window);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let frame = project_frame(self.previous.as_ref(), &self.host.state);
        let surface_size = runtime.surface_size();
        let viewport = viewport(surface_size);
        let grid_plan = if let Some(patch) = frame.patch() {
            plan_patch(patch, &self.atlas, u32::MAX, viewport, GpuClip::Surface)
        } else {
            plan_surface(
                frame.surface(),
                &self.atlas,
                u32::MAX,
                viewport,
                GpuClip::Surface,
            )
        };
        let grid_plan = match grid_plan {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("GPU submission planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let overlay_plan = plan_palette_overlay(
            &self.host.palette_state,
            self.host.planner_notice(),
            surface_size,
        );
        let mut overlay_result = Ok(());
        let renderer = &mut self.overlay_renderer;
        let result = runtime.present_plan_with_overlay(
            &grid_plan,
            |device, queue, target, encoder, format, size| {
                overlay_result =
                    renderer.encode(&overlay_plan, device, queue, target, encoder, format, size);
            },
        );

        if let Err(error) = overlay_result {
            eprintln!("GPU palette rendering failed: {error}");
            event_loop.exit();
            return;
        }
        match result {
            Ok(outcome) => {
                self.previous = Some(frame.into_surface());
                match outcome {
                    PresentOutcome::Reconfigured => window.request_redraw(),
                    PresentOutcome::SurfaceLost => {
                        runtime.resize(runtime.surface_size());
                        window.request_redraw();
                    }
                    PresentOutcome::Presented
                    | PresentOutcome::PresentedAndReconfigured
                    | PresentOutcome::SkippedMinimized
                    | PresentOutcome::SkippedTimeout
                    | PresentOutcome::SkippedOccluded => {}
                }
            }
            Err(error) => {
                eprintln!("GPU presentation failed: {error}");
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

    fn handle_keyboard_event(&mut self, event: winit::event::KeyEvent) {
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
        if self.host.mode == HostMode::Palette && is_planner_submit(&key) {
            if key.phase == KeyPhase::Press {
                self.submit_planner();
            }
            return;
        }
        if self
            .host
            .handle_keyboard_input(&key, text.as_ref(), Instant::now())
        {
            self.request_redraw();
        }
    }

    fn handle_tick(&mut self) {
        if self.host.handle_tick(Instant::now()) {
            self.request_redraw();
        }
    }

    fn submit_planner(&mut self) {
        match self.host.begin_planner() {
            Ok(job) => {
                let id = job.id;
                if let Err(error) = self.planner_worker.try_submit(job) {
                    self.host.planner_submit_failed(id, error);
                }
            }
            Err(error) => self.host.planner.record_error(error),
        }
        self.request_redraw();
    }

    fn handle_planner_completion(&mut self, completion: PlannerCompletion) {
        if self.host.complete_planner(completion, Instant::now()) {
            self.request_redraw();
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

#[derive(Debug)]
enum AppEvent {
    PlannerCompleted(PlannerCompletion),
}

impl ApplicationHandler<AppEvent> for TetrisGpu {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("GPU initialization failed: {error}");
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
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard_event(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.handle_tick();
        match self.host.next_deadline() {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::PlannerCompleted(completion) => self.handle_planner_completion(completion),
        }
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let planner_worker = PlannerWorker::spawn(
        OllamaTransport::new(OllamaConfig::from_env()),
        move |completion| {
            let _ = proxy.send_event(AppEvent::PlannerCompleted(completion));
        },
    )?;
    let mut app = TetrisGpu::new(planner_worker)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        time::Instant,
    };

    use punctum_input::{
        KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEvent,
    };
    use punctum_tetris::{TetrisCommand, transition};

    use super::{HostMode, HostModel, PlannerCompletion, PlannerError, PlannerView, TICK_INTERVAL};

    fn model(now: Instant) -> HostModel {
        HostModel::new(now).unwrap()
    }

    fn key(physical: PhysicalKeyCode, logical: LogicalKey, modifiers: Modifiers) -> KeyEvent {
        KeyEvent {
            physical: Some(physical),
            logical,
            modifiers,
            phase: KeyPhase::Press,
        }
    }

    fn named(physical: PhysicalKeyCode, logical: NamedKey) -> KeyEvent {
        key(physical, LogicalKey::Named(logical), Modifiers::default())
    }

    fn control_p() -> KeyEvent {
        key(
            PhysicalKeyCode::KeyP,
            LogicalKey::Character("p".into()),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        )
    }

    fn control_enter() -> KeyEvent {
        key(
            PhysicalKeyCode::Enter,
            LogicalKey::Named(NamedKey::Enter),
            Modifiers {
                control: true,
                ..Modifiers::default()
            },
        )
    }

    fn committed(text: &str) -> TextEvent {
        TextEvent::new(text).unwrap()
    }

    #[test]
    fn ctrl_p_toggles_mode_without_inserting_p() {
        let now = Instant::now();
        let mut host = model(now);
        let p = committed("p");

        assert!(host.handle_keyboard_input(&control_p(), Some(&p), now));
        assert_eq!(host.mode, HostMode::Palette);
        assert!(host.palette_state.is_open());
        assert_eq!(host.palette_state.query(), "");

        let mut repeated = control_p();
        repeated.phase = KeyPhase::Repeat;
        assert!(!host.handle_keyboard_input(&repeated, Some(&p), now));
        assert_eq!(host.mode, HostMode::Palette);
        assert_eq!(host.palette_state.query(), "");

        let closed_at = now + TICK_INTERVAL;
        assert!(host.handle_keyboard_input(&control_p(), Some(&p), closed_at));
        assert_eq!(host.mode, HostMode::Gameplay);
        assert!(!host.palette_state.is_open());
        assert_eq!(host.next_tick, closed_at + TICK_INTERVAL);
    }

    #[test]
    fn palette_captures_gameplay_keys_and_accepts_committed_unicode() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let before = host.state.clone();

        assert!(!host.handle_keyboard_input(
            &named(PhysicalKeyCode::ArrowRight, NamedKey::ArrowRight),
            None,
            now,
        ));
        assert_eq!(host.state, before);

        assert!(host.handle_keyboard_input(
            &named(PhysicalKeyCode::ArrowDown, NamedKey::ArrowDown),
            None,
            now,
        ));
        assert_eq!(host.palette_state.selected_index(), Some(1));

        let cjk = committed("方块");
        assert!(host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::Unidentified,
                LogicalKey::Unidentified,
                Modifiers::default(),
            ),
            Some(&cjk),
            now,
        ));
        assert_eq!(host.palette_state.query(), "方块");
        assert_eq!(host.state, before);
    }

    #[test]
    fn enter_executes_ramus_command_drains_queue_and_closes_palette() {
        let now = Instant::now();
        let mut host = model(now);
        let expected = transition(&host.state, TetrisCommand::MoveLeft);
        host.handle_keyboard_input(&control_p(), None, now);
        let query = committed("left");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::KeyL,
                LogicalKey::Character("l".into()),
                Modifiers::default(),
            ),
            Some(&query),
            now,
        );

        assert!(host.handle_keyboard_input(
            &named(PhysicalKeyCode::Enter, NamedKey::Enter),
            None,
            now,
        ));
        assert_eq!(host.mode, HostMode::Gameplay);
        assert_eq!(host.state, expected);
        assert!(host.command_queue.lock().unwrap().is_empty());
        assert_eq!(host.next_tick, now + TICK_INTERVAL);
    }

    #[test]
    fn execution_failure_keeps_palette_open_with_diagnostic() {
        let now = Instant::now();
        let mut host = model(now);
        let poisoned = host.command_queue.clone();
        let _ = catch_unwind(AssertUnwindSafe(move || {
            let _guard = poisoned.lock().unwrap();
            panic!("poison command queue for host failure fixture");
        }));
        host.handle_keyboard_input(&control_p(), None, now);
        let query = committed("left");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::KeyL,
                LogicalKey::Character("l".into()),
                Modifiers::default(),
            ),
            Some(&query),
            now,
        );

        assert!(host.handle_keyboard_input(
            &named(PhysicalKeyCode::Enter, NamedKey::Enter),
            None,
            now,
        ));
        assert_eq!(host.mode, HostMode::Palette);
        assert!(host.palette_state.is_open());
        assert_eq!(
            host.palette_state.diagnostic().unwrap().code,
            "command-queue-unavailable"
        );
    }

    #[test]
    fn palette_pauses_ticks_and_closing_discards_the_paused_deadline() {
        let started_at = Instant::now();
        let mut host = model(started_at);
        host.handle_keyboard_input(&control_p(), None, started_at);
        let paused = host.state.clone();
        let closed_at = started_at + TICK_INTERVAL * 10;

        assert!(!host.handle_tick(closed_at));
        assert_eq!(host.state, paused);
        assert_eq!(host.next_deadline(), None);

        host.handle_keyboard_input(&control_p(), None, closed_at);
        assert_eq!(host.next_tick, closed_at + TICK_INTERVAL);
        assert!(!host.handle_tick(closed_at));
        assert!(host.handle_tick(closed_at + TICK_INTERVAL));
    }

    #[test]
    fn gameplay_keys_and_ramus_invocations_share_the_same_transition_result() {
        let cases = [
            (
                named(PhysicalKeyCode::ArrowLeft, NamedKey::ArrowLeft),
                "left",
            ),
            (
                named(PhysicalKeyCode::ArrowRight, NamedKey::ArrowRight),
                "right",
            ),
            (named(PhysicalKeyCode::ArrowUp, NamedKey::ArrowUp), "rotate"),
            (
                named(PhysicalKeyCode::ArrowDown, NamedKey::ArrowDown),
                "soft-drop",
            ),
            (named(PhysicalKeyCode::Space, NamedKey::Space), "hard-drop"),
            (
                key(
                    PhysicalKeyCode::KeyR,
                    LogicalKey::Character("r".into()),
                    Modifiers::default(),
                ),
                "restart",
            ),
        ];

        for (gameplay_key, query) in cases {
            let now = Instant::now();
            let mut direct = model(now);
            let mut through_ramus = model(now);
            direct.handle_keyboard_input(&gameplay_key, None, now);

            through_ramus.handle_keyboard_input(&control_p(), None, now);
            let text = committed(query);
            through_ramus.handle_keyboard_input(
                &key(
                    PhysicalKeyCode::Unidentified,
                    LogicalKey::Unidentified,
                    Modifiers::default(),
                ),
                Some(&text),
                now,
            );
            through_ramus.handle_keyboard_input(
                &named(PhysicalKeyCode::Enter, NamedKey::Enter),
                None,
                now,
            );

            assert_eq!(through_ramus.state, direct.state, "query: {query}");
            assert_eq!(through_ramus.mode, HostMode::Gameplay, "query: {query}");
        }
    }

    #[test]
    fn planner_candidate_reenters_ramus_and_applies_exactly_once() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let prompt = committed("turn the piece clockwise");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::Unidentified,
                LogicalKey::Unidentified,
                Modifiers::default(),
            ),
            Some(&prompt),
            now,
        );
        let request = host.begin_planner().unwrap();
        let expected = transition(&host.state, TetrisCommand::RotateClockwise);
        let completion = PlannerCompletion {
            id: request.id,
            result: Ok("/tetris/piece rotate".into()),
        };

        assert!(host.complete_planner(completion.clone(), now));
        assert_eq!(host.state, expected);
        assert_eq!(host.mode, HostMode::Gameplay);
        assert!(host.command_queue.lock().unwrap().is_empty());

        assert!(!host.complete_planner(completion, now));
        assert_eq!(host.state, expected);
    }

    #[test]
    fn unauthorized_planner_output_is_rejected_by_ramus() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let prompt = committed("show developer internals");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::Unidentified,
                LogicalKey::Unidentified,
                Modifiers::default(),
            ),
            Some(&prompt),
            now,
        );
        let request = host.begin_planner().unwrap();
        let before = host.state.clone();

        assert!(host.complete_planner(
            PlannerCompletion {
                id: request.id,
                result: Ok("/developer/tetris inspect".into()),
            },
            now,
        ));
        assert_eq!(host.state, before);
        assert_eq!(host.mode, HostMode::Palette);
        assert!(matches!(
            host.planner.view(),
            PlannerView::Failed(message) if message.contains("operation-unavailable")
        ));
    }

    #[test]
    fn closing_palette_detaches_pending_planner_result() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let prompt = committed("drop it");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::Unidentified,
                LogicalKey::Unidentified,
                Modifiers::default(),
            ),
            Some(&prompt),
            now,
        );
        let request = host.begin_planner().unwrap();
        let before = host.state.clone();
        host.handle_keyboard_input(&control_p(), None, now);

        assert!(!host.complete_planner(
            PlannerCompletion {
                id: request.id,
                result: Ok("/tetris/piece hard-drop".into()),
            },
            now,
        ));
        assert_eq!(host.state, before);
        assert_eq!(host.mode, HostMode::Gameplay);
    }

    #[test]
    fn planner_transport_failure_stays_outside_ramus_diagnostics() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let prompt = committed("rotate please");
        host.handle_keyboard_input(
            &key(
                PhysicalKeyCode::Unidentified,
                LogicalKey::Unidentified,
                Modifiers::default(),
            ),
            Some(&prompt),
            now,
        );
        let request = host.begin_planner().unwrap();
        let before = host.state.clone();

        assert!(host.complete_planner(
            PlannerCompletion {
                id: request.id,
                result: Err(PlannerError::Timeout),
            },
            now,
        ));
        assert_eq!(host.state, before);
        assert!(host.palette_state.diagnostic().is_none());
        assert!(matches!(host.planner.view(), PlannerView::Failed(_)));
    }

    #[test]
    fn ctrl_enter_is_not_treated_as_normal_palette_execute() {
        let now = Instant::now();
        let mut host = model(now);
        host.handle_keyboard_input(&control_p(), None, now);
        let before = host.state.clone();

        assert!(!host.handle_keyboard_input(&control_enter(), None, now));
        assert_eq!(host.mode, HostMode::Palette);
        assert_eq!(host.state, before);
        assert!(host.command_queue.lock().unwrap().is_empty());
    }
}
