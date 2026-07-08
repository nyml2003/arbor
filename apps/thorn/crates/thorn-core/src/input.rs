use crate::{BackendCapabilities, HostNodeId, Size};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
    BackendWake,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub const fn char(ch: char) -> Self {
        Self {
            key: Key::Char(ch),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }

    pub const fn ctrl(ch: char) -> Self {
        Self {
            key: Key::Char(ch),
            modifiers: KeyModifiers::CTRL,
            kind: KeyEventKind::Press,
        }
    }

    pub const fn esc() -> Self {
        Self {
            key: Key::Esc,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }

    pub const fn enter() -> Self {
        Self {
            key: Key::Enter,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }

    pub const fn arrow(direction: Direction) -> Self {
        Self {
            key: Key::Arrow(direction),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Delete,
    Arrow(Direction),
    Home,
    End,
    Page(Direction),
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    bits: u8,
}

impl KeyModifiers {
    pub const CTRL: Self = Self { bits: 0b0000_0001 };
    pub const ALT: Self = Self { bits: 0b0000_0010 };
    pub const SHIFT: Self = Self { bits: 0b0000_0100 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn union(self, other: Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    Press,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKey {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Delete,
    Arrow(Direction),
    Home,
    End,
    Page(Direction),
    Tab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendKeyEvent {
    pub key: BackendKey,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub kind: KeyEventKind,
}

impl BackendKeyEvent {
    pub const fn char(ch: char) -> Self {
        Self {
            key: BackendKey::Char(ch),
            ctrl: false,
            alt: false,
            shift: false,
            kind: KeyEventKind::Press,
        }
    }

    pub const fn ctrl_char(ch: char) -> Self {
        Self {
            key: BackendKey::Char(ch),
            ctrl: true,
            alt: false,
            shift: false,
            kind: KeyEventKind::Press,
        }
    }

    pub fn into_key_event(self) -> KeyEvent {
        KeyEvent {
            key: match self.key {
                BackendKey::Char(ch) => Key::Char(ch),
                BackendKey::Enter => Key::Enter,
                BackendKey::Esc => Key::Esc,
                BackendKey::Backspace => Key::Backspace,
                BackendKey::Delete => Key::Delete,
                BackendKey::Arrow(direction) => Key::Arrow(direction),
                BackendKey::Home => Key::Home,
                BackendKey::End => Key::End,
                BackendKey::Page(direction) => Key::Page(direction),
                BackendKey::Tab => Key::Tab,
            },
            modifiers: backend_modifiers(self.ctrl, self.alt, self.shift),
            kind: self.kind,
        }
    }
}

fn backend_modifiers(ctrl: bool, alt: bool, shift: bool) -> KeyModifiers {
    let mut modifiers = KeyModifiers::empty();
    if ctrl {
        modifiers = modifiers.union(KeyModifiers::CTRL);
    }
    if alt {
        modifiers = modifiers.union(KeyModifiers::ALT);
    }
    if shift {
        modifiers = modifiers.union(KeyModifiers::SHIFT);
    }
    modifiers
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyIntent {
    RequestSubmit,
    RequestCancel,
    RequestEscape,
    RequestQuit,
    Move(Direction),
    Page(Direction),
    GoHome,
    GoEnd,
    DeleteBackward,
    DeleteForward,
    InsertText(String),
    FocusNext,
    FocusPrev,
    App(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction<Action> {
    RuntimeQuit,
    RuntimeCancel,
    FocusNext,
    FocusPrev,
    Control {
        target: HostNodeId,
        action: ControlKeyAction,
    },
    App(Action),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlKeyAction {
    Submit,
    Cancel,
    Move(Direction),
    Page(Direction),
    GoHome,
    GoEnd,
    DeleteBackward,
    DeleteForward,
    InsertText(String),
}

pub trait IntentMapper<Action> {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<Action>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedControlKind {
    TextInput,
    ScrollView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentContext {
    pub active_mode: Option<&'static str>,
    pub focused_control: Option<(HostNodeId, FocusedControlKind)>,
    pub backend_capabilities: BackendCapabilities,
}

impl Default for IntentContext {
    fn default() -> Self {
        Self {
            active_mode: None,
            focused_control: None,
            backend_capabilities: BackendCapabilities::text_only(),
        }
    }
}

pub trait IntentResolver<Action> {
    fn resolve_intent(
        &self,
        context: &IntentContext,
        intent: KeyIntent,
    ) -> Option<KeyAction<Action>>;

    fn resolve_key_action(
        &self,
        _context: &IntentContext,
        action: KeyAction<Action>,
    ) -> Option<KeyAction<Action>> {
        Some(action)
    }
}

impl<Action, T> IntentResolver<Action> for T
where
    T: IntentMapper<Action>,
{
    fn resolve_intent(
        &self,
        context: &IntentContext,
        intent: KeyIntent,
    ) -> Option<KeyAction<Action>> {
        control_action_for_intent(context, &intent)
            .map(|action| KeyAction::Control {
                target: action.0,
                action: action.1,
            })
            .or_else(|| self.map_intent(intent))
    }
}

fn control_action_for_intent(
    context: &IntentContext,
    intent: &KeyIntent,
) -> Option<(HostNodeId, ControlKeyAction)> {
    let (target, focused_kind) = context.focused_control?;
    match focused_kind {
        FocusedControlKind::TextInput => text_input_action_for_intent(intent)
            .map(|control_action| (target, control_action)),
        FocusedControlKind::ScrollView => read_only_action_for_intent(intent)
            .map(|control_action| (target, control_action)),
    }
}

fn text_input_action_for_intent(intent: &KeyIntent) -> Option<ControlKeyAction> {
    match intent {
        KeyIntent::RequestSubmit => Some(ControlKeyAction::Submit),
        KeyIntent::RequestCancel | KeyIntent::RequestEscape => Some(ControlKeyAction::Cancel),
        KeyIntent::Move(direction) => Some(ControlKeyAction::Move(*direction)),
        KeyIntent::Page(direction) => Some(ControlKeyAction::Page(*direction)),
        KeyIntent::GoHome => Some(ControlKeyAction::GoHome),
        KeyIntent::GoEnd => Some(ControlKeyAction::GoEnd),
        KeyIntent::DeleteBackward => Some(ControlKeyAction::DeleteBackward),
        KeyIntent::DeleteForward => Some(ControlKeyAction::DeleteForward),
        KeyIntent::InsertText(text) => Some(ControlKeyAction::InsertText(text.clone())),
        KeyIntent::RequestQuit | KeyIntent::FocusNext | KeyIntent::FocusPrev | KeyIntent::App(_) => {
            None
        }
    }
}

fn read_only_action_for_intent(intent: &KeyIntent) -> Option<ControlKeyAction> {
    match intent {
        KeyIntent::Move(direction) => Some(ControlKeyAction::Move(*direction)),
        KeyIntent::Page(direction) => Some(ControlKeyAction::Page(*direction)),
        KeyIntent::GoHome => Some(ControlKeyAction::GoHome),
        KeyIntent::GoEnd => Some(ControlKeyAction::GoEnd),
        KeyIntent::RequestSubmit
        | KeyIntent::RequestCancel
        | KeyIntent::RequestEscape
        | KeyIntent::RequestQuit
        | KeyIntent::DeleteBackward
        | KeyIntent::DeleteForward
        | KeyIntent::InsertText(_)
        | KeyIntent::FocusNext
        | KeyIntent::FocusPrev
        | KeyIntent::App(_) => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMapError {
    pub event: KeyEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap {
    bindings: Vec<(KeyEvent, KeyIntent)>,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new()
            .bind(KeyEvent::char('+'), KeyIntent::App("increment"))
            .bind(KeyEvent::char('-'), KeyIntent::App("decrement"))
            .bind(KeyEvent::char('q'), KeyIntent::RequestQuit)
    }
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn runtime_reserved() -> Self {
        Self::new().bind(KeyEvent::ctrl('c'), KeyIntent::RequestQuit)
    }

    pub fn bind(mut self, event: KeyEvent, intent: KeyIntent) -> Self {
        self.bindings.push((event, intent));
        self
    }

    pub fn try_bind(mut self, event: KeyEvent, intent: KeyIntent) -> Result<Self, KeyMapError> {
        if self.has_binding(&event) {
            return Err(KeyMapError { event });
        }
        self.bindings.push((event, intent));
        Ok(self)
    }

    pub fn has_binding(&self, event: &KeyEvent) -> bool {
        self.bindings
            .iter()
            .any(|(bound_event, _)| bound_event == event)
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<KeyIntent> {
        self.bindings
            .iter()
            .find_map(|(bound_event, intent)| (bound_event == event).then(|| intent.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyMapResult {
    Handled(KeyIntent),
    Pass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMapLayer {
    pub name: &'static str,
    pub kind: KeyMapLayerKind,
    pub keymap: KeyMap,
}

impl KeyMapLayer {
    pub fn new(name: &'static str, keymap: KeyMap) -> Self {
        Self {
            name,
            kind: KeyMapLayerKind::App,
            keymap,
        }
    }

    pub fn with_kind(name: &'static str, kind: KeyMapLayerKind, keymap: KeyMap) -> Self {
        Self { name, kind, keymap }
    }

    pub fn resolve(&self, event: &KeyEvent) -> KeyMapResult {
        self.keymap
            .resolve(event)
            .map(KeyMapResult::Handled)
            .unwrap_or(KeyMapResult::Pass)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KeyMapLayerKind {
    PlatformFallback = 0,
    Runtime = 1,
    Mode = 2,
    App = 3,
    FocusedControl = 4,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredKeyMap {
    layers: Vec<KeyMapLayer>,
    reserved: KeyMap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeredKeyMapResolution {
    pub layer: &'static str,
    pub intent: KeyIntent,
}

impl Default for LayeredKeyMap {
    fn default() -> Self {
        Self::new()
            .with_layer(KeyMapLayer::with_kind(
                "mode:default",
                KeyMapLayerKind::Mode,
                DefaultKeyMap::new(),
            ))
            .with_layer(KeyMapLayer::with_kind(
                "platform:fallback",
                KeyMapLayerKind::PlatformFallback,
                PlatformFallbackKeyMap::new(),
            ))
    }
}

impl LayeredKeyMap {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            reserved: KeyMap::runtime_reserved(),
        }
    }

    pub fn app_only(keymap: KeyMap) -> Self {
        Self::new().with_layer(KeyMapLayer::with_kind(
            "app",
            KeyMapLayerKind::App,
            keymap,
        ))
    }

    pub fn with_layer(mut self, layer: KeyMapLayer) -> Self {
        self.layers.push(layer);
        self.layers.sort_by(|a, b| b.kind.cmp(&a.kind));
        self
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<KeyIntent> {
        self.resolve_with_layer(event).map(|resolution| resolution.intent)
    }

    pub fn resolve_with_layer(&self, event: &KeyEvent) -> Option<LayeredKeyMapResolution> {
        if let Some(intent) = self.reserved.resolve(event) {
            return Some(LayeredKeyMapResolution {
                layer: "runtime:reserved",
                intent,
            });
        }

        self.layers.iter().find_map(|layer| {
            layer.keymap.resolve(event).map(|intent| LayeredKeyMapResolution {
                layer: layer.name,
                intent,
            })
        })
    }
}

pub struct DefaultKeyMap;

impl DefaultKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::char('+'), KeyIntent::App("increment"))
            .bind(KeyEvent::char('-'), KeyIntent::App("decrement"))
            .bind(KeyEvent::char('q'), KeyIntent::RequestQuit)
            .bind(KeyEvent::enter(), KeyIntent::RequestSubmit)
            .bind(KeyEvent::esc(), KeyIntent::RequestEscape)
    }
}

pub struct PlatformFallbackKeyMap;

impl PlatformFallbackKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::arrow(Direction::Up), KeyIntent::Move(Direction::Up))
            .bind(
                KeyEvent::arrow(Direction::Down),
                KeyIntent::Move(Direction::Down),
            )
            .bind(
                KeyEvent::arrow(Direction::Left),
                KeyIntent::Move(Direction::Left),
            )
            .bind(
                KeyEvent::arrow(Direction::Right),
                KeyIntent::Move(Direction::Right),
            )
    }
}

pub struct EmacsTextKeyMap;

impl EmacsTextKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::ctrl('a'), KeyIntent::GoHome)
            .bind(KeyEvent::ctrl('e'), KeyIntent::GoEnd)
            .bind(KeyEvent::ctrl('b'), KeyIntent::Move(Direction::Left))
            .bind(KeyEvent::ctrl('f'), KeyIntent::Move(Direction::Right))
            .bind(KeyEvent::ctrl('d'), KeyIntent::DeleteForward)
    }
}

pub struct VimNavigationKeyMap;

impl VimNavigationKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::char('h'), KeyIntent::Move(Direction::Left))
            .bind(KeyEvent::char('j'), KeyIntent::Move(Direction::Down))
            .bind(KeyEvent::char('k'), KeyIntent::Move(Direction::Up))
            .bind(KeyEvent::char('l'), KeyIntent::Move(Direction::Right))
    }
}

pub struct ReadOnlyNavigationKeyMap;

impl ReadOnlyNavigationKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::arrow(Direction::Up), KeyIntent::Move(Direction::Up))
            .bind(
                KeyEvent::arrow(Direction::Down),
                KeyIntent::Move(Direction::Down),
            )
            .bind(
                KeyEvent {
                    key: Key::Page(Direction::Up),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::Page(Direction::Up),
            )
            .bind(
                KeyEvent {
                    key: Key::Page(Direction::Down),
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::Page(Direction::Down),
            )
            .bind(
                KeyEvent {
                    key: Key::Home,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::GoHome,
            )
            .bind(
                KeyEvent {
                    key: Key::End,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::GoEnd,
            )
    }
}

pub struct TextInputKeyMap;

impl TextInputKeyMap {
    pub fn new() -> KeyMap {
        KeyMap::new()
            .bind(KeyEvent::enter(), KeyIntent::RequestSubmit)
            .bind(KeyEvent::esc(), KeyIntent::RequestEscape)
            .bind(
                KeyEvent {
                    key: Key::Backspace,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::DeleteBackward,
            )
            .bind(
                KeyEvent {
                    key: Key::Delete,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                },
                KeyIntent::DeleteForward,
            )
            .bind(KeyEvent::arrow(Direction::Left), KeyIntent::Move(Direction::Left))
            .bind(
                KeyEvent::arrow(Direction::Right),
                KeyIntent::Move(Direction::Right),
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedInputQueue {
    inputs: VecDeque<RuntimeInput>,
    capacity: usize,
    shutdown_requested: bool,
}

impl BoundedInputQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            inputs: VecDeque::with_capacity(capacity),
            capacity,
            shutdown_requested: false,
        }
    }

    pub fn push(&mut self, input: RuntimeInput) -> Result<(), RuntimeInput> {
        if self.inputs.len() >= self.capacity {
            return Err(input);
        }
        if input == RuntimeInput::Shutdown {
            self.shutdown_requested = true;
        }
        self.inputs.push_back(input);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<RuntimeInput> {
        self.inputs.pop_front()
    }

    pub fn is_full(&self) -> bool {
        self.inputs.len() >= self.capacity
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendInputEvent {
    Key(BackendKeyEvent),
    Resize(Size),
    Tick,
    Wake,
    Shutdown,
}

impl BackendInputEvent {
    pub fn into_runtime_input(self) -> RuntimeInput {
        match self {
            Self::Key(event) => RuntimeInput::Key(event.into_key_event()),
            Self::Resize(size) => RuntimeInput::Resize(size),
            Self::Tick => RuntimeInput::Tick,
            Self::Wake => RuntimeInput::BackendWake,
            Self::Shutdown => RuntimeInput::Shutdown,
        }
    }
}

pub trait BackendEventSource {
    fn read_event(&mut self) -> Option<BackendInputEvent>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputShutdownSignal {
    requested: bool,
}

impl InputShutdownSignal {
    pub const fn new() -> Self {
        Self { requested: false }
    }

    pub fn request_shutdown(&mut self) {
        self.requested = true;
    }

    pub const fn is_shutdown_requested(&self) -> bool {
        self.requested
    }
}

impl Default for InputShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputThreadDriver<Source> {
    source: Source,
    shutdown: InputShutdownSignal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputThreadStep {
    Queued,
    QueueFull,
    SourceClosed,
    Shutdown,
}

impl<Source> InputThreadDriver<Source>
where
    Source: BackendEventSource,
{
    pub fn new(source: Source) -> Self {
        Self {
            source,
            shutdown: InputShutdownSignal::new(),
        }
    }

    pub fn shutdown_signal(&self) -> &InputShutdownSignal {
        &self.shutdown
    }

    pub fn request_shutdown(&mut self) {
        self.shutdown.request_shutdown();
    }

    pub fn step(&mut self, queue: &mut BoundedInputQueue) -> InputThreadStep {
        if self.shutdown.is_shutdown_requested() || queue.is_shutdown_requested() {
            return InputThreadStep::Shutdown;
        }

        let Some(event) = self.source.read_event() else {
            return InputThreadStep::SourceClosed;
        };
        let input = event.into_runtime_input();
        match queue.push(input) {
            Ok(()) => InputThreadStep::Queued,
            Err(_) => InputThreadStep::QueueFull,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_maps_plus_to_increment_intent() {
        assert_eq!(
            DefaultKeyMap::new().resolve(&KeyEvent::char('+')),
            Some(KeyIntent::App("increment"))
        );
    }

    #[test]
    fn keymap_maps_q_to_quit_intent() {
        assert_eq!(
            DefaultKeyMap::new().resolve(&KeyEvent::char('q')),
            Some(KeyIntent::RequestQuit)
        );
    }

    #[test]
    fn backend_key_event_converts_to_runtime_key_event_without_terminal() {
        let event = BackendKeyEvent {
            key: BackendKey::Esc,
            ctrl: true,
            alt: false,
            shift: true,
            kind: KeyEventKind::Press,
        }
        .into_key_event();

        assert_eq!(event.key, Key::Esc);
        assert!(event.modifiers.contains(KeyModifiers::CTRL));
        assert!(event.modifiers.contains(KeyModifiers::SHIFT));
        assert!(!event.modifiers.contains(KeyModifiers::ALT));
    }

    #[test]
    fn duplicate_bindings_are_detected_in_same_keymap() {
        let result = KeyMap::new()
            .try_bind(KeyEvent::char('x'), KeyIntent::App("first"))
            .unwrap()
            .try_bind(KeyEvent::char('x'), KeyIntent::App("second"));

        assert!(result.is_err());
    }

    #[test]
    fn keymap_layer_can_pass_or_handle_input() {
        let layer = KeyMapLayer::new(
            "app",
            KeyMap::new().bind(KeyEvent::char('p'), KeyIntent::App("prompt")),
        );

        assert_eq!(
            layer.resolve(&KeyEvent::char('p')),
            KeyMapResult::Handled(KeyIntent::App("prompt"))
        );
        assert_eq!(layer.resolve(&KeyEvent::char('x')), KeyMapResult::Pass);
    }

    #[test]
    fn layered_keymap_resolves_different_layer_duplicates_by_priority() {
        let layered = LayeredKeyMap::new()
            .with_layer(KeyMapLayer::with_kind(
                "mode:game",
                KeyMapLayerKind::Mode,
                KeyMap::new().bind(KeyEvent::char('q'), KeyIntent::App("cast_ultimate")),
            ))
            .with_layer(KeyMapLayer::with_kind(
                "runtime",
                KeyMapLayerKind::Runtime,
                KeyMap::new().bind(KeyEvent::char('q'), KeyIntent::RequestQuit),
            ));

        let resolution = layered.resolve_with_layer(&KeyEvent::char('q')).unwrap();

        assert_eq!(resolution.layer, "mode:game");
        assert_eq!(resolution.intent, KeyIntent::App("cast_ultimate"));
    }

    #[test]
    fn runtime_reserved_ctrl_c_is_unoverridable() {
        let layered = LayeredKeyMap::new().with_layer(KeyMapLayer::with_kind(
            "focused",
            KeyMapLayerKind::FocusedControl,
            KeyMap::new().bind(KeyEvent::ctrl('c'), KeyIntent::App("copy")),
        ));

        let resolution = layered.resolve_with_layer(&KeyEvent::ctrl('c')).unwrap();

        assert_eq!(resolution.layer, "runtime:reserved");
        assert_eq!(resolution.intent, KeyIntent::RequestQuit);
    }

    #[test]
    fn app_keymap_can_override_normal_escape() {
        let layered = LayeredKeyMap::default().with_layer(KeyMapLayer::with_kind(
            "app",
            KeyMapLayerKind::App,
            KeyMap::new().bind(KeyEvent::esc(), KeyIntent::App("close_palette")),
        ));

        assert_eq!(
            layered.resolve(&KeyEvent::esc()),
            Some(KeyIntent::App("close_palette"))
        );
    }

    #[test]
    fn focused_control_keymap_beats_app_keymap() {
        let layered = LayeredKeyMap::new()
            .with_layer(KeyMapLayer::with_kind(
                "app",
                KeyMapLayerKind::App,
                KeyMap::new().bind(KeyEvent::enter(), KeyIntent::App("submit_form")),
            ))
            .with_layer(KeyMapLayer::with_kind(
                "focused:text-input",
                KeyMapLayerKind::FocusedControl,
                TextInputKeyMap::new(),
            ));

        let resolution = layered.resolve_with_layer(&KeyEvent::enter()).unwrap();

        assert_eq!(resolution.layer, "focused:text-input");
        assert_eq!(resolution.intent, KeyIntent::RequestSubmit);
    }

    #[test]
    fn mode_keymap_can_change_q_from_quit_to_game_action() {
        let default_mode = LayeredKeyMap::default();
        let game_mode = LayeredKeyMap::new().with_layer(KeyMapLayer::with_kind(
            "mode:game",
            KeyMapLayerKind::Mode,
            KeyMap::new().bind(KeyEvent::char('q'), KeyIntent::App("cast_ultimate")),
        ));

        assert_eq!(
            default_mode.resolve(&KeyEvent::char('q')),
            Some(KeyIntent::RequestQuit)
        );
        assert_eq!(
            game_mode.resolve(&KeyEvent::char('q')),
            Some(KeyIntent::App("cast_ultimate"))
        );
    }

    #[test]
    fn text_input_resolver_interprets_editing_intents_as_control_actions() {
        struct NoAppMapping;

        impl IntentMapper<()> for NoAppMapping {
            fn map_intent(&self, _intent: KeyIntent) -> Option<KeyAction<()>> {
                None
            }
        }

        let context = IntentContext {
            focused_control: Some((HostNodeId::new(7), FocusedControlKind::TextInput)),
            ..IntentContext::default()
        };

        assert_eq!(
            NoAppMapping.resolve_intent(&context, KeyIntent::DeleteBackward),
            Some(KeyAction::Control {
                target: HostNodeId::new(7),
                action: ControlKeyAction::DeleteBackward,
            })
        );
        assert_eq!(
            NoAppMapping.resolve_intent(&context, KeyIntent::InsertText("x".to_string())),
            Some(KeyAction::Control {
                target: HostNodeId::new(7),
                action: ControlKeyAction::InsertText("x".to_string()),
            })
        );
    }

    #[test]
    fn built_in_keymap_presets_have_expected_bindings() {
        assert_eq!(
            EmacsTextKeyMap::new().resolve(&KeyEvent::ctrl('a')),
            Some(KeyIntent::GoHome)
        );
        assert_eq!(
            VimNavigationKeyMap::new().resolve(&KeyEvent::char('j')),
            Some(KeyIntent::Move(Direction::Down))
        );
        assert_eq!(
            ReadOnlyNavigationKeyMap::new().resolve(&KeyEvent::arrow(Direction::Down)),
            Some(KeyIntent::Move(Direction::Down))
        );
        assert_eq!(
            TextInputKeyMap::new().resolve(&KeyEvent::enter()),
            Some(KeyIntent::RequestSubmit)
        );
    }

    #[test]
    fn bounded_input_queue_reports_full_without_blocking() {
        let mut queue = BoundedInputQueue::new(1);

        assert!(queue.push(RuntimeInput::Tick).is_ok());
        assert_eq!(
            queue.push(RuntimeInput::BackendWake),
            Err(RuntimeInput::BackendWake)
        );
        assert!(queue.is_full());
    }

    #[test]
    fn bounded_input_queue_records_shutdown_signal() {
        let mut queue = BoundedInputQueue::new(2);

        assert!(queue.push(RuntimeInput::Shutdown).is_ok());

        assert!(queue.is_shutdown_requested());
        assert_eq!(queue.pop(), Some(RuntimeInput::Shutdown));
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

    impl BackendEventSource for VecEventSource {
        fn read_event(&mut self) -> Option<BackendInputEvent> {
            self.events.pop_front()
        }
    }

    #[test]
    fn input_thread_driver_only_normalizes_backend_events_into_queue() {
        let mut driver = InputThreadDriver::new(VecEventSource::new(VecDeque::from([
            BackendInputEvent::Key(BackendKeyEvent::char('x')),
            BackendInputEvent::Resize(Size::new(10, 2)),
        ])));
        let mut queue = BoundedInputQueue::new(4);

        assert_eq!(driver.step(&mut queue), InputThreadStep::Queued);
        assert_eq!(driver.step(&mut queue), InputThreadStep::Queued);

        assert_eq!(
            queue.pop(),
            Some(RuntimeInput::Key(KeyEvent::char('x')))
        );
        assert_eq!(queue.pop(), Some(RuntimeInput::Resize(Size::new(10, 2))));
    }

    #[test]
    fn input_thread_driver_reports_full_queue_without_blocking() {
        let mut driver = InputThreadDriver::new(VecEventSource::new(VecDeque::from([
            BackendInputEvent::Wake,
        ])));
        let mut queue = BoundedInputQueue::new(0);

        assert_eq!(driver.step(&mut queue), InputThreadStep::QueueFull);
    }

    #[test]
    fn runtime_shutdown_signal_stops_input_thread_driver() {
        let mut driver = InputThreadDriver::new(VecEventSource::new(VecDeque::from([
            BackendInputEvent::Wake,
        ])));
        let mut queue = BoundedInputQueue::new(1);

        driver.request_shutdown();

        assert_eq!(driver.step(&mut queue), InputThreadStep::Shutdown);
        assert!(driver.shutdown_signal().is_shutdown_requested());
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn queued_shutdown_event_stops_later_input_thread_steps() {
        let mut driver = InputThreadDriver::new(VecEventSource::new(VecDeque::from([
            BackendInputEvent::Shutdown,
            BackendInputEvent::Wake,
        ])));
        let mut queue = BoundedInputQueue::new(2);

        assert_eq!(driver.step(&mut queue), InputThreadStep::Queued);
        assert_eq!(driver.step(&mut queue), InputThreadStep::Shutdown);
        assert_eq!(queue.pop(), Some(RuntimeInput::Shutdown));
        assert_eq!(queue.pop(), None);
    }
}
