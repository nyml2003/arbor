// Layout engine — two-pass flexbox layout.
// Pass 1: measure_tree (bottom-up)
// Pass 2: layout_tree (top-down, flex allocation with remainder distribution)

use std::collections::HashMap;

use crate::error::LayoutError;
use crate::layout::{
    Align, AxisConstraint, Direction, Justify, LayoutProps, Rect, RectOffset, Size, SizeCalc, SizeConstraint, sat_sub,
};
use crate::text;
use crate::widget::{WidgetId, WidgetNode};
use crate::widget::TextWidget;

/// Layout info for a single widget — two rects for render + spacing.
#[derive(Clone, Debug)]
pub struct WidgetLayoutInfo {
    pub id: WidgetId,
    /// Outer rect including margin.
    pub outer_rect: Rect,
    /// Inner content rect (minus padding) — where render() draws.
    pub content_rect: Rect,
}

/// Layout result for all widgets in the tree.
#[derive(Clone, Debug)]
pub struct LayoutResult {
    pub widgets: HashMap<WidgetId, WidgetLayoutInfo>,
}

// ── Pass 1: Bottom-up measure ───────────────────────────────────

/// Measure the entire widget tree. Returns constraints keyed by WidgetId.
pub fn measure_tree(node: &WidgetNode, available: Size) -> HashMap<WidgetId, SizeConstraint> {
    let mut constraints = HashMap::new();
    measure_node(node, available, &mut constraints);
    constraints
}

fn measure_node(node: &WidgetNode, available: Size, out: &mut HashMap<WidgetId, SizeConstraint>) -> SizeConstraint {
    let constraint = match node {
        WidgetNode::Text(w) => measure_text(w, available),
        WidgetNode::Input(w) => measure_input(w, available),
        WidgetNode::Button(w) => measure_button(w, available),
        WidgetNode::Box(w) => measure_box(w, available, out),
        WidgetNode::List(w) => measure_generic(&w.props, available),
        WidgetNode::Table(w) => measure_generic(&w.props, available),
        WidgetNode::Tabs(w) => measure_tabs(w, available, out),
        WidgetNode::ScrollView(w) => measure_scrollview(w, available, out),
    };
    out.insert(node.id(), constraint);
    constraint
}

fn measure_text(w: &TextWidget, available: Size) -> SizeConstraint {
    let avail = SizeCalc::content_available(available, w.props.padding, w.props.margin);
    let text_content = w.text.get();
    let expanded = text::expand_tabs(&text_content);
    let text_w = text::measure_width(&expanded);

    match w.wrap {
        text::WrapStrategy::None => {
            let w = w.props.width.unwrap_or(text_w).max(1);
            // Count newlines + 1 for the first line
            let h = (expanded.lines().count() as u16).max(1);
            SizeConstraint::fixed(w, h)
        }
        _ => {
            let max_w = if avail.w > 0 { avail.w } else { text_w };
            let lines = text::wrap_lines(&expanded, max_w, w.wrap);
            let line_count = lines.len() as u16;
            let min_w = 1u16;
            let max_w = w.props.width.unwrap_or(avail.w);
            SizeConstraint {
                min_w,
                min_h: 1,
                max_w: AxisConstraint::Fixed(max_w.max(1)),
                max_h: AxisConstraint::Fixed(line_count.max(1)),
            }
        }
    }
}

fn measure_input(w: &crate::widget::InputWidget, available: Size) -> SizeConstraint {
    if let Some(w_val) = w.props.width {
        SizeConstraint::fixed(w_val, 1)
    } else {
        let avail = SizeCalc::content_available(available, w.props.padding, w.props.margin);
        SizeConstraint {
            min_w: 1,
            min_h: 1,
            max_w: AxisConstraint::Fixed(avail.w.max(1)),
            max_h: AxisConstraint::Fixed(1),
        }
    }
}

fn measure_button(w: &crate::widget::ButtonWidget, _available: Size) -> SizeConstraint {
    if let (Some(w_val), Some(h_val)) = (w.props.width, w.props.height) {
        SizeConstraint::fixed(w_val, h_val)
    } else {
        let label = w.label.get();
        let label_w = text::measure_width(&label) + 4; // 2 padding each side
        let bw = w.props.width.unwrap_or(label_w).max(1);
        let bh = w.props.height.unwrap_or(1);
        SizeConstraint::fixed(bw, bh)
    }
}

fn measure_generic(props: &LayoutProps, available: Size) -> SizeConstraint {
    if let (Some(w), Some(h)) = (props.width, props.height) {
        SizeConstraint::fixed(w, h)
    } else {
        let avail = SizeCalc::content_available(available, props.padding, props.margin);
        SizeConstraint::bounded(avail)
    }
}

fn measure_box(
    w: &crate::widget::BoxWidget,
    available: Size,
    out: &mut HashMap<WidgetId, SizeConstraint>,
) -> SizeConstraint {
    if w.children.is_empty() {
        return SizeConstraint::fixed(
            w.props.width.unwrap_or(0),
            w.props.height.unwrap_or(0),
        );
    }

    let inner = SizeCalc::content_available(available, w.props.padding, RectOffset::default());
    let mut total_main: u16 = 0;
    let mut max_cross: u16 = 0;

    for child in &w.children {
        let child_constraint = measure_node(child, inner, out);
        let child_props = child.layout_props();
        let child_margin = child_props.margin;

        match w.props.direction {
            Direction::Column => {
                total_main += child_constraint.min_h + child_margin.vertical();
                max_cross = max_cross.max(child_constraint.min_w + child_margin.horizontal());
            }
            Direction::Row => {
                total_main += child_constraint.min_w + child_margin.horizontal();
                max_cross = max_cross.max(child_constraint.min_h + child_margin.vertical());
            }
        }
    }

    let (min_w, min_h) = match w.props.direction {
        Direction::Column => (max_cross, total_main),
        Direction::Row => (total_main, max_cross),
    };

    let outer = SizeCalc::outer_size(Size::new(min_w, min_h), w.props.padding, RectOffset::default());
    SizeConstraint {
        min_w: w.props.width.unwrap_or(outer.w).max(1),
        min_h: w.props.height.unwrap_or(outer.h).max(1),
        max_w: AxisConstraint::Fixed(available.w),
        max_h: AxisConstraint::Fixed(available.h),
    }
}

fn measure_tabs(
    w: &crate::widget::TabsWidget,
    available: Size,
    out: &mut HashMap<WidgetId, SizeConstraint>,
) -> SizeConstraint {
    // Measure all tab contents, take the max
    let inner = SizeCalc::content_available(available, w.props.padding, RectOffset::default());
    let mut max_w: u16 = 0;
    let mut max_h: u16 = 0;

    // Tab header row (~1 line)
    let header_h: u16 = 1;

    for tab in &w.tabs {
        let child_c = measure_node(&tab.content, inner, out);
        max_w = max_w.max(child_c.min_w);
        max_h = max_h.max(child_c.min_h);
    }

    SizeConstraint {
        min_w: w.props.width.unwrap_or(max_w).max(1),
        min_h: w.props.height.unwrap_or(header_h + max_h).max(1),
        max_w: AxisConstraint::Fixed(available.w),
        max_h: AxisConstraint::Fixed(available.h),
    }
}

fn measure_scrollview(
    w: &crate::widget::ScrollViewWidget,
    available: Size,
    out: &mut HashMap<WidgetId, SizeConstraint>,
) -> SizeConstraint {
    // Pass viewport size as available to child — child can be larger
    let inner = SizeCalc::content_available(available, w.props.padding, RectOffset::default());
    let _ = measure_node(&w.child, inner, out); // side effect: populates out
    // ScrollView takes whatever space is available
    SizeConstraint::bounded(available)
}

// ── Pass 2: Top-down layout ─────────────────────────────────────

/// Layout the entire widget tree. Returns positions for all widgets.
pub fn layout_tree(
    root_rect: Rect,
    node: &WidgetNode,
    constraints: &HashMap<WidgetId, SizeConstraint>,
) -> Result<LayoutResult, LayoutError> {
    let mut widgets = HashMap::new();
    layout_node(root_rect, node, constraints, &mut widgets)?;
    Ok(LayoutResult { widgets })
}

fn layout_node(
    rect: Rect,
    node: &WidgetNode,
    constraints: &HashMap<WidgetId, SizeConstraint>,
    out: &mut HashMap<WidgetId, WidgetLayoutInfo>,
) -> Result<(), LayoutError> {
    let props = node.layout_props();
    let content_rect = SizeCalc::content_rect(rect, props.padding);

    out.insert(node.id(), WidgetLayoutInfo {
        id: node.id(),
        outer_rect: rect,
        content_rect,
    });

    match node {
        WidgetNode::Box(w) if !w.children.is_empty() => {
            layout_flex_children(content_rect, &w.children, &w.props, constraints, out)?;
        }
        WidgetNode::Tabs(w) => {
            let body_rect = Rect::new(
                content_rect.x,
                content_rect.y + 1,
                content_rect.w,
                sat_sub(content_rect.h, 1),
            );
            if w.active < w.tabs.len() {
                layout_node(body_rect, &w.tabs[w.active].content, constraints, out)?;
            }
        }
        WidgetNode::ScrollView(w) => {
            let child_constraint = constraints.get(&w.child.id()).copied();
            if let Some(cc) = child_constraint {
                let child_rect = Rect::new(
                    content_rect.x,
                    content_rect.y,
                    content_rect.w.max(cc.min_w),
                    content_rect.h.max(cc.min_h),
                );
                layout_node(child_rect, &w.child, constraints, out)?;
            }
        }
        _ => {} // Leaf nodes (Text, Input, Button) — no children to layout
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
        fixed_main: u16, //固有主轴尺寸(含margin)
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
        let cc = constraints.get(&child.id()).copied()
            .ok_or(LayoutError::MissingConstraints(child.id()))?;
        let (margin_main, margin_cross_start, margin_cross_end, cross_min, fixed_main) = if is_column {
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

    // Calculate final main-axis sizes
    let mut final_mains: Vec<u16> = vec![0; children.len()];
    for info in &infos {
        let base = info.fixed_main as i32;
        if free_space >= 0 && info.flex > 0.0 && flex_sum > 0.0 {
            let extra = (free_space as f32 * info.flex / flex_sum) as i32;
            final_mains[info.idx] = (base + extra) as u16;
        } else if free_space < 0 && info.flex > 0.0 && flex_sum > 0.0 {
            let shrink = (free_space.abs() as f32 * info.flex / flex_sum) as i32;
            let constraint = constraints.get(&children[info.idx].id()).copied()
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
        // Give to flex children in descending flex order
        let mut flex_indices: Vec<usize> = infos.iter()
            .filter(|i| i.flex > 0.0)
            .map(|i| i.idx)
            .collect();
        flex_indices.sort_by(|a, b| {
            children[*b].layout_props().flex.partial_cmp(&children[*a].layout_props().flex)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for i in 0..remainder as usize {
            if let Some(&idx) = flex_indices.get(i % flex_indices.len().max(1)) {
                final_mains[idx] += 1;
            }
        }
    }

    // Justify — calculate main-axis offset
    let total_used: u16 = final_mains.iter().sum();
    let main_gap = sat_sub(main_available, total_used);
    let (mut main_offset, gap) = match props.justify {
        Justify::Start => (0u16, 0u16),
        Justify::Center => (main_gap / 2, 0),
        Justify::End => (main_gap, 0),
        Justify::SpaceBetween if children.len() >= 2 => (0, main_gap / (children.len() as u16 - 1)),
        Justify::SpaceBetween => (0, 0),
    };

    // Cross-axis available size
    let cross_available = if is_column { container.w } else { container.h };

    // Position each child
    for info in &infos {
        let child = &children[info.idx];

        let cross_size = match props.align {
            Align::Stretch => sat_sub(cross_available, info.margin_cross_start + info.margin_cross_end).max(1),
            _ => info.cross_min.saturating_sub(info.margin_cross_start + info.margin_cross_end).max(1),
        };

        let cross_offset = match props.align {
            Align::Start => info.margin_cross_start,
            Align::Center => {
                let slack = sat_sub(cross_available, cross_size + info.margin_cross_start + info.margin_cross_end);
                info.margin_cross_start + slack / 2
            }
            Align::End => {
                let slack = sat_sub(cross_available, cross_size + info.margin_cross_start + info.margin_cross_end);
                info.margin_cross_start + slack
            }
            Align::Stretch => info.margin_cross_start,
        };

        let child_rect = if is_column {
            Rect::new(
                container.x + cross_offset,
                container.y + main_offset + info.margin_main / 2, // top margin
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutProps;
    use crate::widget::{BoxWidget, TextWidget};

    fn make_text(id: u64, text: &str) -> WidgetNode {
        WidgetNode::Text(TextWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            text: crate::signal::ReadSignal::constant(text.to_string()),
            style: crate::signal::ReadSignal::constant(crate::widget::TextStyle::default()),
            wrap: text::WrapStrategy::None,
            truncate: text::TruncateStrategy::End,
        })
    }

    fn make_box(id: u64, direction: Direction, children: Vec<WidgetNode>) -> WidgetNode {
        WidgetNode::Box(BoxWidget {
            id: WidgetId(id),
            props: LayoutProps { direction, ..Default::default() },
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
        let box_w = make_box(10, Direction::Column, vec![
            make_text(1, "hello"),
            make_text(2, "world"),
        ]);
        let c = measure_tree(&box_w, Size::new(80, 24));
        let bc = c[&WidgetId(10)];
        // Column: min_h = 1+1 = 2, min_w = max(5,5) = 5
        assert!(bc.min_h >= 2);
        assert!(bc.min_w >= 5);
    }

    #[test]
    fn layout_column_positions_children() {
        let box_w = make_box(10, Direction::Column, vec![
            make_text(1, "hello"),
            make_text(2, "world"),
        ]);
        let c = measure_tree(&box_w, Size::new(80, 24));
        let result = layout_tree(Rect::new(0, 0, 80, 24), &box_w, &c).unwrap();

        let t1 = &result.widgets[&WidgetId(1)];
        let t2 = &result.widgets[&WidgetId(2)];
        // Second child should be below the first
        assert!(t2.content_rect.y > t1.content_rect.y);
    }

    #[test]
    fn flex_child_gets_extra_space() {
        let mut flex_props = LayoutProps::default();
        flex_props.flex = 1.0;

        let flex_text = WidgetNode::Text(TextWidget {
            id: WidgetId(2),
            props: flex_props,
            text: crate::signal::ReadSignal::constant("flex".to_string()),
            style: crate::signal::ReadSignal::constant(crate::widget::TextStyle::default()),
            wrap: text::WrapStrategy::None,
            truncate: text::TruncateStrategy::End,
        });

        let box_w = WidgetNode::Box(BoxWidget {
            id: WidgetId(10),
            props: LayoutProps { direction: Direction::Column, ..Default::default() },
            children: vec![
                make_text(1, "fixed"),
                flex_text,
            ],
        });

        let c = measure_tree(&box_w, Size::new(80, 24));
        let result = layout_tree(Rect::new(0, 0, 80, 24), &box_w, &c).unwrap();

        // flex child should be taller than 1 (text height)
        let flex_info = &result.widgets[&WidgetId(2)];
        assert!(flex_info.content_rect.h > 1);
    }
}
