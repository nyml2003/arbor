use std::{error::Error, fmt};

use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Family, FontSystem, Metrics, PrepareError,
    RenderError, Resolution, Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer,
    Viewport,
};
use punctum_gpu::{PixelSize, Rgba8};

use crate::ramus_palette::PaletteState;

pub const MAX_VISIBLE_ITEMS: usize = 5;

const OUTER_MARGIN: u32 = 16;
const PANEL_PADDING: u32 = 12;
const QUERY_HEIGHT: u32 = 36;
const ITEM_HEIGHT: u32 = 28;
const DIAGNOSTIC_HEIGHT: u32 = 28;
const QUERY_FONT_SIZE: u32 = 18;
const ITEM_FONT_SIZE: u32 = 16;
const DIAGNOSTIC_FONT_SIZE: u32 = 14;

const PANEL_COLOR: Rgba8 = Rgba8::new(20, 23, 29, 246);
const SELECTION_COLOR: Rgba8 = Rgba8::new(42, 112, 138, 255);
const PRIMARY_TEXT_COLOR: Rgba8 = Rgba8::new(240, 243, 247, 255);
const SECONDARY_TEXT_COLOR: Rgba8 = Rgba8::new(184, 193, 204, 255);
const DIAGNOSTIC_TEXT_COLOR: Rgba8 = Rgba8::new(255, 150, 126, 255);

const RECT_INSTANCE_STRIDE: u64 = 20;

const RECT_SHADER: &str = r#"
struct VertexInput {
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];
    let position = mix(input.rect.xy, input.rect.zw, corner);
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
"#;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl UiRect {
    pub const fn right(self) -> u32 {
        self.x.saturating_add(self.width)
    }

    pub const fn bottom(self) -> u32 {
        self.y.saturating_add(self.height)
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > left && bottom > top).then_some(Self {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectRole {
    Panel,
    Selection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Query,
    Candidate(usize),
    Empty,
    Planner,
    Diagnostic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannerNotice<'a> {
    Pending,
    Failed(&'a str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OverlayPrimitive {
    Rect {
        role: RectRole,
        bounds: UiRect,
        color: Rgba8,
    },
    Text {
        role: TextRole,
        bounds: UiRect,
        content: String,
        color: Rgba8,
        font_size: u32,
    },
}

impl OverlayPrimitive {
    #[allow(dead_code)]
    pub const fn bounds(&self) -> UiRect {
        match self {
            Self::Rect { bounds, .. } | Self::Text { bounds, .. } => *bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaletteOverlayPlan {
    surface_size: PixelSize,
    panel: Option<UiRect>,
    primitives: Vec<OverlayPrimitive>,
    first_visible_item: usize,
    visible_item_count: usize,
}

impl PaletteOverlayPlan {
    #[allow(dead_code)]
    pub const fn surface_size(&self) -> PixelSize {
        self.surface_size
    }

    #[allow(dead_code)]
    pub const fn panel(&self) -> Option<UiRect> {
        self.panel
    }

    #[allow(dead_code)]
    pub fn primitives(&self) -> &[OverlayPrimitive] {
        &self.primitives
    }

    #[allow(dead_code)]
    pub const fn first_visible_item(&self) -> usize {
        self.first_visible_item
    }

    #[allow(dead_code)]
    pub const fn visible_item_count(&self) -> usize {
        self.visible_item_count
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}

pub fn plan_palette_overlay(
    state: &PaletteState,
    planner_notice: Option<PlannerNotice<'_>>,
    surface_size: PixelSize,
) -> PaletteOverlayPlan {
    if !state.is_open() || surface_size.is_empty() {
        return empty_plan(surface_size);
    }

    let horizontal_margin = OUTER_MARGIN.min(surface_size.width / 4);
    let bottom_margin = OUTER_MARGIN.min(surface_size.height / 4);
    let panel_width = surface_size
        .width
        .saturating_sub(horizontal_margin.saturating_mul(2));
    let available_height = surface_size.height.saturating_sub(bottom_margin);
    if panel_width == 0 || available_height == 0 {
        return empty_plan(surface_size);
    }

    let notice_height = if planner_notice.is_some() || state.diagnostic().is_some() {
        DIAGNOSTIC_HEIGHT
    } else {
        0
    };
    let minimum_height = PANEL_PADDING
        .saturating_mul(2)
        .saturating_add(QUERY_HEIGHT)
        .saturating_add(notice_height);
    let proportional_height = ((u64::from(surface_size.height) * 45) / 100) as u32;
    let maximum_height = available_height.min(proportional_height.max(minimum_height));
    let item_capacity = maximum_height
        .saturating_sub(minimum_height)
        .checked_div(ITEM_HEIGHT)
        .unwrap_or(0) as usize;
    let visible_item_count = state
        .items()
        .len()
        .min(MAX_VISIBLE_ITEMS)
        .min(item_capacity);
    let empty_row_count = usize::from(state.items().is_empty() && item_capacity > 0);
    let row_count = visible_item_count.max(empty_row_count);
    let desired_height = minimum_height.saturating_add(
        u32::try_from(row_count)
            .unwrap_or(u32::MAX)
            .saturating_mul(ITEM_HEIGHT),
    );
    let panel_height = desired_height.min(maximum_height);
    if panel_height == 0 {
        return empty_plan(surface_size);
    }

    let panel = UiRect {
        x: horizontal_margin,
        y: surface_size
            .height
            .saturating_sub(bottom_margin)
            .saturating_sub(panel_height),
        width: panel_width,
        height: panel_height,
    };
    let content_width = panel.width.saturating_sub(PANEL_PADDING.saturating_mul(2));
    let content_x = panel.x.saturating_add(PANEL_PADDING);
    let query_bounds = clipped_row(
        panel,
        content_x,
        panel.y.saturating_add(PANEL_PADDING),
        content_width,
        QUERY_HEIGHT,
    );

    let first_visible_item = visible_window_start(state, visible_item_count);
    let mut primitives = vec![OverlayPrimitive::Rect {
        role: RectRole::Panel,
        bounds: panel,
        color: PANEL_COLOR,
    }];
    if let Some(bounds) = query_bounds {
        primitives.push(OverlayPrimitive::Text {
            role: TextRole::Query,
            bounds,
            content: format!("> {}", state.query()),
            color: PRIMARY_TEXT_COLOR,
            font_size: QUERY_FONT_SIZE,
        });
    }

    let rows_y = panel
        .y
        .saturating_add(PANEL_PADDING)
        .saturating_add(QUERY_HEIGHT);
    for (visible_offset, item_index) in
        (first_visible_item..first_visible_item.saturating_add(visible_item_count)).enumerate()
    {
        let row_y = rows_y.saturating_add(
            u32::try_from(visible_offset)
                .unwrap_or(u32::MAX)
                .saturating_mul(ITEM_HEIGHT),
        );
        let Some(bounds) = clipped_row(panel, content_x, row_y, content_width, ITEM_HEIGHT) else {
            continue;
        };
        if state.selected_index() == Some(item_index) {
            primitives.push(OverlayPrimitive::Rect {
                role: RectRole::Selection,
                bounds,
                color: SELECTION_COLOR,
            });
        }
        if let Some(item) = state.items().get(item_index) {
            primitives.push(OverlayPrimitive::Text {
                role: TextRole::Candidate(item_index),
                bounds,
                content: item.clone(),
                color: PRIMARY_TEXT_COLOR,
                font_size: ITEM_FONT_SIZE,
            });
        }
    }

    if state.items().is_empty()
        && let Some(bounds) = clipped_row(panel, content_x, rows_y, content_width, ITEM_HEIGHT)
    {
        primitives.push(OverlayPrimitive::Text {
            role: TextRole::Empty,
            bounds,
            content: "No matching commands".into(),
            color: SECONDARY_TEXT_COLOR,
            font_size: ITEM_FONT_SIZE,
        });
    }

    if let Some((role, content, color)) = planner_notice
        .map(|notice| match notice {
            PlannerNotice::Pending => (
                TextRole::Planner,
                "Planning command...".to_owned(),
                SECONDARY_TEXT_COLOR,
            ),
            PlannerNotice::Failed(message) => (
                TextRole::Planner,
                format!("planner: {message}"),
                DIAGNOSTIC_TEXT_COLOR,
            ),
        })
        .or_else(|| {
            state.diagnostic().map(|diagnostic| {
                (
                    TextRole::Diagnostic,
                    format!("{}: {}", diagnostic.code, diagnostic.message),
                    DIAGNOSTIC_TEXT_COLOR,
                )
            })
        })
    {
        let diagnostic_y = rows_y.saturating_add(
            u32::try_from(row_count)
                .unwrap_or(u32::MAX)
                .saturating_mul(ITEM_HEIGHT),
        );
        if let Some(bounds) = clipped_row(
            panel,
            content_x,
            diagnostic_y,
            content_width,
            DIAGNOSTIC_HEIGHT,
        ) {
            primitives.push(OverlayPrimitive::Text {
                role,
                bounds,
                content,
                color,
                font_size: DIAGNOSTIC_FONT_SIZE,
            });
        }
    }

    PaletteOverlayPlan {
        surface_size,
        panel: Some(panel),
        primitives,
        first_visible_item,
        visible_item_count,
    }
}

fn empty_plan(surface_size: PixelSize) -> PaletteOverlayPlan {
    PaletteOverlayPlan {
        surface_size,
        panel: None,
        primitives: Vec::new(),
        first_visible_item: 0,
        visible_item_count: 0,
    }
}

fn clipped_row(panel: UiRect, x: u32, y: u32, width: u32, height: u32) -> Option<UiRect> {
    UiRect {
        x,
        y,
        width,
        height,
    }
    .intersection(panel)
}

fn visible_window_start(state: &PaletteState, visible_item_count: usize) -> usize {
    if visible_item_count == 0 {
        return 0;
    }
    let maximum_start = state.items().len().saturating_sub(visible_item_count);
    state
        .selected_index()
        .map_or(0, |selected| {
            selected
                .saturating_add(1)
                .saturating_sub(visible_item_count)
        })
        .min(maximum_start)
}

pub struct PaletteOverlayRenderer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    gpu: Option<OverlayGpu>,
}

impl PaletteOverlayRenderer {
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
        plan: &PaletteOverlayPlan,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        format: wgpu::TextureFormat,
        surface_size: PixelSize,
    ) -> Result<(), PaletteOverlayRenderError> {
        if plan.surface_size != surface_size {
            return Err(PaletteOverlayRenderError::SurfaceSizeMismatch {
                plan: plan.surface_size,
                actual: surface_size,
            });
        }
        if plan.panel.is_none() {
            return Ok(());
        }

        if self.gpu.as_ref().is_none_or(|gpu| gpu.format != format) {
            self.gpu = Some(OverlayGpu::new(device, queue, format));
        }
        let gpu = self.gpu.as_mut().expect("GPU resources were initialized");
        gpu.prepare_rectangles(device, queue, plan, surface_size)?;
        gpu.prepare_text(
            device,
            queue,
            &mut self.font_system,
            &mut self.swash_cache,
            plan,
            surface_size,
        )?;
        gpu.render(target, encoder)
    }
}

impl Default for PaletteOverlayRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum PaletteOverlayRenderError {
    SurfaceSizeMismatch { plan: PixelSize, actual: PixelSize },
    PixelStorageOverflow,
    CoordinateOverflow,
    PrepareText(PrepareError),
    RenderText(RenderError),
}

impl fmt::Display for PaletteOverlayRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceSizeMismatch { plan, actual } => write!(
                formatter,
                "palette overlay plan targets {plan:?}, but the frame is {actual:?}"
            ),
            Self::PixelStorageOverflow => {
                formatter.write_str("palette overlay pixel storage exceeds addressable memory")
            }
            Self::CoordinateOverflow => {
                formatter.write_str("palette overlay text bounds exceed signed GPU coordinates")
            }
            Self::PrepareText(error) => {
                write!(formatter, "failed to prepare palette text: {error}")
            }
            Self::RenderText(error) => write!(formatter, "failed to render palette text: {error}"),
        }
    }
}

impl Error for PaletteOverlayRenderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PrepareText(error) => Some(error),
            Self::RenderText(error) => Some(error),
            _ => None,
        }
    }
}

impl From<PrepareError> for PaletteOverlayRenderError {
    fn from(error: PrepareError) -> Self {
        Self::PrepareText(error)
    }
}

impl From<RenderError> for PaletteOverlayRenderError {
    fn from(error: RenderError) -> Self {
        Self::RenderText(error)
    }
}

struct OverlayGpu {
    format: wgpu::TextureFormat,
    rect_pipeline: wgpu::RenderPipeline,
    rect_buffer: wgpu::Buffer,
    rect_capacity: u64,
    rect_count: u32,
    text_viewport: Viewport,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
}

impl OverlayGpu {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let rect_pipeline = create_rect_pipeline(device, format);
        let cache = Cache::new(device);
        let text_viewport = Viewport::new(device, &cache);
        let mut text_atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer = TextRenderer::new(
            &mut text_atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        Self {
            format,
            rect_pipeline,
            rect_buffer: create_vertex_buffer(
                device,
                "Tetris palette rect instances",
                RECT_INSTANCE_STRIDE,
            ),
            rect_capacity: RECT_INSTANCE_STRIDE,
            rect_count: 0,
            text_viewport,
            text_atlas,
            text_renderer,
        }
    }

    fn prepare_rectangles(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: &PaletteOverlayPlan,
        surface_size: PixelSize,
    ) -> Result<(), PaletteOverlayRenderError> {
        let bytes = encode_rectangles(plan, surface_size);
        let required = u64::try_from(bytes.len())
            .map_err(|_| PaletteOverlayRenderError::PixelStorageOverflow)?;
        if required > self.rect_capacity {
            self.rect_buffer = create_vertex_buffer(
                device,
                "Tetris palette rect instances",
                required.max(RECT_INSTANCE_STRIDE),
            );
            self.rect_capacity = required;
        }
        if !bytes.is_empty() {
            queue.write_buffer(&self.rect_buffer, 0, &bytes);
        }
        self.rect_count = u32::try_from(bytes.len() as u64 / RECT_INSTANCE_STRIDE)
            .map_err(|_| PaletteOverlayRenderError::PixelStorageOverflow)?;
        Ok(())
    }

    fn prepare_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        plan: &PaletteOverlayPlan,
        surface_size: PixelSize,
    ) -> Result<(), PaletteOverlayRenderError> {
        self.text_viewport.update(
            queue,
            Resolution {
                width: surface_size.width,
                height: surface_size.height,
            },
        );

        let mut buffers = Vec::new();
        let mut areas = Vec::new();
        for primitive in &plan.primitives {
            let OverlayPrimitive::Text {
                bounds,
                content,
                color,
                font_size,
                ..
            } = primitive
            else {
                continue;
            };
            let mut buffer = Buffer::new(
                font_system,
                Metrics::new(*font_size as f32, bounds.height as f32),
            );
            buffer.set_size(Some(bounds.width as f32), Some(bounds.height as f32));
            buffer.set_text(
                content,
                &Attrs::new().family(Family::Monospace),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(font_system, false);
            buffers.push(buffer);
            areas.push((
                *bounds,
                text_bounds(*bounds)?,
                GlyphColor::rgba(color.red, color.green, color.blue, color.alpha),
            ));
        }

        let text_areas = buffers
            .iter()
            .zip(&areas)
            .map(|(buffer, (bounds, clip, color))| TextArea {
                buffer,
                left: bounds.x as f32,
                top: bounds.y as f32,
                scale: 1.0,
                bounds: *clip,
                default_color: *color,
                custom_glyphs: &[],
            });
        self.text_renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.text_atlas,
            &self.text_viewport,
            text_areas,
            swash_cache,
        )?;
        Ok(())
    }

    fn render(
        &mut self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), PaletteOverlayRenderError> {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tetris command palette overlay"),
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
        if self.rect_count > 0 {
            pass.set_pipeline(&self.rect_pipeline);
            pass.set_vertex_buffer(0, self.rect_buffer.slice(..));
            pass.draw(0..6, 0..self.rect_count);
        }
        self.text_renderer
            .render(&self.text_atlas, &self.text_viewport, &mut pass)?;
        drop(pass);
        self.text_atlas.trim();
        Ok(())
    }
}

fn text_bounds(bounds: UiRect) -> Result<TextBounds, PaletteOverlayRenderError> {
    Ok(TextBounds {
        left: i32::try_from(bounds.x).map_err(|_| PaletteOverlayRenderError::CoordinateOverflow)?,
        top: i32::try_from(bounds.y).map_err(|_| PaletteOverlayRenderError::CoordinateOverflow)?,
        right: i32::try_from(bounds.right())
            .map_err(|_| PaletteOverlayRenderError::CoordinateOverflow)?,
        bottom: i32::try_from(bounds.bottom())
            .map_err(|_| PaletteOverlayRenderError::CoordinateOverflow)?,
    })
}

fn encode_rectangles(plan: &PaletteOverlayPlan, surface_size: PixelSize) -> Vec<u8> {
    let rect_count = plan
        .primitives
        .iter()
        .filter(|primitive| matches!(primitive, OverlayPrimitive::Rect { .. }))
        .count();
    let mut bytes = Vec::with_capacity(rect_count.saturating_mul(RECT_INSTANCE_STRIDE as usize));
    for primitive in &plan.primitives {
        let OverlayPrimitive::Rect { bounds, color, .. } = primitive else {
            continue;
        };
        for value in encode_ndc_rect(*bounds, surface_size) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&color.to_array());
    }
    bytes
}

fn encode_ndc_rect(bounds: UiRect, surface_size: PixelSize) -> [f32; 4] {
    let width = surface_size.width.max(1) as f32;
    let height = surface_size.height.max(1) as f32;
    [
        bounds.x as f32 * 2.0 / width - 1.0,
        1.0 - bounds.y as f32 * 2.0 / height,
        bounds.right() as f32 * 2.0 / width - 1.0,
        1.0 - bounds.bottom() as f32 * 2.0 / height,
    ]
}

fn create_vertex_buffer(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size.max(4),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn create_rect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Tetris palette rect shader"),
        source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Tetris palette rect pipeline layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Tetris palette rect pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: RECT_INSTANCE_STRIDE,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Unorm8x4,
                        offset: 16,
                        shader_location: 1,
                    },
                ],
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(alpha_target(format))],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn alpha_target(format: wgpu::TextureFormat) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    }
}
