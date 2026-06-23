mod button;
mod image;
mod primitive;
mod row;
mod surface;
mod text;

use super::context::RenderContext;
use crate::error::RenderResult;

pub(super) trait WindowsComponentAdapter {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()>;
}
