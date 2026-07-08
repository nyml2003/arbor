use crate::{HostNodeId, Size};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
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

    pub const fn empty() -> Self {
        Self { bits: 0 }
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
    pub keymap: KeyMap,
}

impl KeyMapLayer {
    pub fn new(name: &'static str, keymap: KeyMap) -> Self {
        Self { name, keymap }
    }

    pub fn resolve(&self, event: &KeyEvent) -> KeyMapResult {
        self.keymap
            .resolve(event)
            .map(KeyMapResult::Handled)
            .unwrap_or(KeyMapResult::Pass)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keymap_maps_plus_to_increment_intent() {
        assert_eq!(
            KeyMap::default().resolve(&KeyEvent::char('+')),
            Some(KeyIntent::App("increment"))
        );
    }

    #[test]
    fn keymap_maps_q_to_quit_intent() {
        assert_eq!(
            KeyMap::default().resolve(&KeyEvent::char('q')),
            Some(KeyIntent::RequestQuit)
        );
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
}
