use crate::layout::Size;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeInput {
    Key(KeyEvent),
    Resize(Size),
    Tick,
}

impl RuntimeInput {
    pub const fn is_default_exit(self) -> bool {
        match self {
            Self::Key(event) => event.is_default_exit(),
            Self::Resize(_) | Self::Tick => false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: KeyModifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
        }
    }

    pub const fn is_default_exit(self) -> bool {
        self.kind.is_press() && matches!(self.key, Key::Escape | Key::Char('q') | Key::Char('Q'))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Escape,
    Backspace,
    Tab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct KeyModifiers {
    bits: u8,
}

impl KeyModifiers {
    pub const NONE: Self = Self { bits: 0 };
    pub const SHIFT: Self = Self { bits: 0b001 };
    pub const CTRL: Self = Self { bits: 0b010 };
    pub const ALT: Self = Self { bits: 0b100 };

    pub const fn empty() -> Self {
        Self::NONE
    }

    pub const fn contains(self, modifier: Self) -> bool {
        self.bits & modifier.bits == modifier.bits
    }

    pub const fn union(self, modifier: Self) -> Self {
        Self {
            bits: self.bits | modifier.bits,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

impl KeyEventKind {
    pub const fn is_press(self) -> bool {
        matches!(self, Self::Press)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_defaults_to_press_without_modifiers() {
        assert_eq!(
            KeyEvent::new(Key::Char('x')),
            KeyEvent {
                key: Key::Char('x'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }
        );
    }

    #[test]
    fn q_and_escape_are_default_exit_inputs() {
        assert!(RuntimeInput::Key(KeyEvent::new(Key::Char('q'))).is_default_exit());
        assert!(RuntimeInput::Key(KeyEvent::new(Key::Char('Q'))).is_default_exit());
        assert!(RuntimeInput::Key(KeyEvent::new(Key::Escape)).is_default_exit());
        assert!(!RuntimeInput::Key(KeyEvent::new(Key::Enter)).is_default_exit());
    }

    #[test]
    fn key_modifiers_can_be_combined_and_queried() {
        let modifiers = KeyModifiers::SHIFT.union(KeyModifiers::CTRL);

        assert!(modifiers.contains(KeyModifiers::SHIFT));
        assert!(modifiers.contains(KeyModifiers::CTRL));
        assert!(!modifiers.contains(KeyModifiers::ALT));
        assert!(!modifiers.is_empty());
    }
}
