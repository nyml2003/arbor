use windows::Win32::Graphics::Direct2D::{D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_ELLIPSE};
use windows_numerics::Vector2;

use super::WindowsComponentAdapter;
use crate::context::RenderContext;
use crate::d2d::d2d_rect;
use crate::error::RenderResult;
use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::view::components::{Button, ButtonState, ComponentNode};

impl WindowsComponentAdapter for Button {
    fn draw_windows(&self, cx: &mut RenderContext<'_, '_>) -> RenderResult<()> {
        let token = match self.state() {
            ButtonState::Normal => ColorToken::Button,
            ButtonState::Hovered => ColorToken::ButtonHovered,
            ButtonState::Pressed => ColorToken::ButtonPressed,
            ButtonState::Active => ColorToken::ButtonActive,
            ButtonState::Disabled => ColorToken::ButtonDisabled,
        };
        let brush = cx.brush(token)?;
        let rect = d2d_rect(self.rect());
        // SAFETY: rect points to initialized stack storage and brush is a live brush created for this
        // render target. Direct2D reads both during the call.
        unsafe {
            cx.target().FillRectangle(&rect, &brush);
        }
        draw_ripples(cx, self)?;
        let border = cx.brush(ColorToken::Border)?;
        // SAFETY: rect and brush are valid for the duration of the call. No stroke style is used.
        unsafe {
            cx.target().DrawRectangle(&rect, &border, 1.0, None);
        }
        cx.draw_primitive(&self.content)
    }
}

fn draw_ripples(cx: &mut RenderContext<'_, '_>, button: &Button) -> RenderResult<()> {
    if button.ripples().is_empty() {
        return Ok(());
    }

    let clip = d2d_rect(button.rect());
    // SAFETY: clip points to initialized stack storage and the clip is popped before return.
    unsafe {
        cx.target()
            .PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
    }

    for ripple in button.ripples() {
        let mut color = ripple.color.color();
        color.a *= ripple.opacity;
        let brush = cx.brush_for_color(color)?;
        let ellipse = D2D1_ELLIPSE {
            point: Vector2 {
                X: ripple.origin.x,
                Y: ripple.origin.y,
            },
            radiusX: ripple.radius,
            radiusY: ripple.radius,
        };

        // SAFETY: ellipse is an initialized stack value and brush is live for this render target.
        unsafe {
            cx.target().FillEllipse(&ellipse, &brush);
        }
    }

    // SAFETY: Balanced with PushAxisAlignedClip above in the same draw call.
    unsafe {
        cx.target().PopAxisAlignedClip();
    }

    Ok(())
}
