use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::error::RenderResult;
use arbor_ui_core::view::components::Row;

impl WindowsComponentAdapter for Row {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        for child in self.children() {
            cx.draw_primitive(child)?;
        }
        Ok(())
    }
}
