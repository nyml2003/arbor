use arbor_tui::prelude::*;
use aster_domain::{ChatMessage, ChatRole, ConversationStatus};

use crate::runner::AsterAction;
use crate::state::{AppState, CommandPalette};

pub fn estimate_line_count(
    messages: &[ChatMessage],
    state: &ConversationStatus,
    theme: &Theme,
) -> usize {
    transcript_component(theme, messages, state).line_count(theme)
}

pub fn build_ui(
    ui: &Ui<AsterAction>,
    state: &AppState,
    scroll_y: ReadSignal<u16>,
    loading_phase: usize,
) -> Node<AsterAction> {
    ui.component(AsterPage::new(
        UiSnapshot::from_state(state),
        scroll_y,
        loading_phase,
    ))
}

struct AsterPage {
    props: AsterPageProps,
}

struct AsterPageProps {
    snapshot: UiSnapshot,
    scroll_y: ReadSignal<u16>,
    loading_phase: usize,
}

impl AsterPage {
    fn new(snapshot: UiSnapshot, scroll_y: ReadSignal<u16>, loading_phase: usize) -> Self {
        Self::from_props(AsterPageProps {
            snapshot,
            scroll_y,
            loading_phase,
        })
    }
}

impl PropsComponent<AsterAction> for AsterPage {
    type Props = AsterPageProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AsterAction> for AsterPage {
    fn render(self, ui: &Ui<AsterAction>) -> Node<AsterAction> {
        let theme = ui.theme();
        let panel_bg = theme.surface();
        let snapshot = self.props.snapshot;
        let title = title_for(&snapshot.conversation_state);
        let line_count =
            estimate_line_count(&snapshot.messages, &snapshot.conversation_state, theme);

        let mut body = Col::new()
            .fill()
            .child(TranscriptPane::new(
                snapshot.messages.clone(),
                snapshot.conversation_state.clone(),
                self.props.scroll_y,
            ))
            .child(ChatInputPanel::new(
                snapshot.draft.clone(),
                snapshot.is_streaming,
                self.props.loading_phase,
            ));

        if let Some(palette) = snapshot.palette.clone() {
            body = body.child(CommandPalettePanel::new(palette));
        }

        body = body.child(FooterLine::new(
            line_count,
            snapshot.theme_name,
            snapshot.model,
            snapshot.status_message,
            snapshot.is_streaming,
        ));

        ui.component(
            Panel::new(body)
                .title(title)
                .rounded()
                .fill()
                .padding(RectOffset {
                    top: 1,
                    bottom: 1,
                    left: 1,
                    right: 1,
                })
                .fg(title_color(&snapshot.conversation_state, theme))
                .bg(panel_bg),
        )
    }
}

struct TranscriptPane {
    props: TranscriptPaneProps,
}

struct TranscriptPaneProps {
    messages: Vec<ChatMessage>,
    conversation_state: ConversationStatus,
    scroll_y: ReadSignal<u16>,
}

impl TranscriptPane {
    fn new(
        messages: Vec<ChatMessage>,
        conversation_state: ConversationStatus,
        scroll_y: ReadSignal<u16>,
    ) -> Self {
        Self::from_props(TranscriptPaneProps {
            messages,
            conversation_state,
            scroll_y,
        })
    }
}

impl PropsComponent<AsterAction> for TranscriptPane {
    type Props = TranscriptPaneProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AsterAction> for TranscriptPane {
    fn render(self, ui: &Ui<AsterAction>) -> Node<AsterAction> {
        let theme = ui.theme();
        ui.component(
            transcript_component(theme, &self.props.messages, &self.props.conversation_state)
                .scroll_y(self.props.scroll_y)
                .bg(theme.surface())
                .fill(),
        )
    }
}

struct ChatInputPanel {
    props: ChatInputPanelProps,
}

struct ChatInputPanelProps {
    draft: String,
    is_streaming: bool,
    loading_phase: usize,
}

impl ChatInputPanel {
    fn new(draft: String, is_streaming: bool, loading_phase: usize) -> Self {
        Self::from_props(ChatInputPanelProps {
            draft,
            is_streaming,
            loading_phase,
        })
    }
}

impl PropsComponent<AsterAction> for ChatInputPanel {
    type Props = ChatInputPanelProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AsterAction> for ChatInputPanel {
    fn render(self, ui: &Ui<AsterAction>) -> Node<AsterAction> {
        let theme = ui.theme();
        let input_placeholder = if self.props.is_streaming {
            "Aster is responding - Esc/Enter interrupts"
        } else {
            "Type a message or /theme /model"
        };

        ui.component(
            Panel::new(
                Input::new()
                    .value(self.props.draft)
                    .placeholder(input_placeholder)
                    .loading(self.props.is_streaming)
                    .loading_phase(self.props.loading_phase)
                    .on_change(AsterAction::DraftChanged)
                    .on_submit(AsterAction::SubmitInput),
            )
            .fg(theme.border())
            .bg(theme.surface()),
        )
    }
}

struct CommandPalettePanel {
    props: CommandPalettePanelProps,
}

struct CommandPalettePanelProps {
    palette: PaletteSnapshot,
}

impl CommandPalettePanel {
    fn new(palette: PaletteSnapshot) -> Self {
        Self::from_props(CommandPalettePanelProps { palette })
    }
}

impl PropsComponent<AsterAction> for CommandPalettePanel {
    type Props = CommandPalettePanelProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AsterAction> for CommandPalettePanel {
    fn render(self, ui: &Ui<AsterAction>) -> Node<AsterAction> {
        let theme = ui.theme();
        let palette = self.props.palette;
        ui.component(
            FuzzyPanel::new(palette.items)
                .title(" Commands ")
                .placeholder("Filter")
                .empty_text("No command matches")
                .query(palette.query)
                .selected_index(palette.selected)
                .rounded()
                .fg(theme.border())
                .bg(theme.surface())
                .accent(theme.accent())
                .on_submit(|selection: FuzzyPanelSelection| {
                    AsterAction::AcceptPaletteValue(selection.item)
                }),
        )
    }
}

struct FooterLine {
    props: FooterLineProps,
}

struct FooterLineProps {
    line_count: usize,
    theme_name: String,
    model: String,
    status_message: Option<String>,
    is_streaming: bool,
}

impl FooterLine {
    fn new(
        line_count: usize,
        theme_name: String,
        model: String,
        status_message: Option<String>,
        is_streaming: bool,
    ) -> Self {
        Self::from_props(FooterLineProps {
            line_count,
            theme_name,
            model,
            status_message,
            is_streaming,
        })
    }
}

impl PropsComponent<AsterAction> for FooterLine {
    type Props = FooterLineProps;

    fn from_props(props: Self::Props) -> Self {
        Self { props }
    }

    fn into_props(self) -> Self::Props {
        self.props
    }
}

impl UiComponent<AsterAction> for FooterLine {
    fn render(self, ui: &Ui<AsterAction>) -> Node<AsterAction> {
        let theme = ui.theme();
        ui.component(
            TextBlock::new(footer_text(
                self.props.line_count,
                &self.props.theme_name,
                &self.props.model,
                self.props.status_message.as_deref(),
                self.props.is_streaming,
            ))
            .fg(theme.text_dim())
            .bg(theme.surface())
            .padding(RectOffset {
                left: 1,
                ..Default::default()
            }),
        )
    }
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

fn transcript_component(
    theme: &Theme,
    messages: &[ChatMessage],
    state: &ConversationStatus,
) -> Transcript {
    Transcript::new()
        .messages(
            messages
                .iter()
                .map(|message| transcript_message(theme, message)),
        )
        .empty_text("Welcome to Aster. Type a message and press Enter.")
        .notice(transcript_notice(theme, state))
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

fn title_color(state: &ConversationStatus, theme: &Theme) -> AnsiColor {
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
    use arbor_tui::testing::TestApp;
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
        let mut app = test_app(AppState::new(FakeClient { events: vec![] })).theme(Theme::light());

        app.render(80, 24)
            .assert_text("Welcome to Aster")
            .assert_no_default_bg();
    }

    #[test]
    fn clamped_scroll_offset_keeps_short_reply_visible() {
        let mut state = AppState::new(FakeClient {
            events: vec![
                aster_application::StreamEvent::Token("visible reply".to_string()),
                aster_application::StreamEvent::Done,
            ],
        });
        state.submit_input("hello".to_string());
        state.poll_stream_and_take_changed();
        let mut app = test_app(state);

        app.render(80, 24).assert_text("visible reply");
    }

    #[test]
    fn streaming_state_sets_input_loading_copy() {
        let mut state = AppState::new(FakeClient { events: vec![] });
        state.submit_input("hello".to_string());
        let mut app = test_app(state);

        app.render(80, 24).assert_text("responding");
    }

    #[test]
    fn slash_draft_renders_fuzzy_panel() {
        let mut state = AppState::new(FakeClient { events: vec![] });
        state.update_draft("/th".to_string());
        let mut app = test_app(state);

        app.render(80, 24).assert_text("/theme");
    }

    fn test_app(state: AppState) -> TestApp<AppState, AsterAction> {
        TestApp::new(
            state,
            |_state, _action, _ctx| {},
            |state, ui| {
                build_ui(
                    ui,
                    state,
                    ReadSignal::constant(0),
                    usize::from(matches!(
                        state.chat().state(),
                        ConversationStatus::Streaming { .. }
                    )),
                )
            },
        )
    }
}
