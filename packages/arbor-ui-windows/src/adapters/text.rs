use windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_NONE;
use windows::Win32::Graphics::DirectWrite::DWRITE_MEASURING_MODE_NATURAL;

use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::d2d::d2d_rect;
use crate::error::RenderResult;
use arbor_ui_core::view::components::{ComponentNode, Text};

impl WindowsComponentAdapter for Text {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        let brush = cx.brush(self.style().color)?;
        let rect = d2d_rect(self.rect());
        let value: Vec<u16> = self.content().encode_utf16().collect();
        let style = self.style();
        let format = cx.text_format(style.weight, style.size, self.align())?;
        // SAFETY: text UTF-16 buffer, layout rect, format, and brush all live through the draw call.
        unsafe {
            cx.target().DrawText(
                &value,
                &format,
                &rect,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
        Ok(())
    }
}
