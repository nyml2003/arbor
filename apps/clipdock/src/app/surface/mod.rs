mod animation;
mod interaction;
mod view;

#[cfg(test)]
mod tests;

use animation::{advance_ripples, append_ripple, ActiveRipple};
use interaction::{activate_target, reduce_pointer_event};

use super::error::{AppError, AppResult};
use super::history::ClipboardHistory;
use super::layout::{compute_layout, LayoutConfig, LayoutSnapshot};
use super::model::AppCommand;
use arbor_ui_core::event::PointerEvent;
use arbor_ui_core::geometry::{Point, Rect, Size};
use arbor_ui_core::ViewSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChromeHit {
    Client,
    Drag,
    Command(AppCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerUpdate {
    pub commands: Vec<AppCommand>,
    pub needs_render: bool,
}

#[derive(Debug)]
pub struct ClipDockSurface {
    history: ClipboardHistory,
    config: LayoutConfig,
    size: Size,
    snapshot: LayoutSnapshot,
    hovered_target: Option<String>,
    pressed_target: Option<String>,
    ripples: Vec<ActiveRipple>,
}

impl ClipDockSurface {
    pub fn new() -> AppResult<Self> {
        let history = ClipboardHistory::default();
        let config = LayoutConfig::default();
        let size = Size::new(420.0, 520.0);
        let snapshot = compute_layout(&history, size, config);

        Ok(Self {
            history,
            config,
            size,
            snapshot,
            hovered_target: None,
            pressed_target: None,
            ripples: Vec::new(),
        })
    }

    pub fn resize(&mut self, size: Size) -> AppResult<()> {
        self.size = size;
        self.snapshot = compute_layout(&self.history, size, self.config);
        Ok(())
    }

    pub fn record_clipboard_text(&mut self, text: impl Into<String>) -> AppResult<bool> {
        let update = self.history.push_text(text);
        if update.changed {
            self.history = update.history;
            self.snapshot = compute_layout(&self.history, self.size, self.config);
        }
        Ok(update.changed)
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> AppResult<PointerUpdate> {
        let hit_target = match event {
            PointerEvent::Move(point) | PointerEvent::Down(point) | PointerEvent::Up(point) => {
                self.button_hit_test(point)
            }
            PointerEvent::Cancel => None,
        };
        let outcome = reduce_pointer_event(
            self.hovered_target.as_deref(),
            self.pressed_target.as_deref(),
            event,
            hit_target.as_deref(),
        );

        let mut needs_render = outcome.changed;
        self.hovered_target = outcome.hovered_target;
        self.pressed_target = outcome.pressed_target;
        if let Some(ripple) = outcome.ripple {
            self.ripples = append_ripple(&self.ripples, ripple);
            needs_render = true;
        }

        let commands = if let Some(target_id) = outcome.activation {
            needs_render = true;
            activate_target(&target_id, &self.history)?
        } else {
            Vec::new()
        };

        Ok(PointerUpdate {
            commands,
            needs_render,
        })
    }

    pub fn chrome_hit_test(&self, point: Point) -> ChromeHit {
        if self.snapshot.close_rect.contains(point) {
            return ChromeHit::Command(AppCommand::CloseApp);
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

    pub(super) fn drag_title_rect(&self) -> Rect {
        Rect::new(
            self.snapshot.title_rect.x,
            self.snapshot.title_rect.y,
            (self.snapshot.close_rect.x - self.snapshot.title_rect.x - self.config.gap).max(0.0),
            self.snapshot.title_rect.height,
        )
    }

    fn button_hit_test(&self, point: Point) -> Option<String> {
        if self.snapshot.close_rect.contains(point) {
            return Some(close_target_id().to_string());
        }

        self.snapshot
            .items
            .iter()
            .find(|item| item.rect.contains(point))
            .map(|item| item.item.id.clone())
    }
}

pub(super) fn close_target_id() -> &'static str {
    "clip-close-title"
}

pub(super) fn missing_item(id: &str) -> AppError {
    AppError::MissingHistoryItem(id.to_string())
}
