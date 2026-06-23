mod animation;
mod interaction;
mod view;

#[cfg(test)]
mod tests;

use animation::{advance_ripples, append_ripple, ActiveRipple};
use interaction::{activate_key, reduce_pointer_event, ActivationOutcome};

use super::error::{AppError, AppResult};
use super::input::InputCommand;
use super::keyboard::KeySpec;
use super::layout::{compute_layout, KeyboardLayout, LayoutConfig, LayoutSnapshot};
use super::state::KeyboardState;
use arbor_ui_core::event::PointerEvent;
use arbor_ui_core::geometry::{Point, Rect, Size};
use arbor_ui_core::ViewSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeHit {
    Client,
    Drag,
    Command(InputCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerUpdate {
    pub commands: Vec<InputCommand>,
    pub needs_render: bool,
}

#[derive(Debug)]
pub struct KeyboardSurface {
    pub(super) layout: KeyboardLayout,
    pub(super) config: LayoutConfig,
    pub(super) size: Size,
    pub(super) snapshot: LayoutSnapshot,
    pub(super) state: KeyboardState,
    pub(super) ripples: Vec<ActiveRipple>,
}

impl KeyboardSurface {
    pub fn new(layout: KeyboardLayout) -> AppResult<Self> {
        let config = LayoutConfig::default();
        let size = Size::new(900.0, 320.0);
        let snapshot = compute_layout(&layout, size, config)?;

        Ok(Self {
            layout,
            config,
            size,
            snapshot,
            state: KeyboardState::default(),
            ripples: Vec::new(),
        })
    }

    pub fn resize(&mut self, size: Size) -> AppResult<()> {
        self.size = size;
        self.snapshot = compute_layout(&self.layout, size, self.config)?;
        Ok(())
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> AppResult<PointerUpdate> {
        let hit_key = match event {
            PointerEvent::Move(point) | PointerEvent::Down(point) | PointerEvent::Up(point) => {
                self.button_hit_test(point).map(str::to_string)
            }
            PointerEvent::Cancel => None,
        };
        let outcome = reduce_pointer_event(&self.state, event, hit_key.as_deref());

        let mut needs_render = outcome.changed;
        self.state = outcome.next_state;
        if let Some(ripple) = outcome.ripple {
            self.ripples = append_ripple(&self.ripples, ripple);
            needs_render = true;
        }

        if let Some(key_id) = outcome.activation {
            let activation = self.activate_button(&self.state, &key_id)?;
            self.state = activation.next_state;
            return Ok(PointerUpdate {
                commands: activation.commands,
                needs_render: true,
            });
        }

        Ok(PointerUpdate {
            commands: Vec::new(),
            needs_render,
        })
    }

    pub fn chrome_hit_test(&self, point: Point) -> ChromeHit {
        if self.close_title_rect().contains(point) {
            return ChromeHit::Command(InputCommand::CloseApp);
        }

        if self.drag_title_rect().contains(point) {
            return ChromeHit::Drag;
        }

        ChromeHit::Client
    }

    pub fn snapshot(&self) -> ViewSnapshot {
        self.view_snapshot()
    }

    pub fn advance_animations(&mut self, delta_ms: f32) {
        self.ripples = advance_ripples(&self.ripples, delta_ms);
    }

    pub fn has_active_animations(&self) -> bool {
        !self.ripples.is_empty()
    }

    pub(super) fn close_title_rect(&self) -> Rect {
        Rect::new(
            self.snapshot.title_rect.right() - 72.0,
            self.snapshot.title_rect.y,
            72.0,
            self.snapshot.title_rect.height,
        )
    }

    pub(super) fn drag_title_rect(&self) -> Rect {
        let close_rect = self.close_title_rect();
        Rect::new(
            self.snapshot.title_rect.x,
            self.snapshot.title_rect.y,
            (close_rect.x - self.snapshot.title_rect.x - self.config.key_gap).max(0.0),
            self.snapshot.title_rect.height,
        )
    }

    fn button_hit_test(&self, point: Point) -> Option<&str> {
        if self.close_title_rect().contains(point) {
            return Some("key-close-title");
        }

        self.snapshot
            .keys
            .iter()
            .find(|key| key.rect.contains(point))
            .map(|key| key.spec.id.as_str())
    }

    fn key_by_id(&self, key_id: &str) -> AppResult<&KeySpec> {
        self.snapshot
            .keys
            .iter()
            .find(|key| key.spec.id == key_id)
            .map(|key| &key.spec)
            .ok_or_else(|| AppError::UnknownKey(key_id.to_string()))
    }

    fn activate_button(&self, state: &KeyboardState, key_id: &str) -> AppResult<ActivationOutcome> {
        if key_id == "key-close-title" {
            return Ok(ActivationOutcome {
                next_state: state.clone(),
                commands: vec![InputCommand::CloseApp],
            });
        }

        let spec = self.key_by_id(key_id)?;
        Ok(activate_key(state, spec))
    }
}
