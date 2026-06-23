use arbor_ui_core::event::PointerEvent;

use super::animation::RippleStart;
use super::{close_target_id, missing_item};
use crate::app::error::AppResult;
use crate::app::history::ClipboardHistory;
use crate::app::model::AppCommand;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PointerOutcome {
    pub(super) hovered_target: Option<String>,
    pub(super) pressed_target: Option<String>,
    pub(super) ripple: Option<RippleStart>,
    pub(super) activation: Option<String>,
    pub(super) changed: bool,
}

pub(super) fn reduce_pointer_event(
    hovered_target: Option<&str>,
    pressed_target: Option<&str>,
    event: PointerEvent,
    hit_target: Option<&str>,
) -> PointerOutcome {
    match event {
        PointerEvent::Move(_) => PointerOutcome {
            hovered_target: hit_target.map(str::to_string),
            pressed_target: pressed_target.map(str::to_string),
            ripple: None,
            activation: None,
            changed: hovered_target != hit_target,
        },
        PointerEvent::Down(point) => {
            let target = hit_target.map(str::to_string);
            PointerOutcome {
                hovered_target: target.clone(),
                pressed_target: target.clone(),
                ripple: target.map(|target_id| RippleStart {
                    target_id,
                    origin: point,
                }),
                activation: None,
                changed: pressed_target != hit_target || hovered_target != hit_target,
            }
        }
        PointerEvent::Up(_) => {
            let activation = match (pressed_target, hit_target) {
                (Some(pressed), Some(released)) if pressed == released => {
                    Some(released.to_string())
                }
                _ => None,
            };
            PointerOutcome {
                hovered_target: hit_target.map(str::to_string),
                pressed_target: None,
                ripple: None,
                activation,
                changed: pressed_target.is_some() || hovered_target != hit_target,
            }
        }
        PointerEvent::Cancel => PointerOutcome {
            hovered_target: None,
            pressed_target: None,
            ripple: None,
            activation: None,
            changed: hovered_target.is_some() || pressed_target.is_some(),
        },
    }
}

pub(super) fn activate_target(
    target_id: &str,
    history: &ClipboardHistory,
) -> AppResult<Vec<AppCommand>> {
    if target_id == close_target_id() {
        return Ok(vec![AppCommand::CloseApp]);
    }

    let text = history
        .item_text(target_id)
        .ok_or_else(|| missing_item(target_id))?;
    Ok(vec![AppCommand::PasteText(text.to_string())])
}
