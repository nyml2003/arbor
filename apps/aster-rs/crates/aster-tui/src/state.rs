use arbor_tui_domain::theme::Theme;
use aster_application::{ChatRequestOptions, ChatSession, ChatStreamPort};
use aster_domain::ConversationStatus;
use ofsh::{
    resolve, CommandRegistry, CommandResult, CommandSpec, CompletionEngine, CompletionKind, Lexer,
    Parser, StatementNode,
};

const THEME_CHOICES: [&str; 3] = ["dark", "light", "high-contrast"];
const MODEL_CHOICES: [&str; 2] = ["deepseek-chat", "deepseek-reasoner"];

pub struct AppState {
    chat: ChatSession<Box<dyn ChatStreamPort>>,
    commands: CommandRegistry,
    active_theme: AsterTheme,
    active_model: String,
    draft: String,
    palette: Option<CommandPalette>,
    status_message: Option<String>,
    changed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StreamPollOutcome {
    pub streamed_tokens: usize,
    pub state_changed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AsterTheme {
    Dark,
    Light,
    HighContrast,
}

impl AsterTheme {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "high-contrast" | "high_contrast" | "highcontrast" => Some(Self::HighContrast),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::HighContrast => "high-contrast",
        }
    }

    pub fn to_theme(self) -> Theme {
        match self {
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
            Self::HighContrast => Theme::high_contrast(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPalette {
    query: String,
    selected: usize,
    items: Vec<CommandPaletteItem>,
}

impl CommandPalette {
    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn items(&self) -> &[CommandPaletteItem] {
        &self.items
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandPaletteItem {
    value: String,
    kind: CompletionKind,
}

impl CommandPaletteItem {
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl AppState {
    #[cfg(test)]
    pub fn new(client: impl ChatStreamPort + 'static) -> Self {
        Self::with_model(client, MODEL_CHOICES[0])
    }

    pub fn with_model(client: impl ChatStreamPort + 'static, model: impl Into<String>) -> Self {
        Self {
            chat: ChatSession::new(Box::new(client)),
            commands: aster_command_registry(),
            active_theme: AsterTheme::Dark,
            active_model: model.into(),
            draft: String::new(),
            palette: None,
            status_message: None,
            changed: true,
        }
    }

    pub fn chat(&self) -> &ChatSession<Box<dyn ChatStreamPort>> {
        &self.chat
    }

    pub fn active_theme(&self) -> AsterTheme {
        self.active_theme
    }

    pub fn active_model(&self) -> &str {
        &self.active_model
    }

    pub fn draft(&self) -> &str {
        &self.draft
    }

    pub fn palette(&self) -> Option<&CommandPalette> {
        self.palette.as_ref()
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn is_palette_open(&self) -> bool {
        self.palette.is_some()
    }

    pub fn update_draft(&mut self, draft: String) {
        let was_open = self.palette.is_some();
        self.draft = draft;
        self.refresh_palette();
        if was_open || self.palette.is_some() {
            self.changed = true;
        }
    }

    pub fn submit_input(&mut self, message: String) {
        let message = message.trim().to_string();
        if message.is_empty() {
            self.clear_draft();
            return;
        }

        if message.starts_with('/') {
            self.execute_command(&message);
            self.clear_draft();
            return;
        }

        self.submit_message(message);
    }

    pub fn submit_message(&mut self, message: String) {
        if !matches!(self.chat.state(), ConversationStatus::Idle) {
            self.status_message =
                Some("Agent is responding. Press Esc or Enter to interrupt.".to_string());
            self.changed = true;
            return;
        }

        let options = ChatRequestOptions::new(self.active_model.clone());
        match self.chat.send(message, options) {
            Ok(()) => {
                self.status_message = None;
                self.clear_draft();
                self.changed = true;
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
                self.changed = true;
            }
        }
    }

    pub fn cancel_stream(&mut self) {
        if matches!(self.chat.state(), ConversationStatus::Streaming { .. }) {
            self.chat.cancel_stream();
            self.status_message = Some("Response interrupted.".to_string());
            self.changed = true;
        }
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.chat.state(), ConversationStatus::Error { .. }) {
            self.chat.dismiss_error();
            self.changed = true;
        }
    }

    pub fn close_palette(&mut self) {
        if self.palette.take().is_some() {
            self.changed = true;
        }
    }

    pub fn move_palette_selection(&mut self, delta: i32) {
        let Some(palette) = self.palette.as_mut() else {
            return;
        };
        let item_count = palette.items.len();
        if item_count == 0 {
            return;
        }

        let current = palette.selected as i32;
        let max_index = item_count.saturating_sub(1) as i32;
        palette.selected = current.saturating_add(delta).clamp(0, max_index) as usize;
        self.changed = true;
    }

    pub fn accept_palette_selection(&mut self) -> bool {
        let Some(item) = self.selected_palette_item().cloned() else {
            return false;
        };

        self.apply_completion(item)
    }

    pub fn accept_palette_value(&mut self, value: &str) -> bool {
        let Some(item) = self
            .palette
            .as_ref()
            .and_then(|palette| palette.items.iter().find(|item| item.value == value))
            .cloned()
        else {
            return false;
        };

        self.apply_completion(item)
    }

    fn apply_completion(&mut self, item: CommandPaletteItem) -> bool {
        match item.kind {
            CompletionKind::Command => {
                self.draft = format!("{} ", item.value);
                self.refresh_palette();
            }
            CompletionKind::Argument => {
                let Some(command) = command_name(&self.draft) else {
                    return false;
                };
                self.draft = format!("{command} {}", item.value);
                self.palette = None;
            }
        }

        self.changed = true;
        true
    }

    pub fn poll_stream_and_take_changed(&mut self) -> StreamPollOutcome {
        let before = self.chat.state().clone();
        let streamed_tokens = self.chat.poll();
        if streamed_tokens > 0 || self.chat.state() != &before {
            self.changed = true;
        }

        let state_changed = self.changed;
        self.changed = false;
        StreamPollOutcome {
            streamed_tokens,
            state_changed,
        }
    }

    fn execute_command(&mut self, input: &str) {
        match self.parse_command(input) {
            Ok(statement) => self.apply_command(statement),
            Err(message) => {
                self.status_message = Some(message);
                self.changed = true;
            }
        }
    }

    fn parse_command(&self, input: &str) -> Result<StatementNode, String> {
        let tokens = Lexer::new(input)
            .tokenize()
            .map_err(|error| error.to_string())?;
        let statement = Parser::new(&tokens)
            .parse()
            .map_err(|error| error.to_string())?;
        resolve(&statement, &self.commands).map_err(|error| error.to_string())?;

        if statement.pipeline.commands.len() != 1 {
            return Err("Pipes are not supported in Aster commands yet.".to_string());
        }
        if statement.redirection.is_some() {
            return Err("Redirection is not supported in Aster commands yet.".to_string());
        }

        Ok(statement)
    }

    fn apply_command(&mut self, statement: StatementNode) {
        let command = &statement.pipeline.commands[0];
        let args = command
            .args
            .iter()
            .map(|arg| arg.value.as_str())
            .collect::<Vec<_>>();

        match command.name.value.as_str() {
            "/theme" => self.apply_theme_command(&args),
            "/model" => self.apply_model_command(&args),
            other => {
                self.status_message = Some(format!("Unknown command: {other}"));
                self.changed = true;
            }
        }
    }

    fn apply_theme_command(&mut self, args: &[&str]) {
        if args.len() != 1 {
            self.status_message = Some("Usage: /theme <dark|light|high-contrast>".to_string());
            self.changed = true;
            return;
        }

        let Some(theme) = AsterTheme::from_name(args[0]) else {
            self.status_message = Some(format!("Unknown theme: {}", args[0]));
            self.changed = true;
            return;
        };

        self.active_theme = theme;
        self.status_message = Some(format!("Theme switched to {}.", theme.name()));
        self.changed = true;
    }

    fn apply_model_command(&mut self, args: &[&str]) {
        if args.len() != 1 {
            self.status_message =
                Some("Usage: /model <deepseek-chat|deepseek-reasoner>".to_string());
            self.changed = true;
            return;
        }

        if !MODEL_CHOICES.contains(&args[0]) {
            self.status_message = Some(format!("Unknown model: {}", args[0]));
            self.changed = true;
            return;
        }

        self.active_model = args[0].to_string();
        self.status_message = Some(format!("Model switched to {}.", self.active_model));
        self.changed = true;
    }

    fn refresh_palette(&mut self) {
        if !self.draft.trim_start().starts_with('/') {
            self.palette = None;
            return;
        }

        let selected = self.palette.as_ref().map_or(0, CommandPalette::selected);
        let engine = CompletionEngine::new(&self.commands);
        let items = engine
            .complete(self.draft.trim_start())
            .into_iter()
            .map(|item| CommandPaletteItem {
                value: item.value,
                kind: item.kind,
            })
            .collect::<Vec<_>>();

        let selected = selected.min(items.len().saturating_sub(1));
        self.palette = Some(CommandPalette {
            query: completion_query(&self.draft),
            selected,
            items,
        });
    }

    fn selected_palette_item(&self) -> Option<&CommandPaletteItem> {
        let palette = self.palette.as_ref()?;
        palette.items.get(palette.selected)
    }

    fn clear_draft(&mut self) {
        if !self.draft.is_empty() || self.palette.is_some() {
            self.draft.clear();
            self.palette = None;
            self.changed = true;
        }
    }

    #[cfg(test)]
    fn take_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed
    }
}

fn aster_command_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    registry.register(
        CommandSpec::new("/theme", "Switch UI theme").with_arg_completions(THEME_CHOICES),
        |_| CommandResult::output(""),
    );
    registry.register(
        CommandSpec::new("/model", "Switch DeepSeek model").with_arg_completions(MODEL_CHOICES),
        |_| CommandResult::output(""),
    );
    registry
}

fn command_name(input: &str) -> Option<&str> {
    input.split_whitespace().next()
}

fn completion_query(input: &str) -> String {
    let input = input.trim_start();
    if input.chars().last().is_some_and(char::is_whitespace) {
        return String::new();
    }

    input.split_whitespace().last().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_application::{ChatRequestOptions, ChatStreamError, StreamEvent, StreamReceiver};
    use aster_domain::ChatMessage;
    use std::sync::mpsc;

    #[derive(Clone)]
    struct FakeClient;

    #[derive(Clone)]
    struct DoneOnlyClient;

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamEvent::Token("hello".to_string())).unwrap();
            tx.send(StreamEvent::Done).unwrap();
            Ok(StreamReceiver::new(rx))
        }
    }

    impl ChatStreamPort for DoneOnlyClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamEvent::Done).unwrap();
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn submitting_message_marks_state_changed() {
        let mut state = AppState::new(FakeClient);

        state.take_changed();
        state.submit_input("hi".to_string());

        assert!(state.take_changed());
    }

    #[test]
    fn polling_tokens_marks_state_changed() {
        let mut state = AppState::new(FakeClient);

        state.submit_input("hi".to_string());
        state.take_changed();
        let outcome = state.poll_stream_and_take_changed();

        assert_eq!(outcome.streamed_tokens, 1);
        assert!(outcome.state_changed);
    }

    #[test]
    fn polling_done_marks_state_changed_even_without_new_tokens() {
        let mut state = AppState::new(DoneOnlyClient);

        state.submit_input("hi".to_string());
        state.take_changed();
        let outcome = state.poll_stream_and_take_changed();

        assert_eq!(outcome.streamed_tokens, 0);
        assert_eq!(state.chat().state(), &ConversationStatus::Idle);
        assert!(outcome.state_changed);
    }

    #[test]
    fn cancel_stream_marks_state_changed_and_returns_to_idle() {
        let mut state = AppState::new(FakeClient);

        state.submit_input("hi".to_string());
        state.take_changed();
        state.cancel_stream();

        assert_eq!(state.chat().state(), &ConversationStatus::Idle);
        assert!(state.take_changed());
    }

    #[test]
    fn slash_theme_command_switches_theme() {
        let mut state = AppState::new(FakeClient);

        state.take_changed();
        state.submit_input("/theme light".to_string());

        assert_eq!(state.active_theme(), AsterTheme::Light);
        assert_eq!(state.status_message(), Some("Theme switched to light."));
        assert!(state.take_changed());
    }

    #[test]
    fn slash_model_command_switches_model() {
        let mut state = AppState::new(FakeClient);

        state.submit_input("/model deepseek-reasoner".to_string());

        assert_eq!(state.active_model(), "deepseek-reasoner");
    }

    #[test]
    fn command_draft_opens_completion_palette() {
        let mut state = AppState::new(FakeClient);

        state.take_changed();
        state.update_draft("/th".to_string());

        let palette = state.palette().expect("palette should open");
        assert_eq!(palette.query(), "/th");
        assert_eq!(palette.items()[0].value(), "/theme");
        assert!(state.take_changed());
    }

    #[test]
    fn accepting_command_completion_fills_draft() {
        let mut state = AppState::new(FakeClient);

        state.update_draft("/th".to_string());
        assert!(state.accept_palette_selection());

        assert_eq!(state.draft(), "/theme ");
        assert_eq!(state.palette().unwrap().items()[0].value(), "dark");
    }
}
