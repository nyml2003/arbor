#![cfg_attr(not(test), allow(dead_code))]

fn main() {}

#[path = "gpu/palette_overlay.rs"]
mod palette_overlay;
#[path = "gpu/ramus_palette.rs"]
mod ramus_palette;

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use punctum_gpu::PixelSize;

    use super::palette_overlay::{
        MAX_VISIBLE_ITEMS, OverlayPrimitive, PaletteOverlayRenderer, PlannerNotice, RectRole,
        TextRole, plan_palette_overlay,
    };
    use super::ramus_palette::{CommandQueue, PaletteIntent, PaletteState, RamusPalette};

    fn open_palette() -> (RamusPalette, PaletteState) {
        let palette = RamusPalette::new(CommandQueue::default());
        let mut state = PaletteState::default();
        assert!(matches!(
            palette.handle(&mut state, PaletteIntent::Open),
            super::ramus_palette::PaletteOutcome::Updated
        ));
        (palette, state)
    }

    fn assert_within_surface(plan: &super::palette_overlay::PaletteOverlayPlan) {
        let surface = plan.surface_size();
        for primitive in plan.primitives() {
            let bounds = primitive.bounds();
            assert!(bounds.right() <= surface.width, "{primitive:?}");
            assert!(bounds.bottom() <= surface.height, "{primitive:?}");
        }
    }

    fn candidate_range(plan: &super::palette_overlay::PaletteOverlayPlan) -> Range<usize> {
        plan.first_visible_item()..plan.first_visible_item() + plan.visible_item_count()
    }

    #[test]
    fn closed_or_zero_sized_palettes_produce_an_empty_plan() {
        let closed = PaletteState::default();
        assert!(plan_palette_overlay(&closed, None, PixelSize::new(480, 704)).is_empty());

        let (_, open) = open_palette();
        assert!(plan_palette_overlay(&open, None, PixelSize::new(0, 704)).is_empty());
        assert!(plan_palette_overlay(&open, None, PixelSize::new(480, 0)).is_empty());
    }

    #[test]
    fn three_required_viewports_keep_every_primitive_inside_the_surface() {
        let (_, state) = open_palette();

        for size in [
            PixelSize::new(480, 704),
            PixelSize::new(320, 480),
            PixelSize::new(960, 1408),
        ] {
            let plan = plan_palette_overlay(&state, None, size);
            assert!(!plan.is_empty());
            assert!(plan.panel().is_some());
            assert_within_surface(&plan);
        }
    }

    #[test]
    fn text_and_rectangles_remain_distinct_plan_primitives() {
        let (_, state) = open_palette();
        let plan = plan_palette_overlay(&state, None, PixelSize::new(480, 704));

        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Rect {
                    role: RectRole::Panel,
                    ..
                }
            )
        }));
        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Text {
                    role: TextRole::Query,
                    ..
                }
            )
        }));
        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Text {
                    role: TextRole::Candidate(_),
                    ..
                }
            )
        }));
    }

    #[test]
    fn selected_item_stays_visible_when_the_candidate_list_is_clipped() {
        let (palette, mut state) = open_palette();
        palette.handle(&mut state, PaletteIntent::Previous);
        let selected = state.selected_index().expect("the last item is selected");
        let plan = plan_palette_overlay(&state, None, PixelSize::new(320, 480));

        assert!(plan.visible_item_count() <= MAX_VISIBLE_ITEMS);
        assert!(candidate_range(&plan).contains(&selected));
        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Rect {
                    role: RectRole::Selection,
                    ..
                }
            )
        }));
    }

    #[test]
    fn zero_matches_show_a_diagnostic_without_overlapping_the_query() {
        let (palette, mut state) = open_palette();
        palette.handle(
            &mut state,
            PaletteIntent::InsertText("no-command-can-match-this".into()),
        );
        palette.handle(&mut state, PaletteIntent::Execute);
        let plan = plan_palette_overlay(&state, None, PixelSize::new(320, 480));

        let query = plan
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                OverlayPrimitive::Text {
                    role: TextRole::Query,
                    bounds,
                    ..
                } => Some(*bounds),
                _ => None,
            })
            .expect("query text");
        let diagnostic = plan
            .primitives()
            .iter()
            .find_map(|primitive| match primitive {
                OverlayPrimitive::Text {
                    role: TextRole::Diagnostic,
                    bounds,
                    ..
                } => Some(*bounds),
                _ => None,
            })
            .expect("diagnostic text");

        assert!(query.bottom() <= diagnostic.y);
        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Text {
                    role: TextRole::Empty,
                    ..
                }
            )
        }));
        assert_within_surface(&plan);
    }

    #[test]
    fn selection_changes_do_not_resize_the_panel_or_candidate_rows() {
        let (palette, mut first) = open_palette();
        let first_plan = plan_palette_overlay(&first, None, PixelSize::new(480, 704));
        palette.handle(&mut first, PaletteIntent::Next);
        let second_plan = plan_palette_overlay(&first, None, PixelSize::new(480, 704));

        assert_eq!(first_plan.panel(), second_plan.panel());
        let candidate_bounds = |plan: &super::palette_overlay::PaletteOverlayPlan| {
            plan.primitives()
                .iter()
                .filter_map(|primitive| match primitive {
                    OverlayPrimitive::Text {
                        role: TextRole::Candidate(_),
                        bounds,
                        ..
                    } => Some(*bounds),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            candidate_bounds(&first_plan),
            candidate_bounds(&second_plan)
        );
    }

    #[test]
    fn planner_notice_uses_one_status_row_and_stays_inside_the_surface() {
        let (_, state) = open_palette();
        for notice in [
            PlannerNotice::Pending,
            PlannerNotice::Failed("timeout: the Ollama request timed out"),
        ] {
            let plan = plan_palette_overlay(&state, Some(notice), PixelSize::new(320, 480));
            let planner_rows = plan
                .primitives()
                .iter()
                .filter(|primitive| {
                    matches!(
                        primitive,
                        OverlayPrimitive::Text {
                            role: TextRole::Planner,
                            ..
                        }
                    )
                })
                .count();

            assert_eq!(planner_rows, 1);
            assert_within_surface(&plan);
        }
    }

    #[test]
    fn planner_failure_takes_precedence_over_palette_diagnostic() {
        let (palette, mut state) = open_palette();
        palette.handle(
            &mut state,
            PaletteIntent::InsertText("no-command-can-match-this".into()),
        );
        palette.handle(&mut state, PaletteIntent::Execute);
        let plan = plan_palette_overlay(
            &state,
            Some(PlannerNotice::Failed("invalid response")),
            PixelSize::new(320, 480),
        );

        assert!(plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Text {
                    role: TextRole::Planner,
                    content,
                    ..
                } if content.contains("invalid response")
            )
        }));
        assert!(!plan.primitives().iter().any(|primitive| {
            matches!(
                primitive,
                OverlayPrimitive::Text {
                    role: TextRole::Diagnostic,
                    ..
                }
            )
        }));
    }

    #[test]
    fn renderer_can_be_created_without_owning_a_surface_or_queue() {
        let _renderer = PaletteOverlayRenderer::new();
    }

    #[test]
    fn close_and_backspace_intents_remain_available_to_the_host() {
        let (palette, mut state) = open_palette();
        palette.handle(&mut state, PaletteIntent::InsertText("rot".into()));
        palette.handle(&mut state, PaletteIntent::Backspace);
        assert_eq!(state.query(), "ro");
        palette.handle(&mut state, PaletteIntent::Close);
        assert!(!state.is_open());
    }

    #[test]
    #[ignore = "requires a local GPU adapter"]
    fn headless_overlay_pipeline_encodes_without_submitting() {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("local GPU adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("local GPU device");
        let size = PixelSize::new(480, 704);
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Tetris palette smoke target"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Tetris palette smoke encoder"),
        });
        let (_, state) = open_palette();
        let plan = plan_palette_overlay(&state, None, size);
        let mut renderer = PaletteOverlayRenderer::new();

        renderer
            .encode(&plan, &device, &queue, &view, &mut encoder, format, size)
            .expect("overlay encodes into the caller-owned command encoder");

        drop(encoder.finish());
    }
}
