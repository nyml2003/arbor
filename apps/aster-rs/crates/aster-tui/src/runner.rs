use std::cell::Cell;
use std::rc::Rc;

use anyhow::Context;
use arbor_tui::prelude::*;
use aster_adapters::DeepSeekClient;
use aster_application::ChatStreamPort;
use aster_domain::ConversationStatus;

use crate::state::AppState;
use crate::ui::{build_ui, estimate_line_count};

const UI_CHROME_ROWS: u16 = 7;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AsterAction {
    DraftChanged(String),
    SubmitInput(String),
    CancelStream,
    DismissError,
    MovePaletteSelection(i32),
    AcceptPaletteSelection,
    AcceptPaletteValue(String),
}

pub(crate) struct AsterState {
    pub(crate) app: AppState,
    pub(crate) scroll_y: Rc<Signal<u16>>,
    pub(crate) loading_phase: Rc<Signal<usize>>,
    scroll: ScrollConfig,
    line_count_cache: LineCountCache,
}

impl AsterState {
    #[cfg(test)]
    fn new(client: impl ChatStreamPort + 'static) -> Self {
        Self::with_model(client, "deepseek-chat")
    }

    pub(crate) fn with_model(
        client: impl ChatStreamPort + 'static,
        model: impl Into<String>,
    ) -> Self {
        Self {
            app: AppState::with_model(client, model),
            scroll_y: Rc::new(Signal::new(0)),
            loading_phase: Rc::new(Signal::new(0)),
            scroll: ScrollConfig::default(),
            line_count_cache: LineCountCache::default(),
        }
    }

    pub(crate) fn apply(&mut self, action: AsterAction) -> Option<Theme> {
        let before_theme = self.app.active_theme();
        match action {
            AsterAction::DraftChanged(draft) => self.app.update_draft(draft),
            AsterAction::SubmitInput(input) => self.app.submit_input(input),
            AsterAction::CancelStream => self.app.cancel_stream(),
            AsterAction::DismissError => self.app.dismiss_error(),
            AsterAction::MovePaletteSelection(delta) => self.app.move_palette_selection(delta),
            AsterAction::AcceptPaletteSelection => {
                self.app.accept_palette_selection();
            }
            AsterAction::AcceptPaletteValue(value) => {
                self.app.accept_palette_value(&value);
            }
        }

        self.line_count_cache.invalidate();
        (self.app.active_theme() != before_theme).then(|| self.app.active_theme().to_theme())
    }

    fn is_streaming(&self) -> bool {
        matches!(
            self.app.chat().state(),
            ConversationStatus::Streaming { .. }
        )
    }

    fn is_error(&self) -> bool {
        matches!(self.app.chat().state(), ConversationStatus::Error { .. })
    }

    pub(crate) fn line_count(&self, theme: &Theme) -> usize {
        self.line_count_cache.get_or_compute(|| {
            let chat = self.app.chat();
            estimate_line_count(chat.messages(), chat.state(), theme)
        })
    }

    pub(crate) fn invalidate_line_count(&self) {
        self.line_count_cache.invalidate();
    }
}

#[derive(Default)]
struct LineCountCache {
    value: Cell<Option<usize>>,
}

impl LineCountCache {
    fn get_or_compute(&self, compute: impl FnOnce() -> usize) -> usize {
        match self.value.get() {
            Some(value) => value,
            None => {
                let value = compute();
                self.value.set(Some(value));
                value
            }
        }
    }

    fn invalidate(&self) {
        self.value.set(None);
    }
}

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

pub fn run() -> Result<()> {
    let theme = Theme::dark();
    let client = DeepSeekClient::from_env().context("failed to create DeepSeek client")?;
    let initial_model = client.model().to_string();
    run_with_client(client, theme, initial_model)
}

fn run_with_client(
    client: impl ChatStreamPort + 'static,
    theme: Theme,
    initial_model: impl Into<String>,
) -> Result<()> {
    ArborApp::new(AsterState::with_model(client, initial_model))
        .theme(theme)
        .update(update)
        .view(view)
        .before_events(before_events)
        .before_render(before_render)
        .run()
}

pub(crate) fn update(
    state: &mut AsterState,
    action: AsterAction,
    ctx: &mut AppContext<AsterAction>,
) {
    if let Some(theme) = state.apply(action) {
        ctx.set_theme(theme);
    }
}

pub(crate) fn view(state: &AsterState, ui: &Ui<AsterAction>) -> Node<AsterAction> {
    build_ui(
        ui,
        &state.app,
        state.scroll_y.read_only(),
        state.loading_phase.get(),
        state.line_count(ui.theme()),
    )
}

pub(crate) fn before_events(
    state: &mut AsterState,
    ctx: &mut AppContext<AsterAction>,
    app: &mut App,
    theme: &Theme,
    events: &mut Vec<KeyEvent>,
) -> bool {
    let outcome = handle_events(state, app, theme, events);
    for action in outcome.actions {
        ctx.dispatch(action);
    }
    outcome.needs_render
}

pub(crate) fn before_render(
    state: &mut AsterState,
    _ctx: &mut AppContext<AsterAction>,
    app: &mut App,
    theme: &Theme,
) -> bool {
    render_tick(state, app, theme)
}

pub(crate) struct EventOutcome {
    needs_render: bool,
    actions: Vec<AsterAction>,
}

pub(crate) fn handle_events(
    state: &mut AsterState,
    app: &mut App,
    theme: &Theme,
    events: &mut Vec<KeyEvent>,
) -> EventOutcome {
    let mut needs_render = false;
    let mut actions = Vec::new();
    let mut scroll_delta = 0i32;
    let mut remaining = Vec::with_capacity(events.len());
    let viewport = MessageViewport::from_rows(app.screen_size().1);

    for event in events.drain(..) {
        if event.kind == KeyEventKind::Release {
            remaining.push(event);
            continue;
        }

        let consumed = match &event.key {
            Key::Enter if state.is_streaming() => {
                actions.push(AsterAction::CancelStream);
                needs_render = true;
                true
            }
            Key::Escape if state.is_streaming() => {
                actions.push(AsterAction::CancelStream);
                needs_render = true;
                true
            }
            Key::Escape if state.app.is_palette_open() => {
                state.app.close_palette();
                needs_render = true;
                true
            }
            Key::Escape if state.is_error() => {
                actions.push(AsterAction::DismissError);
                needs_render = true;
                true
            }
            Key::Enter if state.app.is_palette_open() => {
                actions.push(AsterAction::AcceptPaletteSelection);
                needs_render = true;
                true
            }
            Key::ArrowUp if state.app.is_palette_open() => {
                actions.push(AsterAction::MovePaletteSelection(-1));
                needs_render = true;
                true
            }
            Key::ArrowDown if state.app.is_palette_open() => {
                actions.push(AsterAction::MovePaletteSelection(1));
                needs_render = true;
                true
            }
            Key::ArrowUp => {
                scroll_delta = scroll_delta.saturating_sub(i32::from(state.scroll.line_step));
                true
            }
            Key::ArrowDown => {
                scroll_delta = scroll_delta.saturating_add(i32::from(state.scroll.line_step));
                true
            }
            Key::PageUp => {
                scroll_delta = scroll_delta.saturating_sub(i32::from(state.scroll.page_step));
                true
            }
            Key::PageDown => {
                scroll_delta = scroll_delta.saturating_add(i32::from(state.scroll.page_step));
                true
            }
            Key::Home => {
                scroll_delta = 0;
                needs_render |= set_scroll_y(state, app, 0);
                true
            }
            Key::End => {
                scroll_delta = 0;
                needs_render |= scroll_to_bottom(state, app, theme, viewport);
                true
            }
            _ => false,
        };

        if !consumed {
            remaining.push(event);
        }
    }

    if scroll_delta != 0 {
        needs_render |= move_scroll(state, app, theme, viewport, scroll_delta);
    }
    *events = remaining;

    EventOutcome {
        needs_render,
        actions,
    }
}

pub(crate) fn render_tick(state: &mut AsterState, app: &mut App, theme: &Theme) -> bool {
    let viewport = MessageViewport::from_rows(app.screen_size().1);
    let outcome = state.app.poll_stream_and_take_changed();
    let mut needs_rebuild = outcome.streamed_tokens > 0 || outcome.state_changed;
    if needs_rebuild {
        state.invalidate_line_count();
    }

    if state.is_streaming() {
        needs_rebuild |= advance_loading_phase(state, app);
    } else {
        needs_rebuild |= reset_loading_phase(state, app);
    }

    if outcome.streamed_tokens > 0 {
        scroll_to_bottom(state, app, theme, viewport);
    }

    needs_rebuild | clamp_current_scroll(state, app, theme, viewport)
}

fn move_scroll(
    state: &mut AsterState,
    app: &mut App,
    theme: &Theme,
    viewport: MessageViewport,
    delta: i32,
) -> bool {
    let current = i32::from(state.scroll_y.get());
    let next = i32_to_u16_saturating(current.saturating_add(delta));
    let next = clamp_scroll_y(next, line_count(state, theme), viewport);
    set_scroll_y(state, app, next)
}

fn scroll_to_bottom(
    state: &mut AsterState,
    app: &mut App,
    theme: &Theme,
    viewport: MessageViewport,
) -> bool {
    set_scroll_y(state, app, max_scroll_y(line_count(state, theme), viewport))
}

fn clamp_current_scroll(
    state: &mut AsterState,
    app: &mut App,
    theme: &Theme,
    viewport: MessageViewport,
) -> bool {
    let next = clamp_scroll_y(state.scroll_y.get(), line_count(state, theme), viewport);
    set_scroll_y(state, app, next)
}

fn set_scroll_y(state: &mut AsterState, app: &mut App, next: u16) -> bool {
    if state.scroll_y.get() == next {
        return false;
    }

    app.update_signal(&state.scroll_y, next);
    true
}

fn advance_loading_phase(state: &mut AsterState, app: &mut App) -> bool {
    app.update_signal(
        &state.loading_phase,
        state.loading_phase.get().wrapping_add(1),
    );
    true
}

fn reset_loading_phase(state: &mut AsterState, app: &mut App) -> bool {
    if state.loading_phase.get() == 0 {
        return false;
    }

    app.update_signal(&state.loading_phase, 0);
    true
}

fn line_count(state: &AsterState, theme: &Theme) -> usize {
    state.line_count(theme)
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
        let mut state = AsterState::new(FakeClient { events: vec![] });
        state.app.submit_input("hello".to_string());
        let theme = Theme::dark();
        let mut app = running_app();
        let mut events = vec![key_event(Key::Escape)];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);
        apply_actions(&mut state, outcome.actions);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert!(app.is_running());
        assert_eq!(state.app.chat().state(), &ConversationStatus::Idle);
    }

    #[test]
    fn streaming_enter_cancels_stream_without_quitting() {
        let mut state = AsterState::new(FakeClient { events: vec![] });
        state.app.submit_input("hello".to_string());
        let theme = Theme::dark();
        let mut app = running_app();
        let mut events = vec![key_event(Key::Enter)];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);
        apply_actions(&mut state, outcome.actions);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert!(app.is_running());
        assert_eq!(state.app.chat().state(), &ConversationStatus::Idle);
    }

    #[test]
    fn palette_enter_accepts_completion_before_widget_submit() {
        let mut state = AsterState::new(FakeClient { events: vec![] });
        state.app.update_draft("/th".to_string());
        let theme = Theme::dark();
        let mut app = running_app();
        let mut events = vec![key_event(Key::Enter)];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);
        apply_actions(&mut state, outcome.actions);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert_eq!(state.app.draft(), "/theme ");
    }

    #[test]
    fn before_render_advances_loading_phase_while_streaming() {
        let mut state = AsterState::new(PendingClient);
        state.app.submit_input("hello".to_string());
        let theme = Theme::dark();
        let mut app = running_app();

        let needs_render = render_tick(&mut state, &mut app, &theme);

        assert!(needs_render);
        assert_eq!(state.loading_phase.get(), 1);
    }

    #[test]
    fn line_count_cache_reuses_value_until_state_changes() {
        let mut state = AsterState::new(FakeClient {
            events: vec![StreamEvent::Token("line\n".repeat(40)), StreamEvent::Done],
        });
        state.app.submit_input("hello".to_string());
        state.app.poll_stream_and_take_changed();
        let theme = Theme::dark();

        let before = state.line_count(&theme);
        assert_eq!(state.line_count_cache.value.get(), Some(before));
        state.line_count_cache.value.set(Some(123));

        assert_eq!(state.line_count(&theme), 123);

        state.apply(AsterAction::DismissError);
        assert_eq!(state.line_count_cache.value.get(), None);
    }

    #[test]
    fn home_event_updates_scroll_before_widget_keymap() {
        let mut state = AsterState::new(FakeClient { events: vec![] });
        let theme = Theme::dark();
        let mut app = running_app();
        app.update_signal(&state.scroll_y, 10);
        let mut events = vec![key_event(Key::Home)];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert_eq!(state.scroll_y.get(), 0);
    }

    #[test]
    fn scroll_keys_in_one_batch_are_coalesced() {
        let mut state = AsterState::new(FakeClient {
            events: vec![StreamEvent::Token("line\n".repeat(40)), StreamEvent::Done],
        });
        state.app.submit_input("hello".to_string());
        state.app.poll_stream_and_take_changed();
        let theme = Theme::dark();
        let mut app = running_app();
        let mut events = vec![
            key_event(Key::ArrowDown),
            key_event(Key::ArrowDown),
            key_event(Key::PageDown),
        ];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert_eq!(state.scroll_y.get(), 12);
    }

    #[test]
    fn error_escape_dismisses_error_without_quitting() {
        let mut state = AsterState::new(FakeClient {
            events: vec![StreamEvent::Error("network down".to_string())],
        });
        state.app.submit_input("hello".to_string());
        state.app.poll_stream_and_take_changed();
        assert!(matches!(
            state.app.chat().state(),
            ConversationStatus::Error { .. }
        ));
        let theme = Theme::dark();
        let mut app = running_app();
        let mut events = vec![key_event(Key::Escape)];

        let outcome = handle_events(&mut state, &mut app, &theme, &mut events);
        apply_actions(&mut state, outcome.actions);

        assert!(events.is_empty());
        assert!(outcome.needs_render);
        assert!(app.is_running());
        assert_eq!(state.app.chat().state(), &ConversationStatus::Idle);
    }

    fn apply_actions(state: &mut AsterState, actions: Vec<AsterAction>) {
        for action in actions {
            state.apply(action);
        }
    }

    fn running_app() -> App {
        let mut app = App::new(80, 24);
        app.run();
        app
    }

    fn key_event(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }
    }
}
