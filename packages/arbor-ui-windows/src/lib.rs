mod adapters;
mod cache;
mod context;
mod d2d;
mod error;
mod resources;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_UNKNOWN, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Factory, ID2D1HwndRenderTarget, D2D1_FEATURE_LEVEL_DEFAULT,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
};
use windows::Win32::Graphics::DirectWrite::IDWriteFactory;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_UNKNOWN;

use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::ViewSnapshot;

use adapters::WindowsComponentAdapter;
use cache::{BrushCache, TextFormatCache};
use context::RenderContext;
use d2d::d2d_color;
use error::WindowsResultExt;
pub use error::{RenderError, RenderResult};
use resources::{create_d2d_factory, create_dwrite_factory};

pub struct Renderer {
    hwnd: HWND,
    d2d_factory: ID2D1Factory,
    dwrite_factory: IDWriteFactory,
    target: Option<ID2D1HwndRenderTarget>,
    brush_cache: BrushCache,
    text_format_cache: TextFormatCache,
}

impl Renderer {
    pub fn new(hwnd: HWND) -> RenderResult<Self> {
        let d2d_factory = create_d2d_factory()?;
        let dwrite_factory = create_dwrite_factory()?;

        Ok(Self {
            hwnd,
            d2d_factory,
            dwrite_factory,
            target: None,
            brush_cache: BrushCache::default(),
            text_format_cache: TextFormatCache::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> RenderResult<()> {
        if let Some(target) = &self.target {
            let size = D2D_SIZE_U { width, height };
            // SAFETY: The render target belongs to this hwnd and the size is from WM_SIZE/client rect.
            unsafe { target.Resize(&size).context("resize d2d render target")? };
            self.brush_cache.clear();
        }
        Ok(())
    }

    pub fn draw(&mut self, snapshot: &ViewSnapshot, width: u32, height: u32) -> RenderResult<()> {
        self.ensure_target(width, height)?;
        let target = self.target.as_ref().expect("target ensured");
        let clear = d2d_color(ColorToken::Surface.color());
        let mut context = RenderContext::new(
            target,
            &mut self.brush_cache,
            &self.dwrite_factory,
            &mut self.text_format_cache,
        );

        // SAFETY: All Direct2D calls use initialized COM interfaces owned by this renderer. Rects and
        // colors are stack values that live through each call. BeginDraw/EndDraw are paired.
        unsafe {
            target.BeginDraw();
            target.Clear(Some(&clear));
            snapshot.primitive_tree.draw_windows(&mut context)?;
            target.EndDraw(None, None).context("end d2d draw")?;
        }
        Ok(())
    }

    fn ensure_target(&mut self, width: u32, height: u32) -> RenderResult<()> {
        if self.target.is_some() {
            return Ok(());
        }

        let render_props = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_UNKNOWN,
                alphaMode: D2D1_ALPHA_MODE_UNKNOWN,
            },
            dpiX: 0.0,
            dpiY: 0.0,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };
        let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
            hwnd: self.hwnd,
            pixelSize: D2D_SIZE_U { width, height },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE,
        };

        // SAFETY: hwnd is a live window owned by KeyDockWindow. Property pointers reference stack
        // values valid for the duration of the call. The resulting COM pointer is owned by Renderer.
        let target = unsafe {
            self.d2d_factory
                .CreateHwndRenderTarget(&render_props, &hwnd_props)
                .context("create d2d hwnd render target")?
        };
        self.target = Some(target);
        Ok(())
    }
}
