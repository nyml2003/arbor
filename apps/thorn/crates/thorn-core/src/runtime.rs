use crate::layout::Size;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap<Action> {
    bindings: Vec<KeyBinding<Action>>,
}

impl<Action> KeyMap<Action> {
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl<Action: Clone> KeyMap<Action> {
    pub fn bind(mut self, key: Key, action: Action) -> Self {
        self.bindings.push(KeyBinding {
            key,
            modifiers: KeyModifiers::empty(),
            action,
        });
        self
    }

    pub fn bind_modified(mut self, key: Key, modifiers: KeyModifiers, action: Action) -> Self {
        self.bindings.push(KeyBinding {
            key,
            modifiers,
            action,
        });
        self
    }

    pub fn action_for(&self, event: &KeyEvent) -> Option<Action> {
        if !event.kind.is_press() {
            return None;
        }

        self.bindings
            .iter()
            .find(|binding| binding.matches(event))
            .map(|binding| binding.action.clone())
    }
}

impl<Action> Default for KeyMap<Action> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct KeyBinding<Action> {
    key: Key,
    modifiers: KeyModifiers,
    action: Action,
}

impl<Action> KeyBinding<Action> {
    fn matches(&self, event: &KeyEvent) -> bool {
        self.key == event.key && self.modifiers == event.modifiers
    }
}

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
        if !self.kind.is_press() {
            return false;
        }

        match self.key {
            Key::Escape => true,
            Key::Char('c') | Key::Char('C') | Key::Char('q') | Key::Char('Q') => {
                self.modifiers.contains(KeyModifiers::CTRL)
            }
            _ => false,
        }
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
    Insert,
    Delete,
    F(u8),
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
    fn escape_ctrl_c_and_ctrl_q_are_default_exit_inputs() {
        assert!(RuntimeInput::Key(KeyEvent::new(Key::Escape)).is_default_exit());
        assert!(RuntimeInput::Key(KeyEvent {
            key: Key::Char('c'),
            modifiers: KeyModifiers::CTRL,
            kind: KeyEventKind::Press,
        })
        .is_default_exit());
        assert!(RuntimeInput::Key(KeyEvent {
            key: Key::Char('q'),
            modifiers: KeyModifiers::CTRL,
            kind: KeyEventKind::Press,
        })
        .is_default_exit());
        assert!(!RuntimeInput::Key(KeyEvent::new(Key::Char('q'))).is_default_exit());
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

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Action {
        Increment,
        Submit,
    }

    #[test]
    fn keymap_maps_key_event_to_action() {
        let keymap = KeyMap::new()
            .bind(Key::Char('+'), Action::Increment)
            .bind_modified(Key::Enter, KeyModifiers::CTRL, Action::Submit);

        assert_eq!(
            keymap.action_for(&KeyEvent::new(Key::Char('+'))),
            Some(Action::Increment)
        );
        assert_eq!(
            keymap.action_for(&KeyEvent {
                key: Key::Enter,
                modifiers: KeyModifiers::CTRL,
                kind: KeyEventKind::Press,
            }),
            Some(Action::Submit)
        );
        assert_eq!(keymap.action_for(&KeyEvent::new(Key::Enter)), None);
    }

    #[test]
    fn keymap_ignores_release_events() {
        let keymap = KeyMap::new().bind(Key::Char('+'), Action::Increment);

        assert_eq!(
            keymap.action_for(&KeyEvent {
                key: Key::Char('+'),
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Release,
            }),
            None
        );
    }
}
