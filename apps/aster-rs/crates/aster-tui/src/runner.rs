use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Context;
use arbor_tui_application::app::App;
use arbor_tui_domain::input::{Key, KeyEvent};
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_runtime::{run_crossterm_terminal_app, TerminalApp};
use arbor_tui_widgets::widget_factory::WidgetFactory;
use aster_adapters::DeepSeekClient;
use aster_application::ChatStreamPort;
use aster_domain::ConversationStatus;

use crate::state::AppState;
use crate::ui::{build_ui, estimate_line_count};

const UI_CHROME_ROWS: u16 = 7;

#[derive(Copy, Clone)]
struct ScrollConfig {
    line_step: u16,
    page_step: u16,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            line_step: 1,
            page_step: 10,
        }
    }
}

#[derive(Clone)]
struct AsterRuntime {
    state: Rc<RefCell<AppState>>,
    scroll_y: Rc<Signal<u16>>,
    loading_phase: Rc<Signal<usize>>,
    scroll: ScrollConfig,
}

impl AsterRuntime {
    #[cfg(test)]
    fn new(client: impl ChatStreamPort + 'static) -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::new(client))),
            scroll_y: Rc::new(Signal::new(0)),
            loading_phase: Rc::new(Signal::new(0)),
            scroll: ScrollConfig::default(),
        }
    }

    fn with_model(client: impl ChatStreamPort + 'static, model: impl Into<String>) -> Self {
        Self {
            state: Rc::new(RefCell::new(AppState::with_model(client, model))),
            scroll_y: Rc::new(Signal::new(0)),
            loading_phase: Rc::new(Signal::new(0)),
            scroll: ScrollConfig::default(),
        }
    }

    fn build_ui(&self, theme: &Theme, cols: u16, rows: u16) -> WidgetNode {
        let factory = WidgetFactory::new();
        build_ui(
            &factory,
            theme,
            &self.state,
            self.scroll_y.read_only(),
            self.loading_phase.get(),
            cols,
            rows,
        )
    }

    fn handle_events(&self, app: &mut App, theme: &Theme, events: &mut Vec<KeyEvent>) -> bool {
        let mut needs_render = false;
        let mut scroll_delta = 0i32;
        let mut remaining = Vec::with_capacity(events.len());
        let viewport = MessageViewport::from_rows(app.screen_size().1);

        for event in events.drain(..) {
            let consumed = match &event.key {
                Key::Enter if self.is_streaming() => {
                    self.state.borrow_mut().cancel_stream();
                    needs_render = true;
                    true
                }
                Key::Escape if self.is_streaming() => {
                    self.state.borrow_mut().cancel_stream();
                    needs_render = true;
                    true
                }
                Key::Escape if self.is_palette_open() => {
                    self.state.borrow_mut().close_palette();
                    needs_render = true;
                    true
                }
                Key::Escape if self.is_error() => {
                    self.state.borrow_mut().dismiss_error();
                    needs_render = true;
                    true
                }
                Key::Enter if self.is_palette_open() => {
                    needs_render |= self.state.borrow_mut().accept_palette_selection();
                    true
                }
                Key::ArrowUp if self.is_palette_open() => {
                    self.state.borrow_mut().move_palette_selection(-1);
                    needs_render = true;
                    true
                }
                Key::ArrowDown if self.is_palette_open() => {
                    self.state.borrow_mut().move_palette_selection(1);
                    needs_render = true;
                    true
                }
                Key::ArrowUp => {
                    scroll_delta = scroll_delta.saturating_sub(self.line_step());
                    true
                }
                Key::ArrowDown => {
                    scroll_delta = scroll_delta.saturating_add(self.line_step());
                    true
                }
                Key::PageUp => {
                    scroll_delta = scroll_delta.saturating_sub(self.page_step());
                    true
                }
                Key::PageDown => {
                    scroll_delta = scroll_delta.saturating_add(self.page_step());
                    true
                }
                Key::Home => {
                    scroll_delta = 0;
                    needs_render |= self.set_scroll_y(app, 0);
                    true
                }
                Key::End => {
                    scroll_delta = 0;
                    needs_render |= self.scroll_to_bottom(app, theme, viewport);
                    true
                }
                _ => false,
            };

            if !consumed {
                remaining.push(event);
            }
        }

        if scroll_delta != 0 {
            needs_render |= self.move_scroll(app, theme, viewport, scroll_delta);
        }
        *events = remaining;
        needs_render
    }

    fn before_render(&self, app: &mut App, root: &mut WidgetNode, theme: &mut Theme) -> bool {
        let (cols, rows) = app.screen_size();
        let viewport = MessageViewport::from_rows(rows);
        let outcome = self.state.borrow_mut().poll_stream_and_take_changed();
        let mut needs_rebuild = false;
        let desired_theme = self.state.borrow().active_theme().to_theme();

        if theme.variant != desired_theme.variant {
            *theme = desired_theme;
            needs_rebuild = true;
        }

        if self.is_streaming() {
            needs_rebuild |= self.advance_loading_phase(app);
        } else {
            needs_rebuild |= self.reset_loading_phase(app);
        }

        if outcome.streamed_tokens > 0 {
            self.scroll_to_bottom(app, theme, viewport);
            needs_rebuild = true;
        }

        if outcome.state_changed {
            self.clamp_current_scroll(app, theme, viewport);
            needs_rebuild = true;
        } else if self.clamp_current_scroll(app, theme, viewport) {
            needs_rebuild = true;
        }

        if needs_rebuild {
            *root = self.build_ui(theme, cols, rows);
        }

        needs_rebuild
    }

    fn is_streaming(&self) -> bool {
        matches!(
            self.state.borrow().chat().state(),
            ConversationStatus::Streaming { .. }
        )
    }

    fn is_error(&self) -> bool {
        matches!(
            self.state.borrow().chat().state(),
            ConversationStatus::Error { .. }
        )
    }

    fn is_palette_open(&self) -> bool {
        self.state.borrow().is_palette_open()
    }

    fn advance_loading_phase(&self, app: &mut App) -> bool {
        let next = self.loading_phase.get().wrapping_add(1);
        app.update_signal(&self.loading_phase, next);
        true
    }

    fn reset_loading_phase(&self, app: &mut App) -> bool {
        if self.loading_phase.get() == 0 {
            return false;
        }

        app.update_signal(&self.loading_phase, 0);
        true
    }

    fn move_scroll(
        &self,
        app: &mut App,
        theme: &Theme,
        viewport: MessageViewport,
        delta: i32,
    ) -> bool {
        let line_count = self.line_count(theme);
        let current = i32::from(self.scroll_y.get());
        let next = i32_to_u16_saturating(current.saturating_add(delta));
        self.set_scroll_y(app, clamp_scroll_y(next, line_count, viewport))
    }

    fn scroll_to_bottom(&self, app: &mut App, theme: &Theme, viewport: MessageViewport) -> bool {
        let line_count = self.line_count(theme);
        self.set_scroll_y(app, max_scroll_y(line_count, viewport))
    }

    fn clamp_current_scroll(
        &self,
        app: &mut App,
        theme: &Theme,
        viewport: MessageViewport,
    ) -> bool {
        let line_count = self.line_count(theme);
        self.set_scroll_y(
            app,
            clamp_scroll_y(self.scroll_y.get(), line_count, viewport),
        )
    }

    fn set_scroll_y(&self, app: &mut App, next: u16) -> bool {
        let before = self.scroll_y.get();
        if before == next {
            return false;
        }

        app.update_signal(&self.scroll_y, next);
        true
    }

    fn line_count(&self, theme: &Theme) -> usize {
        let state = self.state.borrow();
        let chat = state.chat();
        estimate_line_count(chat.messages(), chat.state(), theme)
    }

    fn line_step(&self) -> i32 {
        i32::from(self.scroll.line_step)
    }

    fn page_step(&self) -> i32 {
        i32::from(self.scroll.page_step)
    }
}

#[derive(Copy, Clone)]
struct MessageViewport {
    visible_rows: u16,
}

impl MessageViewport {
    fn from_rows(rows: u16) -> Self {
        Self {
            visible_rows: rows.saturating_sub(UI_CHROME_ROWS).max(1),
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let theme = Theme::dark();
    let client = DeepSeekClient::from_env().context("failed to create DeepSeek client")?;
    let initial_model = client.model().to_string();
    run_with_client(client, theme, initial_model)
}

fn run_with_client(
    client: impl ChatStreamPort + 'static,
    theme: Theme,
    initial_model: impl Into<String>,
) -> anyhow::Result<()> {
    let runtime = AsterRuntime::with_model(client, initial_model);

    let build_runtime = runtime.clone();
    let app = TerminalApp::with_builder(theme, move |cols, rows, active_theme| {
        build_runtime.build_ui(active_theme, cols, rows)
    });

    let event_runtime = runtime.clone();
    let app = app.before_events(move |app, active_theme, events| {
        event_runtime.handle_events(app, active_theme, events)
    });

    let render_runtime = runtime;
    let app = app.before_render(move |app, root, active_theme| {
        render_runtime.before_render(app, root, active_theme)
    });

    run_crossterm_terminal_app(app)
}

fn clamp_scroll_y(scroll_y: u16, line_count: usize, viewport: MessageViewport) -> u16 {
    scroll_y.min(max_scroll_y(line_count, viewport))
}

fn max_scroll_y(line_count: usize, viewport: MessageViewport) -> u16 {
    let visible_rows = usize::from(viewport.visible_rows);
    usize_to_u16_saturating(line_count.saturating_sub(visible_rows))
}

fn usize_to_u16_saturating(value: usize) -> u16 {
    value.min(usize::from(u16::MAX)) as u16
}

fn i32_to_u16_saturating(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::input::{KeyEventKind, Modifiers};
    use aster_application::{ChatRequestOptions, ChatStreamError, StreamEvent, StreamReceiver};
    use aster_domain::ChatMessage;
    use std::sync::mpsc;

    #[derive(Clone)]
    struct FakeClient {
        events: Vec<StreamEvent>,
    }

    #[derive(Clone)]
    struct PendingClient;

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            for event in self.events.clone() {
                tx.send(event).unwrap();
            }
            Ok(StreamReceiver::new(rx))
        }
    }

    impl ChatStreamPort for PendingClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            std::mem::forget(tx);
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn huge_scroll_is_clamped_when_content_fits_viewport() {
        assert_eq!(
            clamp_scroll_y(u16::MAX, 3, MessageViewport::from_rows(24)),
            0
        );
    }

    #[test]
    fn bottom_scroll_uses_content_minus_visible_rows() {
        assert_eq!(max_scroll_y(30, MessageViewport::from_rows(24)), 13);
    }

    #[test]
    fn tiny_terminal_still_has_one_visible_message_row() {
        let viewport = MessageViewport::from_rows(3);

        assert_eq!(viewport.visible_rows, 1);
        assert_eq!(max_scroll_y(5, viewport), 4);
    }

    #[test]
    fn huge_content_saturates_scroll_to_u16_max() {
        assert_eq!(
            max_scroll_y(usize::MAX, MessageViewport::from_rows(24)),
            u16::MAX
        );
    }

    #[test]
    fn streaming_escape_cancels_stream_without_quitting() {
        let runtime = AsterRuntime::new(FakeClient { events: vec![] });
        runtime.state.borrow_mut().submit_input("hello".to_string());
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut events = vec![key_event(Key::Escape)];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert!(app.is_running());
        assert_eq!(
            runtime.state.borrow().chat().state(),
            &ConversationStatus::Idle
        );
    }

    #[test]
    fn streaming_enter_cancels_stream_without_quitting() {
        let runtime = AsterRuntime::new(FakeClient { events: vec![] });
        runtime.state.borrow_mut().submit_input("hello".to_string());
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut events = vec![key_event(Key::Enter)];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert!(app.is_running());
        assert_eq!(
            runtime.state.borrow().chat().state(),
            &ConversationStatus::Idle
        );
    }

    #[test]
    fn palette_enter_accepts_completion_before_widget_submit() {
        let runtime = AsterRuntime::new(FakeClient { events: vec![] });
        runtime.state.borrow_mut().update_draft("/th".to_string());
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut events = vec![key_event(Key::Enter)];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert_eq!(runtime.state.borrow().draft(), "/theme ");
    }

    #[test]
    fn before_render_advances_loading_phase_while_streaming() {
        let runtime = AsterRuntime::new(PendingClient);
        runtime.state.borrow_mut().submit_input("hello".to_string());
        let mut theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut root = runtime.build_ui(&theme, 80, 24);

        let needs_render = runtime.before_render(&mut app, &mut root, &mut theme);

        assert!(needs_render);
        assert_eq!(runtime.loading_phase.get(), 1);
    }

    #[test]
    fn home_event_updates_scroll_before_widget_keymap() {
        let runtime = AsterRuntime::new(FakeClient { events: vec![] });
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        app.update_signal(&runtime.scroll_y, 10);
        let mut events = vec![key_event(Key::Home)];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert_eq!(runtime.scroll_y.get(), 0);
    }

    #[test]
    fn scroll_keys_in_one_batch_are_coalesced() {
        let runtime = AsterRuntime::new(FakeClient {
            events: vec![StreamEvent::Token("line\n".repeat(40)), StreamEvent::Done],
        });
        runtime.state.borrow_mut().submit_input("hello".to_string());
        runtime.state.borrow_mut().poll_stream_and_take_changed();
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut events = vec![
            key_event(Key::ArrowDown),
            key_event(Key::ArrowDown),
            key_event(Key::PageDown),
        ];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert_eq!(runtime.scroll_y.get(), 12);
    }

    #[test]
    fn error_escape_dismisses_error_without_quitting() {
        let runtime = AsterRuntime::new(FakeClient {
            events: vec![StreamEvent::Error("network down".to_string())],
        });
        runtime.state.borrow_mut().submit_input("hello".to_string());
        runtime.state.borrow_mut().poll_stream_and_take_changed();
        assert!(matches!(
            runtime.state.borrow().chat().state(),
            ConversationStatus::Error { .. }
        ));
        let theme = Theme::dark();
        let mut app = App::new(80, 24);
        app.run();
        let mut events = vec![key_event(Key::Escape)];

        let needs_render = runtime.handle_events(&mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(needs_render);
        assert!(app.is_running());
        assert_eq!(
            runtime.state.borrow().chat().state(),
            &ConversationStatus::Idle
        );
    }

    fn key_event(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }
    }
}
