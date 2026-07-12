use std::{error::Error, fmt};

use game_ui::TextLabel;
use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, PrepareError, RenderError,
    Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use punctum_gpu::{PixelSize, Viewport as GridViewport};

pub struct BattleTextRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    gpu: Option<TextGpu>,
}

impl BattleTextRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            gpu: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn encode(
        &mut self,
        labels: &[TextLabel],
        viewport: GridViewport,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        format: wgpu::TextureFormat,
        surface_size: PixelSize,
    ) -> Result<(), TextRenderError> {
        if self.gpu.as_ref().is_none_or(|gpu| gpu.format != format) {
            self.gpu = Some(TextGpu::new(device, queue, format));
        }
        self.gpu
            .as_mut()
            .expect("text GPU resources were initialized")
            .encode(
                labels,
                viewport,
                device,
                queue,
                target,
                encoder,
                surface_size,
                &mut self.font_system,
                &mut self.swash_cache,
            )
    }
}

impl Default for BattleTextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

struct TextGpu {
    format: wgpu::TextureFormat,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
}

impl TextGpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self {
            format,
            viewport,
            atlas,
            renderer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn encode(
        &mut self,
        labels: &[TextLabel],
        grid_viewport: GridViewport,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        surface_size: PixelSize,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
    ) -> Result<(), TextRenderError> {
        self.viewport.update(
            queue,
            Resolution {
                width: surface_size.width,
                height: surface_size.height,
            },
        );

        let mut buffers = Vec::with_capacity(labels.len());
        let mut areas = Vec::with_capacity(labels.len());
        for label in labels {
            let bounds = pixel_bounds(label, grid_viewport)?;
            let font_size = (grid_viewport.cell_size.height * 3 / 5).clamp(10, 28) as f32;
            let mut buffer = Buffer::new(
                font_system,
                Metrics::new(font_size, bounds.height().max(1) as f32),
            );
            buffer.set_size(
                Some(bounds.width().max(1) as f32),
                Some(bounds.height().max(1) as f32),
            );
            buffer.set_text(
                &label.content,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);
            buffers.push(buffer);
            areas.push((
                bounds,
                Color::rgba(
                    label.color.red,
                    label.color.green,
                    label.color.blue,
                    label.color.alpha,
                ),
            ));
        }

        self.renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            buffers
                .iter()
                .zip(&areas)
                .map(|(buffer, (bounds, color))| TextArea {
                    buffer,
                    left: bounds.left as f32,
                    top: bounds.top as f32,
                    scale: 1.0,
                    bounds: *bounds,
                    default_color: *color,
                    custom_glyphs: &[],
                }),
            swash_cache,
        )?;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("creature battle text"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        self.renderer
            .render(&self.atlas, &self.viewport, &mut pass)?;
        drop(pass);
        self.atlas.trim();
        Ok(())
    }
}

fn pixel_bounds(label: &TextLabel, viewport: GridViewport) -> Result<TextBounds, TextRenderError> {
    let left =
        i64::from(viewport.origin.x) + i64::from(label.col) * i64::from(viewport.cell_size.width);
    let top =
        i64::from(viewport.origin.y) + i64::from(label.row) * i64::from(viewport.cell_size.height);
    let right = left + i64::from(label.width) * i64::from(viewport.cell_size.width);
    let bottom = top + i64::from(label.height) * i64::from(viewport.cell_size.height);
    Ok(TextBounds {
        left: i32::try_from(left).map_err(|_| TextRenderError::CoordinateOverflow)?,
        top: i32::try_from(top).map_err(|_| TextRenderError::CoordinateOverflow)?,
        right: i32::try_from(right).map_err(|_| TextRenderError::CoordinateOverflow)?,
        bottom: i32::try_from(bottom).map_err(|_| TextRenderError::CoordinateOverflow)?,
    })
}

trait TextBoundsSize {
    fn width(self) -> i32;
    fn height(self) -> i32;
}

impl TextBoundsSize for TextBounds {
    fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

#[derive(Debug)]
pub enum TextRenderError {
    CoordinateOverflow,
    Prepare(PrepareError),
    Render(RenderError),
}

impl fmt::Display for TextRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoordinateOverflow => formatter.write_str("battle text coordinates overflowed"),
            Self::Prepare(error) => write!(formatter, "failed to prepare battle text: {error}"),
            Self::Render(error) => write!(formatter, "failed to render battle text: {error}"),
        }
    }
}

impl Error for TextRenderError {}

impl From<PrepareError> for TextRenderError {
    fn from(error: PrepareError) -> Self {
        Self::Prepare(error)
    }
}

impl From<RenderError> for TextRenderError {
    fn from(error: RenderError) -> Self {
        Self::Render(error)
    }
}
