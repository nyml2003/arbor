use windows::core::w;
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Factory, D2D1_FACTORY_TYPE_SINGLE_THREADED,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD,
};

use crate::error::{RenderResult, WindowsResultExt};
use arbor_ui_core::view::components::TextWeight;

pub(super) fn create_d2d_factory() -> RenderResult<ID2D1Factory> {
    // SAFETY: D2D1CreateFactory initializes a COM factory for this process. No custom options pointer
    // is supplied, and windows-rs validates the returned interface.
    unsafe {
        D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).context("create d2d factory")
    }
}

pub(super) fn create_dwrite_factory() -> RenderResult<IDWriteFactory> {
    // SAFETY: DWriteCreateFactory returns a shared DirectWrite factory. windows-rs validates the
    // requested interface type.
    unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).context("create dwrite factory") }
}

pub(super) fn create_text_format(
    factory: &IDWriteFactory,
    weight: TextWeight,
    size: f32,
) -> RenderResult<IDWriteTextFormat> {
    let font_weight = match weight {
        TextWeight::Regular => DWRITE_FONT_WEIGHT_NORMAL,
        TextWeight::Semibold => DWRITE_FONT_WEIGHT_SEMI_BOLD,
    };
    // SAFETY: Font family and locale are static null-terminated UTF-16 strings. Font collection is
    // None, which means the system font collection.
    unsafe {
        factory
            .CreateTextFormat(
                w!("Segoe UI"),
                None,
                font_weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("en-us"),
            )
            .context("create dwrite text format")
    }
}
