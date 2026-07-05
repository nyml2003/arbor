// Input types — Key, KeyEvent, Modifiers, InputReader trait, KeyHandleResult.

/// Keyboard key variants.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,
    F(u8), // F1-F12
}

/// Modifier key flags.
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Whether a key was pressed or released. IME composition on some
/// platforms produces Release/Repeat events for intermediate characters;
/// the application layer should typically only handle Press.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyEventKind {
    Press,
    Repeat,
    Release,
}

/// A keyboard event with key and modifier state.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
    pub kind: KeyEventKind,
}

impl KeyEvent {
    pub fn char(c: char) -> Self {
        Self {
            key: Key::Char(c),
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }
    }

    pub fn with_ctrl(mut self, ctrl: bool) -> Self {
        self.modifiers.ctrl = ctrl;
        self
    }

    pub fn with_alt(mut self, alt: bool) -> Self {
        self.modifiers.alt = alt;
        self
    }
}

/// Result of key event handling.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum KeyHandleResult {
    /// Event was consumed; stop bubbling.
    Handled,
    /// Event was not handled; bubble up to parent.
    Bubble,
}

/// Input reader trait — implemented by infra adapters.
pub trait InputReader: Send {
    /// Non-blocking poll: returns all queued events.
    fn poll(&self) -> Vec<KeyEvent>;

    /// Blocking poll with timeout. Returns empty vec on timeout.
    fn poll_timeout(&self, timeout: std::time::Duration) -> Vec<KeyEvent>;

    /// Blocking read: waits for the next event.
    fn read_blocking(&self) -> KeyEvent;

    /// Send shutdown signal to wake the blocking reader thread.
    fn shutdown(&self);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_event_char_default_mods() {
        let ev = KeyEvent::char('a');
        assert_eq!(ev.key, Key::Char('a'));
        assert!(!ev.modifiers.ctrl);
    }
}
