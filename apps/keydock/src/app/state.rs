use super::input::Modifier;
use super::keyboard::ModifierKind;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyboardState {
    pub shift_latched: bool,
    pub ctrl_active: bool,
    pub alt_active: bool,
    pub hovered_key: Option<String>,
    pub pressed_key: Option<String>,
}

impl KeyboardState {
    pub fn is_modifier_active(&self, modifier: ModifierKind) -> bool {
        match modifier {
            ModifierKind::Shift => self.shift_latched,
            ModifierKind::Control => self.ctrl_active,
            ModifierKind::Alt => self.alt_active,
        }
    }

    pub fn with_toggled_modifier(&self, modifier: ModifierKind) -> Self {
        let mut next = self.clone();
        match modifier {
            ModifierKind::Shift => next.shift_latched = !next.shift_latched,
            ModifierKind::Control => next.ctrl_active = !next.ctrl_active,
            ModifierKind::Alt => next.alt_active = !next.alt_active,
        }
        next
    }

    pub fn active_non_shift_modifiers(&self) -> Vec<Modifier> {
        let mut modifiers = Vec::new();
        if self.ctrl_active {
            modifiers.push(Modifier::Control);
        }
        if self.alt_active {
            modifiers.push(Modifier::Alt);
        }
        modifiers
    }

    pub fn without_transient_modifiers(&self) -> Self {
        Self {
            shift_latched: false,
            ctrl_active: false,
            alt_active: false,
            hovered_key: self.hovered_key.clone(),
            pressed_key: self.pressed_key.clone(),
        }
    }
}
