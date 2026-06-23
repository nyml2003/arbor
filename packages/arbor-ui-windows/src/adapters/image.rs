use windows_numerics::Vector2;

use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::error::RenderResult;
use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::view::components::{ComponentNode, Image};

impl WindowsComponentAdapter for Image {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        if self.id() != "close-icon" {
            return Ok(());
        }

        let mut color = self.tint().unwrap_or(ColorToken::TextPrimary).color();
        color.a *= self.opacity().clamp(0.0, 1.0);
        let brush = cx.brush_for_color(color)?;
        let rect = self.rect();
        // SAFETY: brush is live for this render target. Vector points are plain value types copied
        // into Direct2D during the call.
        unsafe {
            cx.target().DrawLine(
                Vector2 {
                    X: rect.x,
                    Y: rect.y,
                },
                Vector2 {
                    X: rect.right(),
                    Y: rect.bottom(),
                },
                &brush,
                1.6,
                None,
            );
            cx.target().DrawLine(
                Vector2 {
                    X: rect.right(),
                    Y: rect.y,
                },
                Vector2 {
                    X: rect.x,
                    Y: rect.bottom(),
                },
                &brush,
                1.6,
                None,
            );
        }
        Ok(())
    }
}
