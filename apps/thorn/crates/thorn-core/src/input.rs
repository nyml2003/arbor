use crate::Size;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
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
    RequestQuit,
    App(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction<Action> {
    RuntimeQuit,
    App(Action),
}

pub trait IntentMapper<Action> {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<Action>>;
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
            .bind(KeyEvent::ctrl('c'), KeyIntent::RequestQuit)
    }
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn bind(mut self, event: KeyEvent, intent: KeyIntent) -> Self {
        self.bindings.push((event, intent));
        self
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<KeyIntent> {
        self.bindings
            .iter()
            .find_map(|(bound_event, intent)| (bound_event == event).then(|| intent.clone()))
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
}
