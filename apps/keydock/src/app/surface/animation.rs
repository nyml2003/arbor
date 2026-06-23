use arbor_ui_core::geometry::{Point, Rect};
use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::view::components::{self as c, RippleVisual};

pub(super) const RIPPLE_DURATION_MS: f32 = 220.0;
const MAX_RIPPLES_PER_TARGET: usize = 2;

#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct ActiveRipple {
    pub(in crate::app) target_id: String,
    pub(in crate::app) origin: Point,
    pub(in crate::app) age_ms: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::app) struct RippleStart {
    pub(in crate::app) target_id: String,
    pub(in crate::app) origin: Point,
}

pub(super) fn append_ripple(ripples: &[ActiveRipple], start: RippleStart) -> Vec<ActiveRipple> {
    let mut next = ripples.to_vec();
    next.push(ActiveRipple {
        target_id: start.target_id.clone(),
        origin: start.origin,
        age_ms: 0.0,
    });

    let mut target_count = next
        .iter()
        .filter(|ripple| ripple.target_id == start.target_id)
        .count();

    next.into_iter()
        .filter(|ripple| {
            if ripple.target_id == start.target_id && target_count > MAX_RIPPLES_PER_TARGET {
                target_count -= 1;
                false
            } else {
                true
            }
        })
        .collect()
}

pub(super) fn advance_ripples(ripples: &[ActiveRipple], delta_ms: f32) -> Vec<ActiveRipple> {
    let delta_ms = delta_ms.max(0.0);
    ripples
        .iter()
        .filter_map(|ripple| {
            let next = ActiveRipple {
                age_ms: ripple.age_ms + delta_ms,
                ..ripple.clone()
            };
            (next.age_ms < RIPPLE_DURATION_MS).then_some(next)
        })
        .collect()
}

pub(super) fn ripple_visuals_for_button(
    ripples: &[ActiveRipple],
    key_id: &str,
    rect: Rect,
) -> Vec<RippleVisual> {
    ripples
        .iter()
        .filter(|ripple| ripple.target_id == key_id)
        .map(|ripple| {
            let progress = (ripple.age_ms / RIPPLE_DURATION_MS).clamp(0.0, 1.0);
            let eased = ease_out_cubic(progress);
            c::ripple(
                ripple.origin,
                ripple_radius(rect) * eased,
                0.22 * (1.0 - progress),
                ColorToken::Ripple,
            )
        })
        .collect()
}

fn ease_out_cubic(progress: f32) -> f32 {
    1.0 - (1.0 - progress).powi(3)
}

fn ripple_radius(rect: Rect) -> f32 {
    (rect.width * rect.width + rect.height * rect.height).sqrt()
}
