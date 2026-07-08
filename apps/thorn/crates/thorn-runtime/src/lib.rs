use thorn_core::{
    render_pipeline, AppContext, IntentMapper, KeyAction, KeyEvent, KeyIntent, KeyMap,
    RuntimeInput, Screen, ScreenPatch, Size, ThornApp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStats {
    pub frame_index: u64,
    pub host_node_count: usize,
    pub layout_node_count: usize,
    pub paint_primitive_count: usize,
    pub backend_output_cells: usize,
    pub dirty_cells: usize,
}

pub struct AppRuntime<App>
where
    App: ThornApp,
{
    app: App,
    ctx: AppContext<App::Action>,
    keymap: KeyMap,
    reserved_keymap: KeyMap,
    mapper: Box<dyn IntentMapper<App::Action>>,
    size: Size,
    screen: Screen,
    running: bool,
    render_requested: bool,
    frame_index: u64,
    last_frame_stats: Option<FrameStats>,
}

impl<App> AppRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        let size = Size::new(80, 24);
        Self {
            app,
            ctx: AppContext::new(),
            keymap: KeyMap::default(),
            reserved_keymap: KeyMap::runtime_reserved(),
            mapper: Box::new(mapper),
            size,
            screen: Screen::new(size),
            running: true,
            render_requested: true,
            frame_index: 0,
            last_frame_stats: None,
        }
    }

    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.keymap = keymap;
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

    pub fn is_render_requested(&self) -> bool {
        self.render_requested
    }

    pub fn render_frame(&mut self) -> &Screen {
        if self.running {
            let previous = self.screen.clone();
            let element = self.app.view();
            let rendered = render_pipeline(&element, self.size);
            let patch = previous.diff(&rendered.screen);
            self.frame_index += 1;
            self.last_frame_stats = Some(FrameStats {
                frame_index: self.frame_index,
                host_node_count: count_host_nodes(&rendered.host),
                layout_node_count: rendered.layout.len(),
                paint_primitive_count: rendered.paint.len(),
                backend_output_cells: rendered.screen.cells.len(),
                dirty_cells: patch.cells.len(),
            });
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
                if let Some(intent) = self.reserved_keymap.resolve(&event) {
                    self.dispatch_intent(intent);
                } else if let Some(intent) = self.keymap.resolve(&event) {
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
        match self.mapper.map_intent(intent) {
            Some(KeyAction::RuntimeQuit) => {
                self.ctx.quit();
                self.running = false;
            }
            Some(KeyAction::RuntimeCancel) => {}
            Some(KeyAction::FocusNext | KeyAction::FocusPrev | KeyAction::Control { .. }) => {}
            Some(KeyAction::App(action)) => self.ctx.dispatch(action),
            None => {}
        }
        self.drain_actions();
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
        while let Some(action) = self.ctx.pop_action() {
            self.app.update(action, &mut self.ctx);
            updated = true;
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
    use thorn_core::{column, text, Element};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CounterAction {
        Increment,
        Decrement,
        RequestRender,
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
                _ => None,
            }
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
    }
}
