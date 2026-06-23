pub mod error;
pub mod history;
pub mod layout;
pub mod model;
pub mod surface;

pub use arbor_ui_core::{Point, PointerEvent, Size, ViewSnapshot};
pub use error::{AppError, AppResult};
pub use model::AppCommand;
pub use surface::{ChromeHit, ClipDockSurface, PointerUpdate};

#[derive(Debug)]
pub struct ClipDockApp {
    surface: ClipDockSurface,
}

impl ClipDockApp {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            surface: ClipDockSurface::new()?,
        })
    }

    pub fn resize(&mut self, size: Size) -> AppResult<()> {
        self.surface.resize(size)
    }

    pub fn record_clipboard_text(&mut self, text: impl Into<String>) -> AppResult<bool> {
        self.surface.record_clipboard_text(text)
    }

    pub fn handle_pointer_event(&mut self, event: PointerEvent) -> AppResult<PointerUpdate> {
        self.surface.handle_pointer_event(event)
    }

    pub fn chrome_hit_test(&self, point: Point) -> ChromeHit {
        self.surface.chrome_hit_test(point)
    }

    pub fn advance_animations(&mut self, delta_ms: f32) {
        self.surface.advance_animations(delta_ms);
    }

    pub fn has_active_animations(&self) -> bool {
        self.surface.has_active_animations()
    }

    pub fn snapshot(&self) -> ViewSnapshot {
        self.surface.snapshot()
    }
}
