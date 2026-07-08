use std::time::{Duration, Instant};
use thorn_core::{
    render_pipeline, AppContext, BackendCapabilities, IntentContext, IntentMapper, IntentResolver,
    KeyAction, KeyEvent, KeyIntent, KeyMap, KeyMapLayer, KeyMapLayerKind, LayeredKeyMap,
    RuntimeInput, Screen, ScreenPatch, Size, ThornApp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStats {
    pub frame_index: u64,
    pub total_frame_time: Duration,
    pub update_time: Duration,
    pub component_time: Duration,
    pub lowering_time: Duration,
    pub layout_time: Duration,
    pub paint_time: Duration,
    pub backend_lowering_time: Duration,
    pub diff_time: Duration,
    pub present_time: Duration,
    pub host_node_count: usize,
    pub layout_node_count: usize,
    pub paint_primitive_count: usize,
    pub dirty_node_count: usize,
    pub dirty_regions: usize,
    pub backend_output_cells: usize,
    pub backend_output_size: usize,
    pub dirty_cells: usize,
}

pub trait PerfSink {
    fn record(&mut self, stats: &FrameStats);
}

#[derive(Default)]
pub struct NoopPerfSink;

impl PerfSink for NoopPerfSink {
    fn record(&mut self, _stats: &FrameStats) {}
}

pub struct AppRuntime<App>
where
    App: ThornApp,
{
    app: App,
    ctx: AppContext<App::Action>,
    keymap: LayeredKeyMap,
    resolver: Box<dyn IntentResolver<App::Action>>,
    intent_context: IntentContext,
    size: Size,
    screen: Screen,
    running: bool,
    render_requested: bool,
    frame_index: u64,
    last_frame_stats: Option<FrameStats>,
    perf_sink: Box<dyn PerfSink>,
}

impl<App> AppRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        Self::with_resolver(app, mapper)
    }

    pub fn with_resolver(app: App, resolver: impl IntentResolver<App::Action> + 'static) -> Self {
        let size = Size::new(80, 24);
        Self {
            app,
            ctx: AppContext::new(),
            keymap: LayeredKeyMap::default(),
            resolver: Box::new(resolver),
            intent_context: IntentContext::default(),
            size,
            screen: Screen::new(size),
            running: true,
            render_requested: true,
            frame_index: 0,
            last_frame_stats: None,
            perf_sink: Box::<NoopPerfSink>::default(),
        }
    }

    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.keymap = LayeredKeyMap::app_only(keymap);
        self
    }

    pub fn backend_capabilities(mut self, capabilities: BackendCapabilities) -> Self {
        self.intent_context.backend_capabilities = capabilities.clone();
        self.ctx.set_backend_capabilities(capabilities);
        self
    }

    pub fn layered_keymap(mut self, keymap: LayeredKeyMap) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn app_keymap(mut self, keymap: KeyMap) -> Self {
        self.keymap =
            self.keymap
                .with_layer(KeyMapLayer::with_kind("app", KeyMapLayerKind::App, keymap));
        self
    }

    pub fn mode_keymap(mut self, mode: &'static str, keymap: KeyMap) -> Self {
        self.intent_context.active_mode = Some(mode);
        self.keymap =
            self.keymap
                .with_layer(KeyMapLayer::with_kind(mode, KeyMapLayerKind::Mode, keymap));
        self
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.resize(Size::new(width, height));
        self
    }

    pub fn resize(&mut self, size: Size) {
        self.size = size;
        self.screen = Screen::new(size);
        self.request_render();
    }

    pub fn request_render(&mut self) {
        self.render_requested = true;
    }

    pub fn perf_sink(mut self, sink: impl PerfSink + 'static) -> Self {
        self.perf_sink = Box::new(sink);
        self
    }

    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }

    pub fn render_frame(&mut self) -> &Screen {
        if self.running {
            let frame_start = Instant::now();
            let previous = self.screen.clone();
            let component_start = Instant::now();
            let element = self.app.view();
            let component_time = component_start.elapsed();
            let lowering_start = Instant::now();
            let rendered = render_pipeline(&element, self.size);
            let total_pipeline_time = lowering_start.elapsed();
            let diff_start = Instant::now();
            let patch = previous.diff(&rendered.screen);
            let diff_time = diff_start.elapsed();
            self.frame_index += 1;
            let stats = FrameStats {
                frame_index: self.frame_index,
                total_frame_time: frame_start.elapsed(),
                update_time: Duration::ZERO,
                component_time,
                lowering_time: total_pipeline_time,
                layout_time: Duration::ZERO,
                paint_time: Duration::ZERO,
                backend_lowering_time: Duration::ZERO,
                diff_time,
                present_time: Duration::ZERO,
                host_node_count: count_host_nodes(&rendered.host),
                layout_node_count: rendered.layout.len(),
                paint_primitive_count: rendered.paint.len(),
                dirty_node_count: patch.regions.len(),
                dirty_regions: patch.regions.len(),
                backend_output_cells: rendered.screen.cells.len(),
                backend_output_size: rendered.screen.cells.len(),
                dirty_cells: patch.cells.len(),
            };
            self.perf_sink.record(&stats);
            self.last_frame_stats = Some(stats);
            self.screen = rendered.screen;
            self.render_requested = false;
        }
        &self.screen
    }

    pub fn render_patch(&mut self) -> ScreenPatch {
        let previous = self.screen.clone();
        let next = self.render_frame();
        previous.diff(next)
    }

    pub fn render_if_requested(&mut self) -> Option<&Screen> {
        self.render_requested.then(|| self.render_frame())
    }

    pub fn send_key(&mut self, ch: char) {
        self.handle_input(RuntimeInput::Key(KeyEvent::char(ch)));
    }

    pub fn send_ctrl_key(&mut self, ch: char) {
        self.handle_input(RuntimeInput::Key(KeyEvent::ctrl(ch)));
    }

    pub fn handle_input(&mut self, input: RuntimeInput) {
        if !self.running {
            return;
        }

        match input {
            RuntimeInput::Key(event) => {
                if let Some(intent) = self.keymap.resolve(&event) {
                    self.dispatch_intent(intent);
                }
            }
            RuntimeInput::Resize(size) => self.resize(size),
            RuntimeInput::Shutdown => self.running = false,
            RuntimeInput::Tick | RuntimeInput::BackendWake => {}
        }
        self.drain_actions();
    }

    pub fn dispatch_intent(&mut self, intent: KeyIntent) {
        if let Some(action) = self.resolver.resolve_intent(&self.intent_context, intent) {
            self.apply_key_action(action);
        }
        self.drain_actions();
    }

    pub fn dispatch_key_action(&mut self, action: KeyAction<App::Action>) {
        self.apply_key_action(action);
        self.drain_actions();
    }

    fn apply_key_action(&mut self, action: KeyAction<App::Action>) {
        let Some(action) = self
            .resolver
            .resolve_key_action(&self.intent_context, action)
        else {
            return;
        };

        match action {
            KeyAction::RuntimeQuit => {
                self.ctx.quit();
                self.running = false;
            }
            KeyAction::RuntimeCancel => {}
            KeyAction::FocusNext | KeyAction::FocusPrev | KeyAction::Control { .. } => {}
            KeyAction::App(action) => self.ctx.dispatch(action),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn last_frame_stats(&self) -> Option<FrameStats> {
        self.last_frame_stats
    }

    fn drain_actions(&mut self) {
        let mut updated = false;
        loop {
            if let Some(intent) = self.ctx.pop_key_intent() {
                if let Some(action) = self.resolver.resolve_intent(&self.intent_context, intent) {
                    self.apply_key_action(action);
                }
                continue;
            }

            if let Some(action) = self.ctx.pop_key_action() {
                self.apply_key_action(action);
                continue;
            }

            if let Some(action) = self.ctx.pop_action() {
                self.app.update(action, &mut self.ctx);
                updated = true;
                continue;
            }

            break;
        }
        if updated || self.ctx.take_render_requested() {
            self.render_requested = true;
        }
        if self.ctx.is_quit_requested() {
            self.running = false;
        }
    }
}

fn count_host_nodes<Action>(host: &thorn_core::HostNode<Action>) -> usize {
    1 + host.children.iter().map(count_host_nodes).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use thorn_core::{
        column, text, BackendInputEvent, BackendKeyEvent, BoundedInputQueue, Element,
        InputThreadDriver, InputThreadStep,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CounterAction {
        Increment,
        Decrement,
        RequestRender,
        DispatchIntent,
        DispatchAction,
        QuitFromAction,
    }

    struct CounterApp {
        count: i32,
    }

    impl ThornApp for CounterApp {
        type Action = CounterAction;

        fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
            match action {
                CounterAction::Increment => self.count += 1,
                CounterAction::Decrement => self.count -= 1,
                CounterAction::RequestRender => ctx.request_render(),
                CounterAction::DispatchIntent => {
                    ctx.dispatch_key_intent(KeyIntent::App("increment"));
                }
                CounterAction::DispatchAction => {
                    ctx.dispatch_key_action(KeyAction::App(CounterAction::Increment));
                }
                CounterAction::QuitFromAction => ctx.quit(),
            }
        }

        fn view(&self) -> Element<Self::Action> {
            column((
                text("Counter"),
                text(format!("count: {}", self.count)),
                text("+/- change, q quit"),
            ))
        }
    }

    struct CounterIntentMapper;

    impl IntentMapper<CounterAction> for CounterIntentMapper {
        fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<CounterAction>> {
            match intent {
                KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
                KeyIntent::App("increment") => Some(KeyAction::App(CounterAction::Increment)),
                KeyIntent::App("decrement") => Some(KeyAction::App(CounterAction::Decrement)),
                KeyIntent::App("request_render") => {
                    Some(KeyAction::App(CounterAction::RequestRender))
                }
                KeyIntent::App("dispatch_intent") => {
                    Some(KeyAction::App(CounterAction::DispatchIntent))
                }
                KeyIntent::App("dispatch_action") => {
                    Some(KeyAction::App(CounterAction::DispatchAction))
                }
                KeyIntent::App("quit_from_action") => {
                    Some(KeyAction::App(CounterAction::QuitFromAction))
                }
                _ => None,
            }
        }
    }

    struct VecEventSource {
        events: VecDeque<BackendInputEvent>,
    }

    impl VecEventSource {
        fn new(events: impl Into<VecDeque<BackendInputEvent>>) -> Self {
            Self {
                events: events.into(),
            }
        }
    }

    impl thorn_core::BackendEventSource for VecEventSource {
        fn read_event(&mut self) -> Option<BackendInputEvent> {
            self.events.pop_front()
        }
    }

    fn counter_runtime() -> AppRuntime<CounterApp> {
        AppRuntime::new(CounterApp { count: 0 }, CounterIntentMapper).size(40, 8)
    }

    #[test]
    fn initial_runtime_requests_render() {
        let runtime = counter_runtime();

        assert!(runtime.is_render_requested());
    }

    #[test]
    fn render_frame_updates_screen_and_clears_request() {
        let mut runtime = counter_runtime();

        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 0"));
        assert!(!runtime.is_render_requested());
    }

    #[test]
    fn app_action_updates_state_and_requests_render() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.send_key('+');

        assert!(runtime.is_render_requested());
        assert!(runtime.render_frame().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn render_patch_reports_changed_cells_between_frames() {
        let mut runtime = counter_runtime();
        runtime.render_frame();
        runtime.send_key('+');

        let patch = runtime.render_patch();

        assert!(!patch.full);
        assert_eq!(patch.cells.len(), 1);
        assert_eq!(patch.cells[0].cell.ch, '1');
    }

    #[test]
    fn resize_replaces_screen_and_requests_render() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.handle_input(RuntimeInput::Resize(Size::new(20, 4)));

        assert!(runtime.is_render_requested());
        assert_eq!(runtime.screen().size, Size::new(20, 4));
    }

    #[test]
    fn quit_intent_stops_runtime() {
        let mut runtime = counter_runtime();

        runtime.send_key('q');

        assert!(!runtime.is_running());
    }

    #[test]
    fn custom_keymap_can_override_default_intents() {
        let mut runtime = AppRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
            .keymap(KeyMap::new().bind(KeyEvent::char('n'), KeyIntent::App("increment")))
            .size(40, 8);

        runtime.send_key('+');
        runtime.render_frame();
        assert!(runtime.screen().to_plain_text().contains("count: 0"));

        runtime.send_key('n');
        runtime.render_frame();
        assert!(runtime.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn custom_keymap_cannot_disable_ctrl_c_reserved_quit() {
        let mut runtime = AppRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
            .keymap(KeyMap::new())
            .size(40, 8);

        runtime.send_ctrl_key('c');

        assert!(!runtime.is_running());
    }

    #[test]
    fn render_frame_records_frame_stats() {
        let mut runtime = counter_runtime();

        runtime.render_frame();
        let stats = runtime.last_frame_stats().unwrap();

        assert_eq!(stats.frame_index, 1);
        assert_eq!(stats.host_node_count, 4);
        assert_eq!(stats.layout_node_count, 4);
        assert_eq!(stats.paint_primitive_count, 3);
        assert!(stats.backend_output_cells > 0);
        assert_eq!(stats.backend_output_size, stats.backend_output_cells);
        assert!(stats.dirty_regions > 0);
    }

    #[derive(Default)]
    struct RecordingSink {
        frames: Rc<RefCell<Vec<FrameStats>>>,
    }

    impl PerfSink for RecordingSink {
        fn record(&mut self, stats: &FrameStats) {
            self.frames.borrow_mut().push(*stats);
        }
    }

    #[test]
    fn perf_sink_receives_frame_stats_when_enabled() {
        let frames = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = AppRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
            .perf_sink(RecordingSink {
                frames: Rc::clone(&frames),
            })
            .size(40, 8);

        runtime.render_frame();

        let frames = frames.borrow();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame_index, 1);
        assert_eq!(frames[0].paint_primitive_count, 3);
    }

    #[test]
    fn noop_perf_sink_accepts_stats_without_string_formatting_contract() {
        let mut sink = NoopPerfSink;
        let stats = FrameStats {
            frame_index: 1,
            total_frame_time: Duration::ZERO,
            update_time: Duration::ZERO,
            component_time: Duration::ZERO,
            lowering_time: Duration::ZERO,
            layout_time: Duration::ZERO,
            paint_time: Duration::ZERO,
            backend_lowering_time: Duration::ZERO,
            diff_time: Duration::ZERO,
            present_time: Duration::ZERO,
            host_node_count: 0,
            layout_node_count: 0,
            paint_primitive_count: 0,
            dirty_node_count: 0,
            dirty_regions: 0,
            backend_output_cells: 0,
            backend_output_size: 0,
            dirty_cells: 0,
        };

        sink.record(&stats);
    }

    #[test]
    fn input_thread_cannot_mutate_app_state_until_ui_drains_runtime_input() {
        let mut runtime = counter_runtime();
        runtime.render_frame();
        let mut queue = BoundedInputQueue::new(4);
        let mut input_thread = InputThreadDriver::new(VecEventSource::new(VecDeque::from([
            BackendInputEvent::Key(BackendKeyEvent::char('+')),
        ])));

        assert_eq!(input_thread.step(&mut queue), InputThreadStep::Queued);

        runtime.render_frame();
        assert!(runtime.screen().to_plain_text().contains("count: 0"));

        while let Some(input) = queue.pop() {
            runtime.handle_input(input);
        }
        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn app_context_dispatch_key_intent_uses_same_resolver_path() {
        let mut runtime = counter_runtime()
            .keymap(KeyMap::new().bind(KeyEvent::char('i'), KeyIntent::App("dispatch_intent")));

        runtime.send_key('i');
        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn app_context_dispatch_key_action_uses_same_dispatch_path() {
        let mut runtime = counter_runtime()
            .keymap(KeyMap::new().bind(KeyEvent::char('a'), KeyIntent::App("dispatch_action")));

        runtime.send_key('a');
        runtime.render_frame();

        assert!(runtime.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn application_action_can_request_quit() {
        let mut runtime = counter_runtime()
            .keymap(KeyMap::new().bind(KeyEvent::char('x'), KeyIntent::App("quit_from_action")));

        runtime.send_key('x');

        assert!(!runtime.is_running());
    }
}
