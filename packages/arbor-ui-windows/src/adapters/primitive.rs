use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::error::RenderResult;
use arbor_ui_core::view::components::Primitive;

impl WindowsComponentAdapter for Primitive {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        match self {
            Primitive::Surface(surface) => surface.draw_windows(cx),
            Primitive::Row(row) => row.draw_windows(cx),
            Primitive::Button(button) => button.draw_windows(cx),
            Primitive::Text(text) => text.draw_windows(cx),
            Primitive::Image(image) => image.draw_windows(cx),
        }
    }
}
