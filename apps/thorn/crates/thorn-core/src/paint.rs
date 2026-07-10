use crate::{HostKind, HostNode, LayoutNode, Rect, Size, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PaintStyle {
    pub foreground: Option<PaintColor>,
    pub background: Option<PaintColor>,
    pub attrs: PaintAttrs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaintAttrs {
    bits: u8,
}

impl PaintAttrs {
    pub const BOLD: Self = Self { bits: 0b0000_0001 };
    pub const UNDERLINE: Self = Self { bits: 0b0000_0010 };
    pub const REVERSED: Self = Self { bits: 0b0000_0100 };

    pub const fn empty() -> Self {
        Self { bits: 0 }
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.bits & other.bits) == other.bits
    }
}

impl Default for PaintAttrs {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaintPrimitive {
    FillRect {
        rect: Rect,
        style: PaintStyle,
    },
    TextRun {
        x: u16,
        y: u16,
        text: String,
    },
    Border {
        rect: Rect,
        style: PaintStyle,
    },
    Cursor {
        x: u16,
        y: u16,
    },
    Clip {
        rect: Rect,
        children: Vec<PaintPrimitive>,
    },
    Layer {
        z_index: i16,
        children: Vec<PaintPrimitive>,
    },
}

pub fn paint_tree<Action>(host: &HostNode<Action>, layout: &[LayoutNode]) -> Vec<PaintPrimitive> {
    let mut paint = Vec::new();
    paint_node(host, layout, 0, 0, ClipRegion::Unbounded, &mut paint);
    paint
}

pub fn paint_tree_with_theme<Action>(
    host: &HostNode<Action>,
    layout: &[LayoutNode],
    theme: &Theme,
    size: Size,
) -> Vec<PaintPrimitive> {
    let mut paint = Vec::new();
    if theme.canvas != PaintStyle::default() && size.width > 0 && size.height > 0 {
        paint.push(PaintPrimitive::FillRect {
            rect: Rect::new(0, 0, size.width, size.height),
            style: theme.canvas,
        });
    }
    paint_node(host, layout, 0, 0, ClipRegion::Unbounded, &mut paint);
    paint
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipRegion {
    Unbounded,
    Rect(Rect),
    Empty,
}

fn paint_node<Action>(
    host: &HostNode<Action>,
    layout: &[LayoutNode],
    translate_x: i32,
    translate_y: i32,
    inherited_clip: ClipRegion,
    paint: &mut Vec<PaintPrimitive>,
) {
    let Some(layout_node) = layout.iter().find(|node| node.host_id == host.id) else {
        return;
    };
    let mut translate_x = translate_x;
    let mut translate_y = translate_y;
    let mut inherited_clip = inherited_clip;

    if let Some(offset) = host.layout_style.scroll_offset {
        translate_x -= i32::from(offset.x);
        translate_y -= i32::from(offset.y);
        inherited_clip = intersect_clip_regions(inherited_clip, layout_node.clip_rect);
        if inherited_clip == ClipRegion::Empty {
            return;
        }
    }

    match host.kind {
        HostKind::Text => {
            if let Some(text) = host.text.as_ref() {
                if let Some((x, y, text)) =
                    clipped_text_run(text, layout_node, translate_x, translate_y, inherited_clip)
                {
                    paint.push(PaintPrimitive::TextRun { x, y, text });
                }
            }
        }
        HostKind::Clip { .. } => {
            let mut children = Vec::new();
            for child in &host.children {
                paint_node(
                    child,
                    layout,
                    translate_x,
                    translate_y,
                    inherited_clip,
                    &mut children,
                );
            }
            if let Some(rect) = translate_rect(layout_node.clip_rect, translate_x, translate_y) {
                paint.push(PaintPrimitive::Clip { rect, children });
            }
        }
        HostKind::Layer { z_index, .. } => {
            let mut children = Vec::new();
            for child in &host.children {
                paint_node(
                    child,
                    layout,
                    translate_x,
                    translate_y,
                    inherited_clip,
                    &mut children,
                );
            }
            paint.push(PaintPrimitive::Layer { z_index, children });
        }
        HostKind::Border { style, .. } => {
            if let Some(rect) = translate_rect(layout_node.rect, translate_x, translate_y) {
                paint.push(PaintPrimitive::Border { rect, style });
            }
            for child in &host.children {
                paint_node(
                    child,
                    layout,
                    translate_x,
                    translate_y,
                    inherited_clip,
                    paint,
                );
            }
        }
        HostKind::View { .. } | HostKind::ScrollView { .. } => {
            for child in &host.children {
                paint_node(
                    child,
                    layout,
                    translate_x,
                    translate_y,
                    inherited_clip,
                    paint,
                );
            }
        }
    }
}

fn clipped_text_run(
    text: &str,
    layout_node: &LayoutNode,
    translate_x: i32,
    translate_y: i32,
    inherited_clip: ClipRegion,
) -> Option<(u16, u16, String)> {
    let logical_rect = if translate_x != 0 || translate_y != 0 {
        layout_node.content_rect
    } else {
        layout_node.rect
    };
    let left = i32::from(logical_rect.x) + translate_x;
    let top = i32::from(logical_rect.y) + translate_y;
    let right = left + i32::from(logical_rect.width);
    let bottom = top + i32::from(logical_rect.height);

    let clip = match inherited_clip {
        ClipRegion::Unbounded => layout_node.clip_rect,
        ClipRegion::Rect(rect) => rect,
        ClipRegion::Empty => return None,
    };
    let clip_left = i32::from(clip.x);
    let clip_top = i32::from(clip.y);
    let clip_right = clip_left + i32::from(clip.width);
    let clip_bottom = clip_top + i32::from(clip.height);

    if top >= clip_bottom || bottom <= clip_top || left >= clip_right || right <= clip_left {
        return None;
    }

    let visible_x = left.max(clip_left);
    let skip = (visible_x - left).max(0) as usize;
    let take = (clip_right.min(right) - visible_x).max(0) as usize;
    if take == 0 {
        return None;
    }

    let text = text.chars().skip(skip).take(take).collect::<String>();
    if text.is_empty() {
        return None;
    }

    Some((visible_x as u16, top.max(clip_top) as u16, text))
}

fn intersect_clip_regions(current: ClipRegion, next: Rect) -> ClipRegion {
    if next.width == 0 || next.height == 0 {
        return ClipRegion::Empty;
    }
    match current {
        ClipRegion::Unbounded => ClipRegion::Rect(next),
        ClipRegion::Rect(current) => intersect_rects(current, next)
            .map(ClipRegion::Rect)
            .unwrap_or(ClipRegion::Empty),
        ClipRegion::Empty => ClipRegion::Empty,
    }
}

fn intersect_rects(a: Rect, b: Rect) -> Option<Rect> {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let y2 =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    (x2 > x1 && y2 > y1).then(|| Rect::new(x1, y1, x2 - x1, y2 - y1))
}

fn translate_rect(rect: Rect, translate_x: i32, translate_y: i32) -> Option<Rect> {
    let x = i32::from(rect.x) + translate_x;
    let y = i32::from(rect.y) + translate_y;
    if x + i32::from(rect.width) <= 0 || y + i32::from(rect.height) <= 0 {
        return None;
    }

    Some(Rect::new(
        x.max(0).min(i32::from(u16::MAX)) as u16,
        y.max(0).min(i32::from(u16::MAX)) as u16,
        rect.width,
        rect.height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        border, clip, column, layer, layout_tree, lower_element, scroll_view, text, view,
        PaintColor, ScrollOffset, Size,
    };

    #[test]
    fn text_paint_produces_text_run() {
        let element = text::<()>("hello");
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 2));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "hello".to_string(),
            }]
        );
    }

    #[test]
    fn paint_primitives_are_backend_independent() {
        let primitives = vec![
            PaintPrimitive::FillRect {
                rect: Rect::new(0, 0, 4, 2),
                style: PaintStyle::default(),
            },
            PaintPrimitive::Border {
                rect: Rect::new(0, 0, 4, 2),
                style: PaintStyle::default(),
            },
            PaintPrimitive::Cursor { x: 1, y: 1 },
            PaintPrimitive::Clip {
                rect: Rect::new(0, 0, 2, 1),
                children: vec![PaintPrimitive::TextRun {
                    x: 0,
                    y: 0,
                    text: "hello".to_string(),
                }],
            },
            PaintPrimitive::Layer {
                z_index: 1,
                children: Vec::new(),
            },
        ];

        assert_eq!(primitives.len(), 5);
    }

    #[test]
    fn themed_paint_tree_prepends_canvas_fill_rect() {
        let element = text::<()>("hello");
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(5, 1));
        let theme = Theme::new(PaintStyle {
            background: Some(PaintColor::Indexed(6)),
            ..PaintStyle::default()
        });

        let paint = paint_tree_with_theme(&host, &layout, &theme, Size::new(5, 1));

        assert_eq!(
            paint.first(),
            Some(&PaintPrimitive::FillRect {
                rect: Rect::new(0, 0, 5, 1),
                style: theme.canvas,
            })
        );
    }

    #[test]
    fn scrolled_viewport_paints_later_logical_text_runs() {
        let element = view((column((text::<()>("a"), text::<()>("b"), text::<()>("c"))),))
            .scroll_offset(ScrollOffset::new(0, 1));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(3, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "b".to_string(),
            }]
        );
    }

    #[test]
    fn horizontal_scroll_offset_clips_visible_text_window() {
        let element = view((text::<()>("hello"),)).scroll_offset(ScrollOffset::new(2, 0));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(3, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "llo".to_string(),
            }]
        );
    }

    #[test]
    fn nested_viewports_do_not_paint_when_parent_and_child_clips_do_not_intersect() {
        let element = view((column((
            text::<()>("a"),
            view((text::<()>("b"),)).scroll_offset(ScrollOffset::new(0, 0)),
        )),))
        .scroll_offset(ScrollOffset::new(0, 0));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(1, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "a".to_string(),
            }]
        );
    }

    #[test]
    fn nested_viewports_still_paint_when_parent_and_child_clips_intersect() {
        let element = view((column((
            text::<()>("a"),
            view((text::<()>("b"),)).scroll_offset(ScrollOffset::new(0, 0)),
        )),))
        .scroll_offset(ScrollOffset::new(0, 0));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(1, 2));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![
                PaintPrimitive::TextRun {
                    x: 0,
                    y: 0,
                    text: "a".to_string(),
                },
                PaintPrimitive::TextRun {
                    x: 0,
                    y: 1,
                    text: "b".to_string(),
                },
            ]
        );
    }

    #[test]
    fn clip_helper_lowers_to_clip_primitive() {
        let element = clip((text::<()>("hello"),)).fixed_size(Size::new(3, 1));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(3, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::Clip {
                rect: Rect::new(0, 0, 3, 1),
                children: vec![PaintPrimitive::TextRun {
                    x: 0,
                    y: 0,
                    text: "hel".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn layer_helper_lowers_to_layer_primitive_with_stable_z_index() {
        let element = layer(10, (text::<()>("top"),));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(3, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::Layer {
                z_index: 10,
                children: vec![PaintPrimitive::TextRun {
                    x: 0,
                    y: 0,
                    text: "top".to_string(),
                }],
            }]
        );
    }

    #[test]
    fn border_helper_lowers_to_border_primitive_and_children() {
        let element = border((text::<()>("ok"),)).fixed_size(Size::new(4, 3));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(4, 3));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![
                PaintPrimitive::Border {
                    rect: Rect::new(0, 0, 4, 3),
                    style: PaintStyle::default(),
                },
                PaintPrimitive::TextRun {
                    x: 1,
                    y: 1,
                    text: "ok".to_string(),
                },
            ]
        );
    }

    #[test]
    fn scroll_view_helper_reuses_existing_scroll_semantics() {
        let element = scroll_view((column((text::<()>("a"), text::<()>("b"), text::<()>("c"))),))
            .scroll_offset(ScrollOffset::new(0, 1));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(1, 1));
        let paint = paint_tree(&host, &layout);

        assert_eq!(
            paint,
            vec![PaintPrimitive::TextRun {
                x: 0,
                y: 0,
                text: "b".to_string(),
            }]
        );
        assert_eq!(layout.len(), 4);
    }
}
