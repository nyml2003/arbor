pub mod error;
pub mod input;
pub mod keyboard;
pub mod layout;
pub mod state;
pub mod surface;

pub use arbor_ui_core::{Point, PointerEvent, Rect, Size, ViewSnapshot};
pub use error::{AppError, AppResult};
pub use input::{InputCommand, KeyCode, Modifier};
pub use layout::KeyboardLayout;
pub use surface::{ChromeHit, KeyboardSurface, PointerUpdate};

#[derive(Debug)]
pub struct KeyDockApp {
    surface: KeyboardSurface,
}

impl KeyDockApp {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            surface: KeyboardSurface::new(KeyboardLayout::qwerty())?,
        })
    }

    pub fn resize(&mut self, size: Size) -> AppResult<()> {
        self.surface.resize(size)
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
