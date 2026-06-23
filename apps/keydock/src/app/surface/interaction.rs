use super::animation::RippleStart;
use crate::app::input::{InputCommand, KeyCode, Modifier};
use crate::app::keyboard::{ActionKind, KeyBehavior, KeySpec};
use crate::app::state::KeyboardState;
use arbor_ui_core::event::PointerEvent;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PointerOutcome {
    pub(super) next_state: KeyboardState,
    pub(super) ripple: Option<RippleStart>,
    pub(super) activation: Option<String>,
    pub(super) changed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ActivationOutcome {
    pub(super) next_state: KeyboardState,
    pub(super) commands: Vec<InputCommand>,
}

pub(super) fn reduce_pointer_event(
    state: &KeyboardState,
    event: PointerEvent,
    hit_key: Option<&str>,
) -> PointerOutcome {
    match event {
        PointerEvent::Move(_) => PointerOutcome {
            next_state: KeyboardState {
                hovered_key: hit_key.map(str::to_string),
                ..state.clone()
            },
            ripple: None,
            activation: None,
            changed: state.hovered_key.as_deref() != hit_key,
        },
        PointerEvent::Down(point) => {
            let key_id = hit_key.map(str::to_string);
            let changed = state.pressed_key != key_id || state.hovered_key != key_id;
            PointerOutcome {
                next_state: KeyboardState {
                    pressed_key: key_id.clone(),
                    hovered_key: key_id.clone(),
                    ..state.clone()
                },
                ripple: key_id.map(|target_id| RippleStart {
                    target_id,
                    origin: point,
                }),
                activation: None,
                changed,
            }
        }
        PointerEvent::Up(_) => {
            let released_key = hit_key.map(str::to_string);
            let changed = state.pressed_key.is_some()
                || state.hovered_key.as_deref() != released_key.as_deref();
            let activation = match (state.pressed_key.as_deref(), released_key.as_deref()) {
                (Some(pressed), Some(released)) if pressed == released => {
                    Some(released.to_string())
                }
                _ => None,
            };

            PointerOutcome {
                next_state: KeyboardState {
                    pressed_key: None,
                    hovered_key: released_key,
                    ..state.clone()
                },
                ripple: None,
                activation,
                changed,
            }
        }
        PointerEvent::Cancel => PointerOutcome {
            next_state: KeyboardState {
                pressed_key: None,
                hovered_key: None,
                ..state.clone()
            },
            ripple: None,
            activation: None,
            changed: state.pressed_key.is_some() || state.hovered_key.is_some(),
        },
    }
}

pub(super) fn activate_key(state: &KeyboardState, spec: &KeySpec) -> ActivationOutcome {
    match &spec.behavior {
        KeyBehavior::Modifier(modifier) => ActivationOutcome {
            next_state: state.with_toggled_modifier(*modifier),
            commands: Vec::new(),
        },
        KeyBehavior::Action(ActionKind::Close) => ActivationOutcome {
            next_state: state.clone(),
            commands: vec![InputCommand::CloseApp],
        },
        KeyBehavior::Action(action) => {
            let key_code = match action {
                ActionKind::Backspace => KeyCode::Backspace,
                ActionKind::Enter => KeyCode::Enter,
                ActionKind::Escape => KeyCode::Escape,
                ActionKind::Close => unreachable!("close handled above"),
            };
            let (next_state, command) = command_with_active_modifiers(state, key_code);
            ActivationOutcome {
                next_state,
                commands: vec![command],
            }
        }
        KeyBehavior::Space => {
            let (next_state, command) = command_with_active_modifiers(state, KeyCode::Space);
            ActivationOutcome {
                next_state,
                commands: vec![command],
            }
        }
        KeyBehavior::Character { normal, shifted } => {
            let output = if state.shift_latched {
                *shifted
            } else {
                *normal
            };
            if state.ctrl_active || state.alt_active {
                let (next_state, command) =
                    command_with_active_modifiers(state, KeyCode::Character(*normal));
                ActivationOutcome {
                    next_state,
                    commands: vec![command],
                }
            } else {
                ActivationOutcome {
                    next_state: state.without_transient_modifiers(),
                    commands: vec![InputCommand::Text(output)],
                }
            }
        }
    }
}

fn command_with_active_modifiers(
    state: &KeyboardState,
    key: KeyCode,
) -> (KeyboardState, InputCommand) {
    let mut modifiers = state.active_non_shift_modifiers();
    if state.shift_latched {
        modifiers.insert(0, Modifier::Shift);
    }
    let command = if modifiers.is_empty() {
        InputCommand::KeyTap(key)
    } else {
        InputCommand::ModifiedKeyTap { modifiers, key }
    };

    (state.without_transient_modifiers(), command)
}
