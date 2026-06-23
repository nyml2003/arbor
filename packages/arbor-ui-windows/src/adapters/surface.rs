use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::d2d::{d2d_rect, d2d_rounded_rect};
use crate::error::RenderResult;
use arbor_ui_core::view::components::{ComponentNode, Surface};

impl WindowsComponentAdapter for Surface {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        let brush = cx.brush(self.background())?;
        let rect = d2d_rect(self.rect());
        if self.radius() > 0.0 {
            let rounded = d2d_rounded_rect(self.rect(), self.radius());
            // SAFETY: rounded points to initialized stack storage and brush is a live brush created
            // for this render target. Direct2D reads both during the call.
            unsafe {
                cx.target().FillRoundedRectangle(&rounded, &brush);
            }
        } else {
            // SAFETY: rect points to initialized stack storage and brush is a live brush created for
            // this render target. Direct2D reads both during the call.
            unsafe {
                cx.target().FillRectangle(&rect, &brush);
            }
        }
        if let Some(border) = self.border() {
            let border_brush = cx.brush(border.color)?;
            if self.radius() > 0.0 {
                let rounded = d2d_rounded_rect(self.rect(), self.radius());
                // SAFETY: rounded and brush are valid for the duration of the call. No stroke style
                // is used.
                unsafe {
                    cx.target()
                        .DrawRoundedRectangle(&rounded, &border_brush, border.width, None);
                }
            } else {
                // SAFETY: rect and brush are valid for the duration of the call. No stroke style is
                // used.
                unsafe {
                    cx.target()
                        .DrawRectangle(&rect, &border_brush, border.width, None);
                }
            }
        }
        for child in self.children() {
            cx.draw_primitive(child)?;
        }
        Ok(())
    }
}
