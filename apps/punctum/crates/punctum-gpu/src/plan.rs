use std::{error::Error, fmt};

use punctum_grid::{GridPos, GridRect, GridSize, Patch, PatchKind, Surface};

use crate::{GpuAtlas, GpuCell, GpuClip, PixelRect, ResourceId, Viewport};

pub const INSTANCE_STRIDE: u64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionMode {
    Replace,
    Delta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstanceData {
    pub grid_position: [u32; 2],
    pub atlas_rect: [u32; 4],
    pub tint: [u8; 4],
    pub visible: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceUpload {
    pub first_slot: u32,
    pub instances: Vec<InstanceData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmissionPlan {
    pub grid_size: GridSize,
    pub mode: SubmissionMode,
    pub viewport: Viewport,
    pub scissor: Option<PixelRect>,
    pub instance_count: u32,
    pub uploads: Vec<InstanceUpload>,
}

pub fn plan_surface(
    surface: &Surface<GpuCell>,
    atlas: &GpuAtlas,
    max_instances: u32,
    viewport: Viewport,
    clip: GpuClip,
) -> Result<SubmissionPlan, GpuPlanError> {
    let size = surface.size();
    let instance_count = checked_instance_count(size, max_instances)?;
    let uploads = if surface.cells().is_empty() {
        Vec::new()
    } else {
        let mut instances = Vec::with_capacity(surface.cells().len());
        for (index, cell) in surface.cells().iter().enumerate() {
            let col = index as u32 % size.cols;
            let row = index as u32 / size.cols;
            instances.push(plan_cell(
                GridPos::new(col as i32, row as i32),
                cell,
                atlas,
            )?);
        }
        vec![InstanceUpload {
            first_slot: 0,
            instances,
        }]
    };

    Ok(SubmissionPlan {
        grid_size: size,
        mode: SubmissionMode::Replace,
        viewport,
        scissor: plan_scissor(size, viewport, clip),
        instance_count,
        uploads,
    })
}

pub fn plan_patch(
    patch: &Patch<GpuCell>,
    atlas: &GpuAtlas,
    max_instances: u32,
    viewport: Viewport,
    clip: GpuClip,
) -> Result<SubmissionPlan, GpuPlanError> {
    let size = patch.size();
    let instance_count = checked_instance_count(size, max_instances)?;
    let mut uploads = Vec::with_capacity(patch.spans().len());

    for span in patch.spans() {
        let first_slot = u64::from(span.row()) * u64::from(size.cols) + u64::from(span.start_col());
        let mut instances = Vec::with_capacity(span.cells().len());
        for (offset, cell) in span.cells().iter().enumerate() {
            let col = u64::from(span.start_col()) + offset as u64;
            let position = GridPos::new(col as i32, span.row() as i32);
            instances.push(plan_cell(position, cell, atlas)?);
        }
        uploads.push(InstanceUpload {
            first_slot: first_slot as u32,
            instances,
        });
    }

    Ok(SubmissionPlan {
        grid_size: size,
        mode: match patch.kind() {
            PatchKind::Replace => SubmissionMode::Replace,
            PatchKind::Delta => SubmissionMode::Delta,
        },
        viewport,
        scissor: plan_scissor(size, viewport, clip),
        instance_count,
        uploads,
    })
}

fn checked_instance_count(size: GridSize, maximum: u32) -> Result<u32, GpuPlanError> {
    let count = u64::from(size.cols) * u64::from(size.rows);
    if count > u64::from(maximum) {
        return Err(GpuPlanError::InstanceCountOverflow { size, maximum });
    }
    Ok(count as u32)
}

fn plan_cell(
    position: GridPos,
    cell: &GpuCell,
    atlas: &GpuAtlas,
) -> Result<InstanceData, GpuPlanError> {
    let grid_position = [position.col as u32, position.row as u32];
    match *cell {
        GpuCell::Empty => Ok(InstanceData {
            grid_position,
            atlas_rect: [0; 4],
            tint: [0; 4],
            visible: 0,
        }),
        GpuCell::Sprite { resource, tint } => {
            let rect = atlas
                .resource(resource)
                .ok_or(GpuPlanError::MissingResource { position, resource })?;
            Ok(InstanceData {
                grid_position,
                atlas_rect: [rect.x, rect.y, rect.width, rect.height],
                tint: tint.to_array(),
                visible: 1,
            })
        }
    }
}

fn plan_scissor(size: GridSize, viewport: Viewport, clip: GpuClip) -> Option<PixelRect> {
    if viewport.target_size.is_empty() {
        return None;
    }

    let requested = match clip {
        GpuClip::Surface => GridRect::new(GridPos::new(0, 0), size),
        GpuClip::Rect(rect) => rect,
    };
    let clipped = requested.clip_to(size)?;

    let left = i128::from(viewport.origin.x)
        + i128::from(clipped.origin.col) * i128::from(viewport.cell_size.width);
    let top = i128::from(viewport.origin.y)
        + i128::from(clipped.origin.row) * i128::from(viewport.cell_size.height);
    let right = left + i128::from(clipped.size.cols) * i128::from(viewport.cell_size.width);
    let bottom = top + i128::from(clipped.size.rows) * i128::from(viewport.cell_size.height);

    let target_width = i128::from(viewport.target_size.width);
    let target_height = i128::from(viewport.target_size.height);
    let x0 = left.clamp(0, target_width);
    let y0 = top.clamp(0, target_height);
    let x1 = right.clamp(0, target_width);
    let y1 = bottom.clamp(0, target_height);

    if x0 >= x1 || y0 >= y1 {
        return None;
    }

    Some(PixelRect::new(
        x0 as u32,
        y0 as u32,
        (x1 - x0) as u32,
        (y1 - y0) as u32,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPlanError {
    InstanceCountOverflow {
        size: GridSize,
        maximum: u32,
    },
    MissingResource {
        position: GridPos,
        resource: ResourceId,
    },
}

impl fmt::Display for GpuPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstanceCountOverflow { size, maximum } => {
                write!(
                    formatter,
                    "grid {size:?} exceeds the GPU instance limit {maximum}"
                )
            }
            Self::MissingResource { position, resource } => write!(
                formatter,
                "GPU resource {resource:?} is missing for cell {position:?}"
            ),
        }
    }
}

impl Error for GpuPlanError {}
