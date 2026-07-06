use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use anyhow::Context;
use arbor_tui_application::app::{App as RuntimeApp, FrameStats, RenderResult};
use arbor_tui_application::runtime::{runtime_step, RuntimeInput};
use arbor_tui_domain::backend::{BackendResult, TerminalBackend, TerminalGuard};
use arbor_tui_domain::diff::DirtyRegion;
use arbor_tui_domain::focus::mount_tree;
use arbor_tui_domain::input::KeyEvent;
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::app::AppContext;
use crate::ui::{build_root, ActionSink};
use crate::{Node, Ui};

type UpdateFn<State, Action> = Box<dyn FnMut(&mut State, Action, &mut AppContext<Action>)>;
type ViewFn<State, Action> = Box<dyn FnMut(&State, &Ui<Action>) -> Node<Action>>;
type BeforeEventsFn<State, Action> = Box<
    dyn FnMut(
        &mut State,
        &mut AppContext<Action>,
        &mut RuntimeApp,
        &Theme,
        &mut Vec<KeyEvent>,
    ) -> bool,
>;
type BeforeRenderFn<State, Action> =
    Box<dyn FnMut(&mut State, &mut AppContext<Action>, &mut RuntimeApp, &Theme) -> bool>;

pub struct TestApp<State, Action> {
    state: Rc<RefCell<State>>,
    update: UpdateFn<State, Action>,
    view: Rc<RefCell<ViewFn<State, Action>>>,
    theme: Theme,
    factory: Rc<WidgetFactory>,
    actions: ActionSink<Action>,
    running: bool,
}

impl<State, Action> TestApp<State, Action>
where
    State: 'static,
    Action: 'static,
{
    pub fn new(
        state: State,
        update: impl FnMut(&mut State, Action, &mut AppContext<Action>) + 'static,
        view: impl FnMut(&State, &Ui<Action>) -> Node<Action> + 'static,
    ) -> Self {
        Self {
            state: Rc::new(RefCell::new(state)),
            update: Box::new(update),
            view: Rc::new(RefCell::new(Box::new(view))),
            theme: Theme::dark(),
            factory: Rc::new(WidgetFactory::new()),
            actions: ActionSink::new(),
            running: true,
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn dispatch(&mut self, action: Action) -> &mut Self {
        self.actions.push(action);
        self.process_actions();
        self
    }

    pub fn render(&mut self, cols: u16, rows: u16) -> TestFrame {
        self.process_actions();
        let ui = Ui::new(
            Rc::clone(&self.factory),
            self.theme.clone(),
            self.actions.clone(),
        );
        let node = (self.view.borrow_mut())(&self.state.borrow(), &ui);
        let root = build_root(&self.factory, &self.theme, cols, rows, node);
        TestFrame {
            harness: WidgetHarness::render(&root, cols, rows, &self.theme),
        }
    }

    pub fn state(&self) -> std::cell::Ref<'_, State> {
        self.state.borrow()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    fn process_actions(&mut self) {
        while let Some(action) = self.actions.pop() {
            let mut ctx = AppContext::new(self.actions.clone());
            (self.update)(&mut self.state.borrow_mut(), action, &mut ctx);
            if let Some(theme) = ctx.take_theme() {
                self.theme = theme;
            }
            if ctx.should_quit() {
                self.running = false;
            }
        }
    }
}

pub struct TestFrame {
    harness: WidgetHarness,
}

impl TestFrame {
    pub fn assert_text(&self, text: &str) -> &Self {
        assert!(
            !self.harness.find_text(text).is_empty(),
            "expected screen to contain {text:?}\n{}",
            self.visible_text()
        );
        self
    }

    pub fn assert_no_default_bg(&self) -> &Self {
        self.harness
            .assert_no_black_bg_on_text()
            .expect("visible text should not use the default black background");
        self
    }

    pub fn find_text(&self, text: &str) -> Vec<(u16, u16)> {
        self.harness.find_text(text)
    }

    pub fn visible_text(&self) -> String {
        let mut text = String::new();
        for row in 0..self.harness.rows() {
            for col in 0..self.harness.cols() {
                text.push(self.harness.cell_at(col, row).ch);
            }
            if row + 1 < self.harness.rows() {
                text.push('\n');
            }
        }
        text
    }
}

pub struct HeadlessApp<State, Action> {
    state: State,
    update: UpdateFn<State, Action>,
    view: ViewFn<State, Action>,
    before_events: Option<BeforeEventsFn<State, Action>>,
    before_render: Option<BeforeRenderFn<State, Action>>,
    theme: Theme,
    actions: ActionSink<Action>,
    runtime: RuntimeApp,
    backend: HeadlessBackend,
    root: Option<WidgetNode>,
    first_frame: bool,
    root_dirty: bool,
}

impl<State, Action> HeadlessApp<State, Action>
where
    State: 'static,
    Action: 'static,
{
    pub fn new(
        state: State,
        update: impl FnMut(&mut State, Action, &mut AppContext<Action>) + 'static,
        view: impl FnMut(&State, &Ui<Action>) -> Node<Action> + 'static,
        cols: u16,
        rows: u16,
    ) -> Self {
        Self {
            state,
            update: Box::new(update),
            view: Box::new(view),
            before_events: None,
            before_render: None,
            theme: Theme::dark(),
            actions: ActionSink::new(),
            runtime: RuntimeApp::new(cols, rows),
            backend: HeadlessBackend::new(cols, rows),
            root: None,
            first_frame: true,
            root_dirty: true,
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn before_events(
        mut self,
        callback: impl FnMut(
                &mut State,
                &mut AppContext<Action>,
                &mut RuntimeApp,
                &Theme,
                &mut Vec<KeyEvent>,
            ) -> bool
            + 'static,
    ) -> Self {
        self.before_events = Some(Box::new(callback));
        self
    }

    pub fn before_render(
        mut self,
        callback: impl FnMut(&mut State, &mut AppContext<Action>, &mut RuntimeApp, &Theme) -> bool
            + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(callback));
        self
    }

    pub fn dispatch(&mut self, action: Action) {
        self.actions.push(action);
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        self.root_dirty = true;
        &mut self.state
    }

    pub fn request_rebuild(&mut self) {
        self.root_dirty = true;
    }

    pub fn tick(
        &mut self,
        events: impl IntoIterator<Item = KeyEvent>,
    ) -> anyhow::Result<HeadlessFrameStats> {
        self.mount_if_needed();

        let frame_start = Instant::now();
        let mut events = events.into_iter().collect::<Vec<_>>();
        let event_start = Instant::now();
        let mut needs_render = self.first_frame || self.root_dirty;

        if let Some(before_events) = self.before_events.as_mut() {
            let mut ctx = AppContext::new(self.actions.clone());
            if before_events(
                &mut self.state,
                &mut ctx,
                &mut self.runtime,
                &self.theme,
                &mut events,
            ) {
                self.runtime.request_render();
                needs_render = true;
            }
            self.apply_context(ctx);
        }

        let runtime_input = if self.first_frame {
            RuntimeInput::first_frame_with_events(events)
        } else {
            RuntimeInput::new(events)
        };
        let step = runtime_step(
            &mut self.runtime,
            self.root
                .as_mut()
                .context("headless app root was not mounted")?,
            &self.backend,
            runtime_input,
        )?;
        if step.should_clear {
            self.backend.clear()?;
        }
        needs_render |= step.should_render;
        let events_us = event_start.elapsed().as_micros() as u64;

        let update_start = Instant::now();
        let mut processed = self.process_actions();
        let mut update_us = update_start.elapsed().as_micros() as u64;
        if processed {
            needs_render = true;
        }

        let pre_render_start = Instant::now();
        let mut hook_changed = false;
        if let Some(before_render) = self.before_render.as_mut() {
            let mut ctx = AppContext::new(self.actions.clone());
            hook_changed = before_render(&mut self.state, &mut ctx, &mut self.runtime, &self.theme);
            self.apply_context(ctx);
        }
        let pre_render_us = pre_render_start.elapsed().as_micros() as u64;
        if hook_changed {
            self.root_dirty = true;
            needs_render = true;
        }

        let update_start = Instant::now();
        processed = self.process_actions();
        update_us = update_us.saturating_add(update_start.elapsed().as_micros() as u64);
        if processed {
            needs_render = true;
        }

        if self.root_dirty {
            self.rebuild_root();
            needs_render = true;
        }

        let mut render_result = None;
        let render_start = Instant::now();
        if needs_render {
            self.runtime.request_render();
            let result = self
                .runtime
                .render_widget_tree(
                    self.root
                        .as_ref()
                        .context("headless app root was not mounted")?,
                    &self.theme,
                    &mut self.backend,
                )
                .context("headless render failed")?;
            render_result = Some(result);
            if self.first_frame {
                let _ = self.runtime.focus_next();
            }
        }
        let render_us = render_start.elapsed().as_micros() as u64;
        self.first_frame = false;

        let frame_stats = self.runtime.last_frame_stats().clone();
        Ok(HeadlessFrameStats {
            events_us,
            update_us,
            pre_render_us,
            render_us,
            flush_us: frame_stats.emit_flush_us,
            total_us: frame_start.elapsed().as_micros() as u64,
            frame_stats,
            render_result,
        })
    }

    fn mount_if_needed(&mut self) {
        if self.root.is_some() {
            return;
        }

        self.runtime.run();
        self.rebuild_root();
    }

    fn rebuild_root(&mut self) {
        let factory = Rc::new(WidgetFactory::new());
        let (cols, rows) = self.runtime.screen_size();
        let ui = Ui::new(factory.clone(), self.theme.clone(), self.actions.clone());
        let node = (self.view)(&self.state, &ui);
        let mut root = build_root(&factory, &self.theme, cols, rows, node);
        mount_tree(&mut root);
        self.root = Some(root);
        self.runtime.request_focus_rebuild();
        self.root_dirty = false;
    }

    fn process_actions(&mut self) -> bool {
        let mut processed = false;
        while let Some(action) = self.actions.pop() {
            processed = true;
            let mut ctx = AppContext::new(self.actions.clone());
            (self.update)(&mut self.state, action, &mut ctx);
            self.apply_context(ctx);
        }
        if processed {
            self.root_dirty = true;
        }
        processed
    }

    fn apply_context(&mut self, mut ctx: AppContext<Action>) {
        if let Some(theme) = ctx.take_theme() {
            self.theme = theme;
            self.root_dirty = true;
        }
        if ctx.should_quit() {
            self.runtime.quit();
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeadlessFrameStats {
    pub events_us: u64,
    pub update_us: u64,
    pub pre_render_us: u64,
    pub render_us: u64,
    pub flush_us: u64,
    pub total_us: u64,
    pub frame_stats: FrameStats,
    pub render_result: Option<RenderResult>,
}

struct HeadlessBackend {
    screen: VirtualScreen,
}

impl HeadlessBackend {
    fn new(cols: u16, rows: u16) -> Self {
        Self {
            screen: VirtualScreen::new(cols, rows),
        }
    }
}

struct HeadlessGuard;

impl TerminalGuard for HeadlessGuard {
    fn restore(&mut self) {}
}

impl TerminalBackend for HeadlessBackend {
    fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>> {
        Ok(Box::new(HeadlessGuard))
    }

    fn size(&self) -> BackendResult<(u16, u16)> {
        Ok((self.screen.cols(), self.screen.rows()))
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()> {
        for region in regions {
            for col in region.start_col..region.end_col {
                let src = screen.cell_at(col, region.row);
                if let Some(dest) = self.screen.cell_at_mut(col, region.row) {
                    *dest = src;
                }
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> BackendResult<()> {
        Ok(())
    }

    fn show_cursor(&mut self) -> BackendResult<()> {
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> BackendResult<()> {
        Ok(())
    }

    fn exit_alternate_screen(&mut self) -> BackendResult<()> {
        Ok(())
    }

    fn clear(&mut self) -> BackendResult<()> {
        self.screen = VirtualScreen::new(self.screen.cols(), self.screen.rows());
        Ok(())
    }

    fn flush(&mut self) -> BackendResult<()> {
        Ok(())
    }
}
