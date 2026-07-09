use std::time::{Duration, Instant};
use thorn_core::{
    layout_tree, lower_element, paint_tree_with_theme, text_display_width, AppContext,
    BackendCapabilities, DirtyKind, FrameInvalidation, HostKind, IntentContext, IntentMapper,
    IntentResolver, KeyAction, KeyEvent, KeyIntent, KeyMap, KeyMapLayer, KeyMapLayerKind,
    LayeredKeyMap, LayoutNode, LayoutStyle, RuntimeInput, Screen, ScreenPatch, Size, ThornApp,
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
    pub invalidation_kind: DirtyKind,
    pub layout_cache_hit: bool,
    pub layout_passes: usize,
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

#[derive(Debug, Clone)]
struct RetainedLayoutCache {
    size: Size,
    fingerprint: LayoutFingerprint,
    layout: Vec<LayoutNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LayoutFingerprint {
    kind: HostKind,
    layout_style: LayoutStyle,
    text_width: Option<u16>,
    children: Vec<LayoutFingerprint>,
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
    layout_cache: Option<RetainedLayoutCache>,
    patch_base_screen: Option<Screen>,
    running: bool,
    pending_invalidation: Option<FrameInvalidation>,
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
            layout_cache: None,
            patch_base_screen: None,
            running: true,
            pending_invalidation: Some(FrameInvalidation::new(DirtyKind::Full)),
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
        let previous_screen = self.screen.clone();
        self.size = size;
        self.screen = Screen::new(size);
        self.patch_base_screen = Some(previous_screen);
        self.request_invalidation(DirtyKind::Layout);
    }

    pub fn request_render(&mut self) {
        self.request_invalidation(DirtyKind::Render);
    }

    pub fn request_invalidation(&mut self, kind: DirtyKind) {
        self.merge_invalidation(FrameInvalidation::new(kind));
    }

    pub fn perf_sink(mut self, sink: impl PerfSink + 'static) -> Self {
        self.perf_sink = Box::new(sink);
        self
    }

    pub fn is_render_requested(&self) -> bool {
        self.pending_invalidation.is_some()
    }

    pub fn render_frame(&mut self) -> &Screen {
        if self.running {
            let frame_start = Instant::now();
            let previous = self
                .patch_base_screen
                .take()
                .unwrap_or_else(|| self.screen.clone());
            let invalidation = self
                .pending_invalidation
                .take()
                .unwrap_or(FrameInvalidation::new(DirtyKind::Render));
            let component_start = Instant::now();
            let element = self.app.view();
            let component_time = component_start.elapsed();
            let lowering_start = Instant::now();
            let host = lower_element(&element);
            let lowering_time = lowering_start.elapsed();
            let host_node_count = count_host_nodes(&host);
            let layout_fingerprint = layout_fingerprint(&host);
            let layout_start = Instant::now();
            let (layout, layout_cache_hit, layout_passes) =
                self.layout_for_frame(invalidation.kind(), &host, layout_fingerprint);
            let layout_time = if layout_cache_hit {
                Duration::ZERO
            } else {
                layout_start.elapsed()
            };
            let paint_start = Instant::now();
            let paint = paint_tree_with_theme(&host, &layout, self.ctx.theme(), self.size);
            let paint_time = paint_start.elapsed();
            let backend_lowering_start = Instant::now();
            let mut screen = Screen::new(self.size);
            screen.apply(&paint);
            let backend_lowering_time = backend_lowering_start.elapsed();
            let diff_start = Instant::now();
            let patch = previous.diff(&screen);
            let diff_time = diff_start.elapsed();
            self.frame_index += 1;
            let stats = FrameStats {
                frame_index: self.frame_index,
                total_frame_time: frame_start.elapsed(),
                update_time: Duration::ZERO,
                component_time,
                lowering_time,
                layout_time,
                paint_time,
                backend_lowering_time,
                diff_time,
                present_time: Duration::ZERO,
                host_node_count,
                layout_node_count: layout.len(),
                paint_primitive_count: paint.len(),
                invalidation_kind: invalidation.kind(),
                layout_cache_hit,
                layout_passes,
                dirty_node_count: dirty_node_count_for_invalidation(
                    invalidation.kind(),
                    host_node_count,
                ),
                dirty_regions: patch.regions.len(),
                backend_output_cells: screen.cells.len(),
                backend_output_size: screen.cells.len(),
                dirty_cells: patch.cells.len(),
            };
            self.perf_sink.record(&stats);
            self.last_frame_stats = Some(stats);
            self.screen = screen;
        }
        &self.screen
    }

    pub fn render_patch(&mut self) -> ScreenPatch {
        let previous = self
            .patch_base_screen
            .clone()
            .unwrap_or_else(|| self.screen.clone());
        let next = self.render_frame();
        previous.diff(next)
    }

    pub fn render_if_requested(&mut self) -> Option<&Screen> {
        self.pending_invalidation
            .is_some()
            .then(|| self.render_frame())
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
                    self.apply_intent(intent);
                }
            }
            RuntimeInput::Resize(size) => self.resize(size),
            RuntimeInput::Shutdown => self.running = false,
            RuntimeInput::Tick | RuntimeInput::BackendWake => {}
        }
        self.drain_actions();
    }

    pub fn dispatch_intent(&mut self, intent: KeyIntent) {
        self.apply_intent(intent);
        self.drain_actions();
    }

    pub fn dispatch_key_action(&mut self, action: KeyAction<App::Action>) {
        self.apply_resolved_key_action(action);
        self.drain_actions();
    }

    fn apply_intent(&mut self, intent: KeyIntent) {
        if let Some(action) = self.resolver.resolve_intent(&self.intent_context, intent) {
            self.apply_resolved_key_action(action);
        }
    }

    fn apply_resolved_key_action(&mut self, action: KeyAction<App::Action>) {
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

    pub fn app_context(&self) -> &AppContext<App::Action> {
        &self.ctx
    }

    pub fn intent_context(&self) -> &IntentContext {
        &self.intent_context
    }

    fn drain_actions(&mut self) {
        let mut updated = false;
        loop {
            if let Some(intent) = self.ctx.pop_key_intent() {
                if let Some(action) = self.resolver.resolve_intent(&self.intent_context, intent) {
                    self.apply_resolved_key_action(action);
                }
                continue;
            }

            if let Some(action) = self.ctx.pop_key_action() {
                self.apply_resolved_key_action(action);
                continue;
            }

            if let Some(action) = self.ctx.pop_action() {
                self.app.update(action, &mut self.ctx);
                updated = true;
                continue;
            }

            break;
        }
        if updated {
            self.request_invalidation(DirtyKind::Render);
        }
        if let Some(invalidation) = self.ctx.take_invalidation() {
            self.merge_invalidation(invalidation);
        }
        if self.ctx.is_quit_requested() {
            self.running = false;
        }
    }

    fn merge_invalidation(&mut self, invalidation: FrameInvalidation) {
        if invalidation.kind() != DirtyKind::Render {
            self.layout_cache = None;
        }
        match self.pending_invalidation.as_mut() {
            Some(existing) => existing.merge(invalidation),
            None => self.pending_invalidation = Some(invalidation),
        }
    }

    fn layout_for_frame(
        &mut self,
        invalidation_kind: DirtyKind,
        host: &thorn_core::HostNode<App::Action>,
        layout_fingerprint: LayoutFingerprint,
    ) -> (Vec<LayoutNode>, bool, usize) {
        if self.can_reuse_layout_cache(invalidation_kind, &layout_fingerprint) {
            let cached_layout = self
                .layout_cache
                .as_ref()
                .expect("layout cache must exist on cache hit")
                .layout
                .clone();
            return (cached_layout, true, 0);
        }

        let layout = layout_tree(host, self.size);
        self.layout_cache = Some(RetainedLayoutCache {
            size: self.size,
            fingerprint: layout_fingerprint,
            layout: layout.clone(),
        });
        (layout, false, 1)
    }

    fn can_reuse_layout_cache(
        &self,
        invalidation_kind: DirtyKind,
        layout_fingerprint: &LayoutFingerprint,
    ) -> bool {
        invalidation_kind == DirtyKind::Render
            && self.layout_cache.as_ref().is_some_and(|cache| {
                cache.size == self.size && cache.fingerprint == *layout_fingerprint
            })
    }
}

fn count_host_nodes<Action>(host: &thorn_core::HostNode<Action>) -> usize {
    1 + host.children.iter().map(count_host_nodes).sum::<usize>()
}

fn dirty_node_count_for_invalidation(kind: DirtyKind, host_node_count: usize) -> usize {
    match kind {
        DirtyKind::Render => 0,
        DirtyKind::Layout | DirtyKind::Structure | DirtyKind::Theme | DirtyKind::Full => {
            host_node_count
        }
    }
}

fn layout_fingerprint<Action>(host: &thorn_core::HostNode<Action>) -> LayoutFingerprint {
    LayoutFingerprint {
        kind: host.kind,
        layout_style: host.layout_style,
        text_width: host.text.as_deref().map(text_display_width),
        children: host.children.iter().map(layout_fingerprint).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use thorn_core::{
        column, text, BackendFeature, BackendInputEvent, BackendKeyEvent, BoundedInputQueue,
        Element, InputThreadDriver, InputThreadStep, PaintColor, PaintStyle, Theme,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CounterAction {
        Increment,
        Decrement,
        RequestRender,
        DispatchIntent,
        DispatchAction,
        ToggleStructure,
        QuitFromAction,
    }

    struct CounterApp {
        count: i32,
        expanded: bool,
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
                CounterAction::ToggleStructure => {
                    self.expanded = !self.expanded;
                    ctx.request_invalidation(DirtyKind::Structure);
                }
                CounterAction::QuitFromAction => ctx.quit(),
            }
        }

        fn view(&self) -> Element<Self::Action> {
            let footer = if self.expanded {
                text("expanded")
            } else {
                text("+/- change, q quit")
            };

            column((
                text("Counter"),
                text(format!("count: {}", self.count)),
                footer,
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
                KeyIntent::App("toggle_structure") => {
                    Some(KeyAction::App(CounterAction::ToggleStructure))
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
        AppRuntime::new(
            CounterApp {
                count: 0,
                expanded: false,
            },
            CounterIntentMapper,
        )
        .size(40, 8)
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
        let stats = runtime.last_frame_stats().unwrap();

        assert!(!patch.full);
        assert_eq!(patch.cells.len(), 1);
        assert_eq!(patch.cells[0].cell.ch, '1');
        assert!(stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 0);
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
    fn resize_forces_next_render_patch_to_be_full() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.resize(Size::new(20, 4));

        let patch = runtime.render_patch();

        assert!(patch.full);
        assert_eq!(patch.size, Size::new(20, 4));
        assert_eq!(
            patch.cells.len(),
            usize::from(patch.size.width) * usize::from(patch.size.height)
        );
    }

    #[test]
    fn quit_intent_stops_runtime() {
        let mut runtime = counter_runtime();

        runtime.send_key('q');

        assert!(!runtime.is_running());
    }

    #[test]
    fn custom_keymap_can_override_default_intents() {
        let mut runtime = AppRuntime::new(
            CounterApp {
                count: 0,
                expanded: false,
            },
            CounterIntentMapper,
        )
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
        let mut runtime = AppRuntime::new(
            CounterApp {
                count: 0,
                expanded: false,
            },
            CounterIntentMapper,
        )
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
        assert_eq!(stats.invalidation_kind, DirtyKind::Full);
        assert!(!stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 1);
        assert_eq!(stats.dirty_node_count, stats.host_node_count);
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
        let mut runtime = AppRuntime::new(
            CounterApp {
                count: 0,
                expanded: false,
            },
            CounterIntentMapper,
        )
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
            invalidation_kind: DirtyKind::Render,
            layout_cache_hit: false,
            layout_passes: 0,
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
    fn render_request_records_render_invalidation() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.send_key('+');
        runtime.render_frame();

        let stats = runtime.last_frame_stats().unwrap();
        assert_eq!(stats.invalidation_kind, DirtyKind::Render);
        assert!(stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 0);
        assert_eq!(stats.dirty_node_count, 0);
    }

    #[test]
    fn resize_records_layout_invalidation() {
        let mut runtime = counter_runtime();
        runtime.render_frame();

        runtime.resize(Size::new(20, 4));
        runtime.render_frame();

        let stats = runtime.last_frame_stats().unwrap();
        assert_eq!(stats.invalidation_kind, DirtyKind::Layout);
        assert!(!stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 1);
        assert_eq!(stats.dirty_node_count, stats.host_node_count);
    }

    #[test]
    fn backend_capabilities_are_synced_into_runtime_contexts() {
        let capabilities = BackendCapabilities::new(vec![BackendFeature::TextInput]);
        let runtime = AppRuntime::new(
            CounterApp {
                count: 0,
                expanded: false,
            },
            CounterIntentMapper,
        )
        .backend_capabilities(capabilities.clone());

        assert_eq!(runtime.intent_context().backend_capabilities, capabilities);
        assert_eq!(runtime.app_context().backend_capabilities(), &capabilities);
    }

    #[test]
    fn resolver_can_gate_actions_on_backend_capabilities() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum CapabilityAction {
            Increment,
        }

        struct CapabilityApp {
            count: i32,
        }

        impl ThornApp for CapabilityApp {
            type Action = CapabilityAction;

            fn update(&mut self, action: Self::Action, _ctx: &mut AppContext<Self::Action>) {
                match action {
                    CapabilityAction::Increment => self.count += 1,
                }
            }

            fn view(&self) -> Element<Self::Action> {
                text(format!("count: {}", self.count))
            }
        }

        struct CapabilityResolver;

        impl IntentResolver<CapabilityAction> for CapabilityResolver {
            fn resolve_intent(
                &self,
                context: &IntentContext,
                intent: KeyIntent,
            ) -> Option<KeyAction<CapabilityAction>> {
                match intent {
                    KeyIntent::App("text_input_only")
                        if context
                            .backend_capabilities
                            .supports(BackendFeature::TextInput) =>
                    {
                        Some(KeyAction::App(CapabilityAction::Increment))
                    }
                    _ => None,
                }
            }
        }

        let keymap = KeyMap::new().bind(KeyEvent::char('t'), KeyIntent::App("text_input_only"));

        let mut denied = AppRuntime::with_resolver(CapabilityApp { count: 0 }, CapabilityResolver)
            .keymap(keymap.clone())
            .size(20, 2);
        denied.render_frame();
        denied.send_key('t');
        denied.render_frame();
        assert!(denied.screen().to_plain_text().contains("count: 0"));

        let mut allowed = AppRuntime::with_resolver(CapabilityApp { count: 0 }, CapabilityResolver)
            .keymap(keymap)
            .backend_capabilities(BackendCapabilities::new(vec![BackendFeature::TextInput]))
            .size(20, 2);
        allowed.render_frame();
        allowed.send_key('t');
        allowed.render_frame();
        assert!(allowed.screen().to_plain_text().contains("count: 1"));
    }

    #[test]
    fn theme_change_requests_theme_invalidation_and_reaches_render_pipeline() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum ThemeAction {
            ApplyTheme,
        }

        struct ThemeApp;

        impl ThornApp for ThemeApp {
            type Action = ThemeAction;

            fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
                match action {
                    ThemeAction::ApplyTheme => ctx.set_theme(Theme::new(PaintStyle {
                        background: Some(PaintColor::Indexed(6)),
                        ..PaintStyle::default()
                    })),
                }
            }

            fn view(&self) -> Element<Self::Action> {
                text("themed")
            }
        }

        struct ThemeResolver;

        impl IntentMapper<ThemeAction> for ThemeResolver {
            fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<ThemeAction>> {
                match intent {
                    KeyIntent::App("apply_theme") => Some(KeyAction::App(ThemeAction::ApplyTheme)),
                    _ => None,
                }
            }
        }

        let mut runtime = AppRuntime::new(ThemeApp, ThemeResolver)
            .keymap(KeyMap::new().bind(KeyEvent::char('t'), KeyIntent::App("apply_theme")))
            .size(8, 1);
        runtime.render_frame();

        runtime.send_key('t');

        assert!(runtime.is_render_requested());
        runtime.render_frame();

        let stats = runtime.last_frame_stats().unwrap();
        assert_eq!(stats.invalidation_kind, DirtyKind::Theme);
        assert!(!stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 1);
        assert_eq!(stats.dirty_node_count, stats.host_node_count);
        assert_eq!(
            runtime.app_context().theme().canvas.background,
            Some(PaintColor::Indexed(6))
        );
        assert_eq!(
            runtime.screen().cells[0].background,
            Some(thorn_core::Color::Indexed(6))
        );
    }

    #[test]
    fn dirty_node_count_is_not_filled_from_dirty_regions() {
        let mut runtime = counter_runtime();

        runtime.render_frame();
        runtime.send_key('+');
        runtime.render_frame();

        let stats = runtime.last_frame_stats().unwrap();
        assert_eq!(stats.invalidation_kind, DirtyKind::Render);
        assert!(stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 0);
        assert_eq!(stats.dirty_node_count, 0);
        assert!(stats.dirty_regions > 0);
        assert_ne!(stats.dirty_node_count, stats.dirty_regions);
    }

    #[test]
    fn structure_invalidation_forces_layout_cache_miss() {
        let mut runtime = counter_runtime()
            .keymap(KeyMap::new().bind(KeyEvent::char('s'), KeyIntent::App("toggle_structure")));
        runtime.render_frame();

        runtime.send_key('s');
        runtime.render_frame();

        let stats = runtime.last_frame_stats().unwrap();
        assert_eq!(stats.invalidation_kind, DirtyKind::Structure);
        assert!(!stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 1);
        assert!(runtime.screen().to_plain_text().contains("expanded"));
    }

    #[test]
    fn render_dirty_text_change_with_same_char_count_forces_layout_cache_miss() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum TextWidthAction {
            Toggle,
        }

        struct TextWidthApp {
            wide: bool,
        }

        impl ThornApp for TextWidthApp {
            type Action = TextWidthAction;

            fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
                match action {
                    TextWidthAction::Toggle => {
                        self.wide = !self.wide;
                        ctx.request_render();
                    }
                }
            }

            fn view(&self) -> Element<Self::Action> {
                text(if self.wide { "中" } else { "a" })
            }
        }

        struct TextWidthMapper;

        impl IntentMapper<TextWidthAction> for TextWidthMapper {
            fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<TextWidthAction>> {
                match intent {
                    KeyIntent::App("toggle") => Some(KeyAction::App(TextWidthAction::Toggle)),
                    _ => None,
                }
            }
        }

        let mut runtime = AppRuntime::new(TextWidthApp { wide: false }, TextWidthMapper)
            .keymap(KeyMap::new().bind(KeyEvent::char('t'), KeyIntent::App("toggle")))
            .size(2, 1);
        runtime.render_frame();

        runtime.send_key('t');

        let patch = runtime.render_patch();
        let stats = runtime.last_frame_stats().unwrap();

        assert!(!stats.layout_cache_hit);
        assert_eq!(stats.layout_passes, 1);
        assert_eq!(stats.invalidation_kind, DirtyKind::Render);
        assert!(!patch.cells.is_empty());
        assert_eq!(runtime.screen().to_plain_text(), "中");
    }

    #[test]
    fn application_action_can_request_quit() {
        let mut runtime = counter_runtime()
            .keymap(KeyMap::new().bind(KeyEvent::char('x'), KeyIntent::App("quit_from_action")));

        runtime.send_key('x');

        assert!(!runtime.is_running());
    }
}
