use std::collections::HashMap;

use windows::Win32::Graphics::Direct2D::{ID2D1HwndRenderTarget, ID2D1SolidColorBrush};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, IDWriteTextFormat, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
};

use super::d2d::d2d_color;
use super::resources::create_text_format;
use crate::error::{RenderResult, WindowsResultExt};
use arbor_ui_core::theme::Color;
use arbor_ui_core::view::components::{Align, TextWeight};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ColorKey {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

impl From<Color> for ColorKey {
    fn from(color: Color) -> Self {
        Self {
            r: color.r.to_bits(),
            g: color.g.to_bits(),
            b: color.b.to_bits(),
            a: color.a.to_bits(),
        }
    }
}

#[derive(Default)]
pub(super) struct BrushCache {
    brushes: HashMap<ColorKey, ID2D1SolidColorBrush>,
}

impl BrushCache {
    pub(super) fn clear(&mut self) {
        self.brushes.clear();
    }

    pub(super) fn brush_for_color(
        &mut self,
        target: &ID2D1HwndRenderTarget,
        color: Color,
    ) -> RenderResult<ID2D1SolidColorBrush> {
        let key = ColorKey::from(color);
        if let Some(brush) = self.brushes.get(&key) {
            return Ok(brush.clone());
        }

        let d2d_color = d2d_color(color);
        // SAFETY: color points to initialized stack storage and the returned brush is owned by the
        // cache through windows-rs COM reference counting.
        let brush = unsafe {
            target
                .CreateSolidColorBrush(&d2d_color, None)
                .context("create solid color brush")?
        };
        self.brushes.insert(key, brush.clone());
        Ok(brush)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TextFormatKey {
    weight: TextWeight,
    size: u32,
    align: Align,
}

impl TextFormatKey {
    pub(super) fn new(weight: TextWeight, size: f32, align: Align) -> Self {
        Self {
            weight,
            size: size.to_bits(),
            align,
        }
    }
}

#[derive(Default)]
pub(super) struct TextFormatCache {
    formats: HashMap<TextFormatKey, IDWriteTextFormat>,
}

impl TextFormatCache {
    pub(super) fn format_for_style(
        &mut self,
        factory: &IDWriteFactory,
        weight: TextWeight,
        size: f32,
        align: Align,
    ) -> RenderResult<IDWriteTextFormat> {
        let key = TextFormatKey::new(weight, size, align);
        if let Some(format) = self.formats.get(&key) {
            return Ok(format.clone());
        }

        let format = create_text_format(factory, weight, size)?;
        let alignment = match align {
            Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
            Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
        };

        // SAFETY: The text format is newly created and owned by this cache; these setters only
        // mutate DirectWrite format properties before it is shared for drawing.
        unsafe {
            format
                .SetTextAlignment(alignment)
                .context("set text alignment")?;
            format
                .SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)
                .context("set paragraph alignment")?;
        }

        self.formats.insert(key, format.clone());
        Ok(format)
    }
}
