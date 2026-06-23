#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    Shift,
    Control,
    Alt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Backspace,
    Enter,
    Escape,
    Space,
    Character(char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputCommand {
    KeyTap(KeyCode),
    ModifiedKeyTap {
        modifiers: Vec<Modifier>,
        key: KeyCode,
    },
    Text(char),
    CloseApp,
}
