use windows::Win32::Graphics::Direct2D::{ID2D1HwndRenderTarget, ID2D1SolidColorBrush};
use windows::Win32::Graphics::DirectWrite::{IDWriteFactory, IDWriteTextFormat};

use super::adapters::WindowsComponentAdapter;
use super::cache::{BrushCache, TextFormatCache};
use crate::error::RenderResult;
use arbor_ui_core::theme::{Color, ColorToken};
use arbor_ui_core::view::components::{Align, Primitive, TextWeight};

pub(super) struct RenderContext<'a, 'cache> {
    target: &'a ID2D1HwndRenderTarget,
    dwrite_factory: &'a IDWriteFactory,
    brush_cache: &'cache mut BrushCache,
    text_format_cache: &'cache mut TextFormatCache,
}

impl<'a, 'cache> RenderContext<'a, 'cache> {
    pub(super) fn new(
        target: &'a ID2D1HwndRenderTarget,
        brush_cache: &'cache mut BrushCache,
        dwrite_factory: &'a IDWriteFactory,
        text_format_cache: &'cache mut TextFormatCache,
    ) -> Self {
        Self {
            target,
            dwrite_factory,
            brush_cache,
            text_format_cache,
        }
    }

    pub(super) fn target(&self) -> &ID2D1HwndRenderTarget {
        self.target
    }

    pub(super) fn draw_primitive(&mut self, primitive: &Primitive) -> RenderResult<()> {
        primitive.draw_windows(self)
    }

    pub(super) fn brush(&mut self, token: ColorToken) -> RenderResult<ID2D1SolidColorBrush> {
        self.brush_for_color(token.color())
    }

    pub(super) fn brush_for_color(&mut self, color: Color) -> RenderResult<ID2D1SolidColorBrush> {
        self.brush_cache.brush_for_color(self.target, color)
    }

    pub(super) fn text_format(
        &mut self,
        weight: TextWeight,
        size: f32,
        align: Align,
    ) -> RenderResult<IDWriteTextFormat> {
        self.text_format_cache
            .format_for_style(self.dwrite_factory, weight, size, align)
    }
}
