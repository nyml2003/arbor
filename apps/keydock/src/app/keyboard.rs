#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModifierKind {
    Shift,
    Control,
    Alt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Backspace,
    Enter,
    Escape,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyBehavior {
    Character { normal: char, shifted: char },
    Modifier(ModifierKind),
    Action(ActionKind),
    Space,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeySpec {
    pub id: String,
    pub label: String,
    pub shifted_label: Option<String>,
    pub width_units: f32,
    pub behavior: KeyBehavior,
}

impl KeySpec {
    pub fn character(label: &str, normal: char, shifted: char) -> Self {
        Self {
            id: format!("key-{normal}"),
            label: label.to_string(),
            shifted_label: Some(shifted.to_string()),
            width_units: 1.0,
            behavior: KeyBehavior::Character { normal, shifted },
        }
    }

    pub fn action(id: &str, label: &str, width_units: f32, action: ActionKind) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            shifted_label: None,
            width_units,
            behavior: KeyBehavior::Action(action),
        }
    }

    pub fn modifier(id: &str, label: &str, width_units: f32, modifier: ModifierKind) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            shifted_label: None,
            width_units,
            behavior: KeyBehavior::Modifier(modifier),
        }
    }

    pub fn space(width_units: f32) -> Self {
        Self {
            id: "key-space".to_string(),
            label: "Space".to_string(),
            shifted_label: None,
            width_units,
            behavior: KeyBehavior::Space,
        }
    }
}
