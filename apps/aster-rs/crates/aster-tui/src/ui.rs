use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_composites::{
    FuzzyPanel, FuzzyPanelSelection, Transcript, TranscriptMessage, TranscriptNotice,
};
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;
use aster_domain::{ChatMessage, ChatRole, ConversationStatus};

use crate::state::{AppState, CommandPalette};

pub fn estimate_line_count(
    messages: &[ChatMessage],
    state: &ConversationStatus,
    theme: &Theme,
) -> usize {
    build_transcript(theme, messages, state, ReadSignal::constant(0)).line_count(theme)
}

pub fn build_ui(
    factory: &WidgetFactory,
    theme: &Theme,
    state: &Rc<RefCell<AppState>>,
    scroll_y: ReadSignal<u16>,
    loading_phase: usize,
    cols: u16,
    rows: u16,
) -> WidgetNode {
    let snapshot = UiSnapshot::from_state(&state.borrow());
    let panel_bg = theme.surface();
    let title = title_for(&snapshot.conversation_state);
    let line_count = estimate_line_count(&snapshot.messages, &snapshot.conversation_state, theme);
    let transcript = build_transcript(
        theme,
        &snapshot.messages,
        &snapshot.conversation_state,
        scroll_y,
    )
    .bg(panel_bg)
    .flex(1.0)
    .build(factory, theme);

    let state_for_change = Rc::clone(state);
    let state_for_submit = Rc::clone(state);
    let input_placeholder = if snapshot.is_streaming {
        "Aster is responding - Esc/Enter interrupts"
    } else {
        "Type a message or /theme /model"
    };
    let input = Border::new()
        .fg(theme.border())
        .bg(panel_bg)
        .child(
            Input::new()
                .value(snapshot.draft.clone())
                .placeholder(input_placeholder)
                .loading(snapshot.is_streaming)
                .loading_phase(loading_phase)
                .on_change(move |message| {
                    state_for_change.borrow_mut().update_draft(message);
                })
                .on_submit(move |message| {
                    state_for_submit.borrow_mut().submit_input(message);
                })
                .build(factory, theme),
        )
        .build(factory, theme);

    let footer = Text::new(footer_text(
        line_count,
        &snapshot.theme_name,
        &snapshot.model,
        snapshot.status_message.as_deref(),
        snapshot.is_streaming,
    ))
    .fg(theme.text_dim())
    .bg(panel_bg)
    .padding(RectOffset {
        left: 1,
        ..Default::default()
    })
    .build(factory, theme);

    let mut children = vec![transcript, input];
    if let Some(palette) = snapshot.palette {
        let state_for_palette = Rc::clone(state);
        let panel = FuzzyPanel::new(palette.items)
            .title(" Commands ")
            .placeholder("Filter")
            .empty_text("No command matches")
            .query(palette.query)
            .selected_index(palette.selected)
            .rounded()
            .fg(theme.border())
            .bg(panel_bg)
            .accent(theme.accent())
            .on_submit(move |selection: FuzzyPanelSelection| {
                state_for_palette
                    .borrow_mut()
                    .accept_palette_value(&selection.item);
            })
            .build(factory, theme);
        children.push(panel);
    }
    children.push(footer);

    let inner = Col::new()
        .flex(1.0)
        .children(children)
        .build(factory, theme);

    let page = Border::new()
        .title(title)
        .rounded()
        .flex(1.0)
        .padding(RectOffset {
            top: 1,
            bottom: 1,
            left: 1,
            right: 1,
        })
        .fg(title_color(&snapshot.conversation_state, theme))
        .bg(panel_bg)
        .child(inner)
        .build(factory, theme);

    Col::new()
        .size(cols, rows)
        .children([page])
        .build(factory, theme)
}

#[derive(Clone)]
struct UiSnapshot {
    messages: Vec<ChatMessage>,
    conversation_state: ConversationStatus,
    draft: String,
    palette: Option<PaletteSnapshot>,
    model: String,
    theme_name: String,
    status_message: Option<String>,
    is_streaming: bool,
}

#[derive(Clone)]
struct PaletteSnapshot {
    query: String,
    selected: usize,
    items: Vec<String>,
}

impl UiSnapshot {
    fn from_state(state: &AppState) -> Self {
        let chat = state.chat();
        let conversation_state = chat.state().clone();
        Self {
            messages: chat.messages().to_vec(),
            is_streaming: matches!(conversation_state, ConversationStatus::Streaming { .. }),
            conversation_state,
            draft: state.draft().to_string(),
            palette: state.palette().map(PaletteSnapshot::from),
            model: state.active_model().to_string(),
            theme_name: state.active_theme().name().to_string(),
            status_message: state.status_message().map(str::to_string),
        }
    }
}

impl From<&CommandPalette> for PaletteSnapshot {
    fn from(palette: &CommandPalette) -> Self {
        Self {
            query: palette.query().to_string(),
            selected: palette.selected(),
            items: palette
                .items()
                .iter()
                .map(|item| item.value().to_string())
                .collect(),
        }
    }
}

fn build_transcript(
    theme: &Theme,
    messages: &[ChatMessage],
    state: &ConversationStatus,
    scroll_y: ReadSignal<u16>,
) -> Transcript {
    Transcript::new()
        .messages(
            messages
                .iter()
                .map(|message| transcript_message(theme, message)),
        )
        .empty_text("Welcome to Aster. Type a message and press Enter.")
        .notice(transcript_notice(theme, state))
        .scroll_y(scroll_y)
}

fn transcript_message(theme: &Theme, message: &ChatMessage) -> TranscriptMessage {
    let (label, color) = match message.role() {
        ChatRole::User => ("You", theme.accent()),
        ChatRole::Assistant => ("Aster", theme.primary()),
        ChatRole::System => ("System", theme.warning()),
        ChatRole::Other(name) => (name.as_str(), theme.text()),
    };

    TranscriptMessage::new(label, color, message.content())
}

fn transcript_notice(theme: &Theme, state: &ConversationStatus) -> Option<TranscriptNotice> {
    let ConversationStatus::Error { message } = state else {
        return None;
    };

    Some(TranscriptNotice::new(
        format!("Error: {message}"),
        "Press Esc after the stream stops, or submit another message.",
        theme.danger(),
    ))
}

fn title_for(state: &ConversationStatus) -> String {
    match state {
        ConversationStatus::Idle => " Aster - Chat ".to_string(),
        ConversationStatus::Streaming { token_count } => format!(" Aster - {token_count} tokens "),
        ConversationStatus::Error { .. } => " Aster - Error ".to_string(),
    }
}

fn title_color(state: &ConversationStatus, theme: &Theme) -> arbor_tui_domain::cell::AnsiColor {
    match state {
        ConversationStatus::Error { .. } => theme.danger(),
        ConversationStatus::Streaming { .. } => theme.primary(),
        ConversationStatus::Idle => theme.accent(),
    }
}

fn footer_text(
    line_count: usize,
    theme_name: &str,
    model: &str,
    status_message: Option<&str>,
    is_streaming: bool,
) -> String {
    let action_hint = if is_streaming {
        "Esc/Enter: interrupt"
    } else {
        "Enter: send  Esc/Ctrl+C: quit"
    };
    let base = format!("{line_count} lines | theme: {theme_name} model: {model} | {action_hint}");
    match status_message {
        Some(message) => format!("{message} | {base}"),
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::signal::Signal;
    use arbor_tui_testing::WidgetHarness;
    use aster_application::{ChatRequestOptions, ChatStreamError, ChatStreamPort, StreamReceiver};
    use aster_domain::ChatMessage;

    #[derive(Clone)]
    struct FakeClient {
        events: Vec<aster_application::StreamEvent>,
    }

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = std::sync::mpsc::channel();
            for event in self.events.clone() {
                tx.send(event).unwrap();
            }
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn welcome_screen_has_no_black_text_background_in_light_theme() {
        let factory = WidgetFactory::new();
        let theme = Theme::light();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient { events: vec![] })));
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 0, 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(!harness.find_text("Welcome to Aster").is_empty());
        harness.assert_no_black_bg_on_text().unwrap();
    }

    #[test]
    fn clamped_scroll_offset_keeps_short_reply_visible() {
        let factory = WidgetFactory::new();
        let theme = Theme::dark();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient {
            events: vec![
                aster_application::StreamEvent::Token("visible reply".to_string()),
                aster_application::StreamEvent::Done,
            ],
        })));
        state.borrow_mut().submit_input("hello".to_string());
        state.borrow_mut().poll_stream_and_take_changed();
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 0, 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(!harness.find_text("visible reply").is_empty());
    }

    #[test]
    fn streaming_state_sets_input_loading_copy() {
        let factory = WidgetFactory::new();
        let theme = Theme::dark();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient { events: vec![] })));
        state.borrow_mut().submit_input("hello".to_string());
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 1, 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(!harness.find_text("responding").is_empty());
    }

    #[test]
    fn slash_draft_renders_fuzzy_panel() {
        let factory = WidgetFactory::new();
        let theme = Theme::dark();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient { events: vec![] })));
        state.borrow_mut().update_draft("/th".to_string());
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 0, 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(!harness.find_text("/theme").is_empty());
    }
}
