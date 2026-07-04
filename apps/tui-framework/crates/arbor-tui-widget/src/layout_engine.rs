// Layout engine — two-pass flexbox layout.
// Pass 1: measure_tree (bottom-up, generic via Widget trait)
// Pass 2: layout_tree (top-down, flex allocation with remainder distribution)
//
// Zero per-widget-type dispatch. All type-specific behavior is in
// Widget::measure_subtree() and Widget::children_rect().

use std::collections::HashMap;

use crate::widget::WidgetNode;
use arbor_tui_primitives::layout::{
    sat_sub, Align, Direction, Justify, LayoutProps, Rect, RectOffset, Size, SizeCalc,
    SizeConstraint,
};
use arbor_tui_primitives::layout_error::LayoutError;
use arbor_tui_primitives::widget_id::{WidgetId, WidgetLayoutInfo};

/// Layout result type alias.
pub type LayoutResult = HashMap<WidgetId, WidgetLayoutInfo>;

// ── Pass 1: Bottom-up measure ──────────────────────────────────────

/// Measure the entire widget tree. Returns constraints keyed by WidgetId.
pub fn measure_tree(root: &WidgetNode, available: Size) -> HashMap<WidgetId, SizeConstraint> {
    let mut constraints = HashMap::new();
    measure_node(root, available, &mut constraints);
    constraints
}

fn measure_node(
    node: &WidgetNode,
    available: Size,
    out: &mut HashMap<WidgetId, SizeConstraint>,
) -> SizeConstraint {
    let props = node.layout_props();
    let inner = SizeCalc::content_available(available, props.padding, RectOffset::default());

    // Measure children first (bottom-up)
    for child in node.children() {
        let _ = measure_node(child, inner, out);
    }

    // Then measure self — widget sees children's constraints via `out`
    let constraint = node.measure_subtree(available, out);
    out.insert(node.id(), constraint);
    constraint
}

// ── Pass 2: Top-down layout ────────────────────────────────────────

/// Layout the entire widget tree. Returns positions for all widgets.
pub fn layout_tree(
    root_rect: Rect,
    root: &WidgetNode,
    constraints: &HashMap<WidgetId, SizeConstraint>,
) -> Result<HashMap<WidgetId, WidgetLayoutInfo>, LayoutError> {
    let mut widgets = HashMap::new();
    layout_node(root_rect, root, constraints, &mut widgets)?;
    Ok(widgets)
}

fn layout_node(
    rect: Rect,
    node: &WidgetNode,
    constraints: &HashMap<WidgetId, SizeConstraint>,
    out: &mut HashMap<WidgetId, WidgetLayoutInfo>,
) -> Result<(), LayoutError> {
    let props = node.layout_props();
    let content_rect = SizeCalc::content_rect(rect, props.padding);

    out.insert(
        node.id(),
        WidgetLayoutInfo {
            id: node.id(),
            outer_rect: rect,
            content_rect,
        },
    );

    let children = node.children();
    if !children.is_empty() {
        // Widget can reserve space for headers etc. via children_rect()
        let child_rect = node.children_rect(content_rect);
        layout_flex_children(child_rect, children, props, constraints, out)?;
    }

    Ok(())
}

/// Flex layout for a container's children.
fn layout_flex_children(
    container: Rect,
    children: &[WidgetNode],
    props: &LayoutProps,
    constraints: &HashMap<WidgetId, SizeConstraint>,
    out: &mut HashMap<WidgetId, WidgetLayoutInfo>,
) -> Result<(), LayoutError> {
    let is_column = props.direction == Direction::Column;
    let main_available = if is_column { container.h } else { container.w };

    #[derive(Clone)]
    struct ChildInfo {
        idx: usize,
        fixed_main: u16,
        flex: f32,
        margin_main: u16,
        margin_cross_start: u16,
        margin_cross_end: u16,
        cross_min: u16,
    }

    let mut infos: Vec<ChildInfo> = Vec::new();
    let mut fixed_total: u16 = 0;
    let mut flex_sum: f32 = 0.0;

    for (i, child) in children.iter().enumerate() {
        let cp = child.layout_props();
        let cc = constraints
            .get(&child.id())
            .copied()
            .ok_or(LayoutError::MissingConstraints(child.id()))?;
        let (margin_main, margin_cross_start, margin_cross_end, cross_min, fixed_main) =
            if is_column {
                (
                    cp.margin.vertical(),
                    cp.margin.left,
                    cp.margin.right,
                    cc.min_w + cp.margin.horizontal() + cp.padding.horizontal(),
                    cc.min_h + cp.margin.vertical() + cp.padding.vertical(),
                )
            } else {
                (
                    cp.margin.horizontal(),
                    cp.margin.top,
                    cp.margin.bottom,
                    cc.min_h + cp.margin.vertical() + cp.padding.vertical(),
                    cc.min_w + cp.margin.horizontal() + cp.padding.horizontal(),
                )
            };

        if cp.flex > 0.0 {
            flex_sum += cp.flex;
        }
        fixed_total += fixed_main;
        infos.push(ChildInfo {
            idx: i,
            fixed_main,
            flex: cp.flex,
            margin_main,
            margin_cross_start,
            margin_cross_end,
            cross_min,
        });
    }

    let free_space = main_available as i32 - fixed_total as i32;

    let mut final_mains: Vec<u16> = vec![0; children.len()];
    for info in &infos {
        let base = info.fixed_main as i32;
        if free_space >= 0 && info.flex > 0.0 && flex_sum > 0.0 {
            let extra = (free_space as f32 * info.flex / flex_sum) as i32;
            final_mains[info.idx] = (base + extra) as u16;
        } else if free_space < 0 && info.flex > 0.0 && flex_sum > 0.0 {
            let shrink = (free_space.abs() as f32 * info.flex / flex_sum) as i32;
            let constraint = constraints
                .get(&children[info.idx].id())
                .copied()
                .ok_or(LayoutError::MissingConstraints(children[info.idx].id()))?;
            let child_padding = children[info.idx].layout_props().padding;
            let min_content = if is_column {
                constraint.min_h
            } else {
                constraint.min_w
            };
            let min_pad = if is_column {
                child_padding.vertical()
            } else {
                child_padding.horizontal()
            };
            final_mains[info.idx] = (base - shrink).max((min_content + min_pad) as i32) as u16;
        } else {
            final_mains[info.idx] = base as u16;
        }
    }

    // Remainder distribution
    if free_space >= 0 {
        let total_allocated: u16 = final_mains.iter().sum();
        let remainder = sat_sub(main_available, total_allocated);
        let mut flex_indices: Vec<usize> = infos
            .iter()
            .filter(|i| i.flex > 0.0)
            .map(|i| i.idx)
            .collect();
        flex_indices.sort_by(|a, b| {
            children[*b]
                .layout_props()
                .flex
                .partial_cmp(&children[*a].layout_props().flex)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for i in 0..remainder as usize {
            if let Some(&idx) = flex_indices.get(i % flex_indices.len().max(1)) {
                final_mains[idx] += 1;
            }
        }
    }

    // Justify
    let total_used: u16 = final_mains.iter().sum();
    let main_gap = sat_sub(main_available, total_used);
    let (mut main_offset, gap) = match props.justify {
        Justify::Start => (0u16, 0u16),
        Justify::Center => (main_gap / 2, 0),
        Justify::End => (main_gap, 0),
        Justify::SpaceBetween if children.len() >= 2 => (0, main_gap / (children.len() as u16 - 1)),
        Justify::SpaceBetween => (0, 0),
    };

    let cross_available = if is_column { container.w } else { container.h };

    // Position each child
    for info in &infos {
        let child = &children[info.idx];

        let cross_size = match props.align {
            Align::Stretch => sat_sub(
                cross_available,
                info.margin_cross_start + info.margin_cross_end,
            )
            .max(1),
            _ => info
                .cross_min
                .saturating_sub(info.margin_cross_start + info.margin_cross_end)
                .max(1),
        };

        let cross_offset = match props.align {
            Align::Start => info.margin_cross_start,
            Align::Center => {
                let slack = sat_sub(
                    cross_available,
                    cross_size + info.margin_cross_start + info.margin_cross_end,
                );
                info.margin_cross_start + slack / 2
            }
            Align::End => {
                let slack = sat_sub(
                    cross_available,
                    cross_size + info.margin_cross_start + info.margin_cross_end,
                );
                info.margin_cross_start + slack
            }
            Align::Stretch => info.margin_cross_start,
        };

        let child_rect = if is_column {
            Rect::new(
                container.x + cross_offset,
                container.y + main_offset + info.margin_main / 2,
                cross_size,
                sat_sub(final_mains[info.idx], info.margin_main),
            )
        } else {
            Rect::new(
                container.x + main_offset + info.margin_main / 2,
                container.y + cross_offset,
                sat_sub(final_mains[info.idx], info.margin_main),
                cross_size,
            )
        };

        main_offset += final_mains[info.idx] + gap;
        layout_node(child_rect, child, constraints, out)?;
    }

    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Widget;
    use arbor_tui_primitives::layout::{
        Direction, LayoutProps, RectOffset, Size, SizeCalc, SizeConstraint,
    };
    use arbor_tui_primitives::text;
    use arbor_tui_primitives::widget_id::WidgetId;

    // Minimal test text widget
    struct TestText {
        id: WidgetId,
        props: LayoutProps,
        text: String,
    }
    impl Widget for TestText {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }
        fn measure(&self, _available: Size) -> SizeConstraint {
            let w = text::measure_width(&self.text);
            SizeConstraint::fixed(w.max(1), 1)
        }
        fn render(
            &self,
            _rect: Rect,
            _theme: &arbor_tui_render::theme::Theme,
        ) -> arbor_tui_render::screen::VirtualScreen {
            let mut s = arbor_tui_render::screen::VirtualScreen::new(
                text::measure_width(&self.text).max(1),
                1,
            );
            s.write_str(
                0,
                0,
                &self.text,
                Default::default(),
                Default::default(),
                Default::default(),
            );
            s
        }
    }

    fn make_text(id: u64, text: &str) -> WidgetNode {
        WidgetNode::new(TestText {
            id: WidgetId(id),
            props: LayoutProps::default(),
            text: text.to_string(),
        })
    }

    // Minimal box widget
    struct TestBox {
        id: WidgetId,
        props: LayoutProps,
        children: Vec<WidgetNode>,
    }
    impl Widget for TestBox {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }
        fn children(&self) -> &[WidgetNode] {
            &self.children
        }
        fn children_mut(&mut self) -> &mut [WidgetNode] {
            &mut self.children
        }
        fn is_transparent(&self) -> bool {
            true
        }
        fn measure_subtree(
            &self,
            _available: Size,
            child_constraints: &HashMap<WidgetId, SizeConstraint>,
        ) -> SizeConstraint {
            if self.children.is_empty() {
                return SizeConstraint::fixed(0, 0);
            }
            let _ =
                SizeCalc::content_available(_available, self.props.padding, RectOffset::default());
            let mut total_main: u16 = 0;
            let mut max_cross: u16 = 0;
            for child in &self.children {
                let cc = child_constraints
                    .get(&child.id())
                    .copied()
                    .unwrap_or(SizeConstraint::unbounded());
                let cp = child.layout_props();
                match self.props.direction {
                    Direction::Column => {
                        total_main += cc.min_h + cp.margin.vertical() + cp.padding.vertical();
                        max_cross = max_cross
                            .max(cc.min_w + cp.margin.horizontal() + cp.padding.horizontal());
                    }
                    Direction::Row => {
                        total_main += cc.min_w + cp.margin.horizontal() + cp.padding.horizontal();
                        max_cross =
                            max_cross.max(cc.min_h + cp.margin.vertical() + cp.padding.vertical());
                    }
                }
            }
            let (min_w, min_h) = match self.props.direction {
                Direction::Column => (max_cross, total_main),
                Direction::Row => (total_main, max_cross),
            };
            let outer = SizeCalc::outer_size(
                Size::new(min_w, min_h),
                self.props.padding,
                RectOffset::default(),
            );
            SizeConstraint {
                min_w: self.props.width.unwrap_or(outer.w).max(1),
                min_h: self.props.height.unwrap_or(outer.h).max(1),
                max_w: arbor_tui_primitives::layout::AxisConstraint::Fixed(_available.w),
                max_h: arbor_tui_primitives::layout::AxisConstraint::Fixed(_available.h),
            }
        }
    }

    fn make_box(id: u64, direction: Direction, children: Vec<WidgetNode>) -> WidgetNode {
        WidgetNode::new(TestBox {
            id: WidgetId(id),
            props: LayoutProps {
                direction,
                ..Default::default()
            },
            children,
        })
    }

    #[test]
    fn measure_single_text() {
        let text = make_text(1, "hello");
        let c = measure_tree(&text, Size::new(80, 24));
        let tc = c[&WidgetId(1)];
        assert_eq!(tc.min_w, 5);
        assert_eq!(tc.min_h, 1);
    }

    #[test]
    fn measure_column_box() {
        let box_w = make_box(
            10,
            Direction::Column,
            vec![make_text(1, "hello"), make_text(2, "world")],
        );
        let c = measure_tree(&box_w, Size::new(80, 24));
        let bc = c[&WidgetId(10)];
        assert!(bc.min_h >= 2);
        assert!(bc.min_w >= 5);
    }

    #[test]
    fn layout_column_positions_children() {
        let box_w = make_box(
            10,
            Direction::Column,
            vec![make_text(1, "hello"), make_text(2, "world")],
        );
        let c = measure_tree(&box_w, Size::new(80, 24));
        let result = layout_tree(Rect::new(0, 0, 80, 24), &box_w, &c).unwrap();
        let t1 = &result[&WidgetId(1)];
        let t2 = &result[&WidgetId(2)];
        assert!(t2.content_rect.y > t1.content_rect.y);
    }

    #[test]
    fn flex_child_gets_extra_space() {
        let flex_text = WidgetNode::new(TestText {
            id: WidgetId(2),
            props: LayoutProps {
                flex: 1.0,
                ..Default::default()
            },
            text: "flex".to_string(),
        });
        let box_w = WidgetNode::new(TestBox {
            id: WidgetId(10),
            props: LayoutProps {
                direction: Direction::Column,
                ..Default::default()
            },
            children: vec![make_text(1, "fixed"), flex_text],
        });
        let c = measure_tree(&box_w, Size::new(80, 24));
        let result = layout_tree(Rect::new(0, 0, 80, 24), &box_w, &c).unwrap();
        let flex_info = &result[&WidgetId(2)];
        assert!(flex_info.content_rect.h > 1);
    }
}
