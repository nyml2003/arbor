use crate::{Axis, HostKind, HostNode, HostNodeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollOffset {
    pub x: u16,
    pub y: u16,
}

impl ScrollOffset {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl Padding {
    pub const fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(value: u16) -> Self {
        Self::new(value, value, value, value)
    }

    pub const fn symmetric(vertical: u16, horizontal: u16) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }

    pub const fn horizontal(self) -> u16 {
        self.left.saturating_add(self.right)
    }

    pub const fn vertical(self) -> u16 {
        self.top.saturating_add(self.bottom)
    }

    pub fn inset_rect(self, rect: Rect) -> Rect {
        let horizontal = self.horizontal();
        let vertical = self.vertical();
        let width = rect.width.saturating_sub(horizontal);
        let height = rect.height.saturating_sub(vertical);
        Rect::new(
            rect.x.saturating_add(self.left.min(rect.width)),
            rect.y.saturating_add(self.top.min(rect.height)),
            width,
            height,
        )
    }
}

impl From<u16> for Padding {
    fn from(value: u16) -> Self {
        Self::all(value)
    }
}

pub type Margin = Padding;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainAxisAlignment {
    #[default]
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAxisAlignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutStyle {
    pub gap: u16,
    pub padding: Padding,
    pub margin: Margin,
    pub fixed_size: Option<Size>,
    pub min_size: Option<Size>,
    pub flex_grow: u16,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub scroll_offset: Option<ScrollOffset>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConstraints {
    pub min: Size,
    pub max: Size,
}

impl LayoutConstraints {
    pub const fn new(min: Size, max: Size) -> Self {
        Self { min, max }
    }

    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::new(0, 0),
            max,
        }
    }

    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    pub fn clamp(self, size: Size) -> Size {
        Size::new(
            size.width.max(self.min.width).min(self.max.width),
            size.height.max(self.min.height).min(self.max.height),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendMetrics {
    pub line_height: u16,
    pub tab_width: u16,
    pub wide_char_width: u16,
    pub baseline: Option<u16>,
}

impl Default for BackendMetrics {
    fn default() -> Self {
        Self {
            line_height: 1,
            tab_width: 4,
            wide_char_width: 2,
            baseline: Some(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextMetrics {
    pub line_height: u16,
    pub baseline: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutOverflow {
    pub horizontal: bool,
    pub vertical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutNode {
    pub host_id: HostNodeId,
    pub rect: Rect,
    pub measured_size: Size,
    pub content_rect: Rect,
    pub clip_rect: Rect,
    pub overflow: LayoutOverflow,
    pub text_metrics: Option<TextMetrics>,
}

pub fn layout_tree<Action>(host: &HostNode<Action>, size: Size) -> Vec<LayoutNode> {
    layout_tree_with_metrics(
        host,
        LayoutConstraints::tight(size),
        &BackendMetrics::default(),
    )
}

pub fn text_display_width(text: &str) -> u16 {
    measure_text_width(text, &BackendMetrics::default())
}

pub fn layout_tree_with_metrics<Action>(
    host: &HostNode<Action>,
    constraints: LayoutConstraints,
    metrics: &BackendMetrics,
) -> Vec<LayoutNode> {
    let mut layout = Vec::new();
    let root_size = constraints.clamp(measure_host(host, metrics));
    let root_rect = Rect::new(0, 0, root_size.width, root_size.height);
    layout_node(host, root_rect, root_rect, metrics, &mut layout);
    layout
}

fn layout_node<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    clip_rect: Rect,
    metrics: &BackendMetrics,
    layout: &mut Vec<LayoutNode>,
) {
    let (measured_size, content_rect, resolved_clip_rect, overflow, text_metrics) =
        layout_metadata(host, rect, clip_rect, metrics);
    layout.push(LayoutNode {
        host_id: host.id,
        rect,
        measured_size,
        content_rect,
        clip_rect: resolved_clip_rect,
        overflow,
        text_metrics,
    });
    match host.kind {
        HostKind::Layer { .. } => {
            layout_layer_children(host, content_rect, resolved_clip_rect, metrics, layout)
        }
        _ => match container_axis(host.kind) {
            None => {}
            Some(Axis::Vertical) => {
                layout_vertical_children(host, content_rect, resolved_clip_rect, metrics, layout)
            }
            Some(Axis::Horizontal) => {
                layout_horizontal_children(host, content_rect, resolved_clip_rect, metrics, layout)
            }
        },
    }
}

fn layout_layer_children<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    clip_rect: Rect,
    metrics: &BackendMetrics,
    layout: &mut Vec<LayoutNode>,
) {
    for child in &host.children {
        layout_node(child, rect, clip_rect, metrics, layout);
    }
}

fn layout_vertical_children<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    clip_rect: Rect,
    metrics: &BackendMetrics,
    layout: &mut Vec<LayoutNode>,
) {
    let scroll_enabled = host.layout_style.scroll_offset.is_some();
    let available_outer_heights = distribute_flex_main_axis(
        host.children.iter(),
        rect.height,
        host.layout_style.gap,
        |child| outer_block_height(child, metrics),
    );
    let total_outer_height = measure_main_axis_extent(
        available_outer_heights.iter().copied(),
        host.layout_style.gap,
    );
    let mut y = rect.y.saturating_add(main_axis_alignment_offset(
        host.layout_style.main_axis_alignment,
        rect.height,
        total_outer_height,
    ));
    let bottom = rect.y.saturating_add(rect.height);
    for (index, child) in host.children.iter().enumerate() {
        let Some(&assigned_outer_height) = available_outer_heights.get(index) else {
            break;
        };
        if assigned_outer_height == 0 || (!scroll_enabled && y >= bottom) {
            break;
        }
        let margin = child.layout_style.margin;
        let available_outer_height = if scroll_enabled {
            assigned_outer_height
        } else {
            bottom.saturating_sub(y)
        };
        let child_outer_height = if scroll_enabled {
            assigned_outer_height
        } else {
            assigned_outer_height.min(available_outer_height)
        };
        let available_inner_width = rect.width.saturating_sub(margin.horizontal());
        let child_height = if child.layout_style.flex_grow > 0 {
            child_outer_height.saturating_sub(margin.vertical())
        } else {
            intrinsic_block_height(child, metrics)
                .min(child_outer_height.saturating_sub(margin.vertical()))
        };
        let child_width = child_cross_axis_width(
            child,
            available_inner_width,
            metrics,
            host.layout_style.cross_axis_alignment,
        );
        let child_outer_width = child_width.saturating_add(margin.horizontal());
        let child_x = rect.x.saturating_add(cross_axis_alignment_offset(
            host.layout_style.cross_axis_alignment,
            rect.width,
            child_outer_width,
        ));
        layout_node(
            child,
            Rect::new(
                child_x.saturating_add(margin.left.min(child_outer_width)),
                y.saturating_add(margin.top.min(child_outer_height)),
                child_width,
                child_height,
            ),
            clip_rect,
            metrics,
            layout,
        );
        y = y.saturating_add(child_outer_height);
        if index + 1 < host.children.len() {
            y = y.saturating_add(host.layout_style.gap);
        }
    }
}

fn layout_horizontal_children<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    clip_rect: Rect,
    metrics: &BackendMetrics,
    layout: &mut Vec<LayoutNode>,
) {
    let scroll_enabled = host.layout_style.scroll_offset.is_some();
    let available_outer_widths = distribute_flex_main_axis(
        host.children.iter(),
        rect.width,
        host.layout_style.gap,
        |child| outer_inline_width(child, metrics),
    );
    let total_outer_width = measure_main_axis_extent(
        available_outer_widths.iter().copied(),
        host.layout_style.gap,
    );
    let mut x = rect.x.saturating_add(main_axis_alignment_offset(
        host.layout_style.main_axis_alignment,
        rect.width,
        total_outer_width,
    ));
    let right = rect.x.saturating_add(rect.width);
    for (index, child) in host.children.iter().enumerate() {
        let Some(&assigned_outer_width) = available_outer_widths.get(index) else {
            break;
        };
        if assigned_outer_width == 0 || (!scroll_enabled && x >= right) {
            break;
        }
        let margin = child.layout_style.margin;
        let available_outer_width = if scroll_enabled {
            assigned_outer_width
        } else {
            right.saturating_sub(x)
        };
        let child_outer_width = if scroll_enabled {
            assigned_outer_width
        } else {
            assigned_outer_width.min(available_outer_width)
        };
        let available_inner_height = rect.height.saturating_sub(margin.vertical());
        let child_width = if child.layout_style.flex_grow > 0 {
            child_outer_width.saturating_sub(margin.horizontal())
        } else {
            intrinsic_inline_width(child, metrics)
                .min(child_outer_width.saturating_sub(margin.horizontal()))
        };
        let child_height = child_cross_axis_height(
            child,
            available_inner_height,
            metrics,
            host.layout_style.cross_axis_alignment,
        );
        let child_outer_height = child_height.saturating_add(margin.vertical());
        let child_y = rect.y.saturating_add(cross_axis_alignment_offset(
            host.layout_style.cross_axis_alignment,
            rect.height,
            child_outer_height,
        ));
        layout_node(
            child,
            Rect::new(
                x.saturating_add(margin.left.min(child_outer_width)),
                child_y.saturating_add(margin.top.min(child_outer_height)),
                child_width,
                child_height,
            ),
            clip_rect,
            metrics,
            layout,
        );
        x = x.saturating_add(child_outer_width);
        if index + 1 < host.children.len() {
            x = x.saturating_add(host.layout_style.gap);
        }
    }
}

fn layout_metadata<Action>(
    host: &HostNode<Action>,
    rect: Rect,
    clip_rect: Rect,
    metrics: &BackendMetrics,
) -> (Size, Rect, Rect, LayoutOverflow, Option<TextMetrics>) {
    match host.kind {
        HostKind::Text => {
            let measured_size = measure_host(host, metrics);
            let content_size = text_content_size(host, metrics);
            let content_rect = Rect::new(rect.x, rect.y, content_size.width, content_size.height);
            let resolved_clip_rect = intersect_rects(rect, clip_rect).unwrap_or(Rect::new(
                rect.x.max(clip_rect.x),
                rect.y.max(clip_rect.y),
                0,
                0,
            ));
            let overflow = LayoutOverflow {
                horizontal: content_rect.width > resolved_clip_rect.width,
                vertical: content_rect.height > resolved_clip_rect.height,
            };
            (
                measured_size,
                content_rect,
                resolved_clip_rect,
                overflow,
                Some(text_metrics_from_backend(metrics)),
            )
        }
        HostKind::View { .. }
        | HostKind::ScrollView { .. }
        | HostKind::Clip { .. }
        | HostKind::Layer { .. } => {
            let measured_size = measure_host(host, metrics);
            let content_rect = host.layout_style.padding.inset_rect(rect);
            let resolved_clip_rect = intersect_rects(content_rect, clip_rect).unwrap_or(Rect::new(
                content_rect.x.max(clip_rect.x),
                content_rect.y.max(clip_rect.y),
                0,
                0,
            ));
            let measured_content = desired_view_content_size(host, metrics);
            let overflow = LayoutOverflow {
                horizontal: measured_content.width > resolved_clip_rect.width,
                vertical: measured_content.height > resolved_clip_rect.height,
            };
            (
                measured_size,
                content_rect,
                resolved_clip_rect,
                overflow,
                None,
            )
        }
    }
}

fn text_metrics_from_backend(metrics: &BackendMetrics) -> TextMetrics {
    TextMetrics {
        line_height: metrics.line_height.max(1),
        baseline: metrics.baseline,
    }
}

fn intrinsic_inline_width<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> u16 {
    resolved_intrinsic_size(host, metrics).width
}

fn natural_intrinsic_inline_width<Action>(
    host: &HostNode<Action>,
    metrics: &BackendMetrics,
) -> u16 {
    match container_axis(host.kind) {
        None => host
            .text
            .as_deref()
            .map(|text| measure_text_width(text, metrics))
            .unwrap_or(0),
        Some(Axis::Horizontal) => {
            host.layout_style
                .padding
                .horizontal()
                .saturating_add(measure_main_axis_extent(
                    host.children
                        .iter()
                        .map(|child| outer_inline_width(child, metrics)),
                    host.layout_style.gap,
                ))
        }
        Some(Axis::Vertical) => host.layout_style.padding.horizontal().saturating_add(
            host.children
                .iter()
                .map(|child| outer_inline_width(child, metrics))
                .max()
                .unwrap_or(0),
        ),
    }
}

fn intrinsic_block_height<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> u16 {
    resolved_intrinsic_size(host, metrics).height
}

fn natural_intrinsic_block_height<Action>(
    host: &HostNode<Action>,
    metrics: &BackendMetrics,
) -> u16 {
    match container_axis(host.kind) {
        None => metrics.line_height.max(1),
        Some(Axis::Vertical) => {
            host.layout_style
                .padding
                .vertical()
                .saturating_add(measure_main_axis_extent(
                    host.children
                        .iter()
                        .map(|child| outer_block_height(child, metrics)),
                    host.layout_style.gap,
                ))
        }
        Some(Axis::Horizontal) => host.layout_style.padding.vertical().saturating_add(
            host.children
                .iter()
                .map(|child| outer_block_height(child, metrics))
                .max()
                .unwrap_or(1),
        ),
    }
}

fn measure_host<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> Size {
    resolved_intrinsic_size(host, metrics)
}

fn resolved_intrinsic_size<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> Size {
    apply_layout_size_floor(
        host.layout_style,
        host.layout_style.fixed_size.unwrap_or_else(|| {
            Size::new(
                natural_intrinsic_inline_width(host, metrics),
                natural_intrinsic_block_height(host, metrics),
            )
        }),
    )
}

fn measure_text_width(text: &str, metrics: &BackendMetrics) -> u16 {
    let chars: Vec<char> = text.chars().collect();
    let mut width = 0u16;
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '\t' {
            width = width.saturating_add(metrics.tab_width);
            index += 1;
            continue;
        }
        if is_zero_width_char(ch) {
            index += 1;
            continue;
        }
        if let Some((advance, consumed)) = measure_emoji_cluster_width(&chars[index..], metrics) {
            width = width.saturating_add(advance);
            index += consumed;
            continue;
        }
        width = width.saturating_add(char_display_width(ch, metrics));
        index += 1;
    }
    width
}

fn char_display_width(ch: char, metrics: &BackendMetrics) -> u16 {
    match ch {
        '\t' => metrics.tab_width,
        _ if is_zero_width_char(ch) => 0,
        _ if is_wide_char(ch) => metrics.wide_char_width.max(1),
        _ => 1,
    }
}

fn measure_emoji_cluster_width(chars: &[char], metrics: &BackendMetrics) -> Option<(u16, usize)> {
    let first = *chars.first()?;
    let wide = metrics.wide_char_width.max(1);

    if is_keycap_base(first) {
        let mut index = 1usize;
        if chars
            .get(index)
            .copied()
            .is_some_and(is_emoji_variation_selector)
        {
            index += 1;
        }
        if chars.get(index).copied().is_some_and(is_keycap_encloser) {
            return Some((wide, index + 1));
        }
    }

    if is_regional_indicator(first) && chars.get(1).copied().is_some_and(is_regional_indicator) {
        return Some((wide, 2));
    }

    let mut index = 1usize;
    let mut consumed_multiple_scalars = false;
    let mut treat_as_emoji_cluster = is_emoji_base(first);
    let mut cluster_width = char_display_width(first, metrics);

    if !treat_as_emoji_cluster
        && chars
            .get(index)
            .copied()
            .is_some_and(is_emoji_variation_selector)
        && is_emoji_presentation_candidate(first)
    {
        treat_as_emoji_cluster = true;
    }

    if !treat_as_emoji_cluster {
        return None;
    }

    loop {
        let mut consumed_suffix = false;
        while let Some(ch) = chars.get(index).copied() {
            if is_emoji_variation_selector(ch) || is_emoji_modifier(ch) {
                cluster_width = wide;
                index += 1;
                consumed_multiple_scalars = true;
                consumed_suffix = true;
                continue;
            }
            break;
        }

        if chars.get(index).copied().is_some_and(is_zwj) {
            let Some(next_base) = chars.get(index + 1).copied() else {
                break;
            };
            if is_emoji_base(next_base)
                || is_emoji_presentation_candidate(next_base)
                || is_regional_indicator(next_base)
            {
                cluster_width = wide;
                index += 2;
                consumed_multiple_scalars = true;
                continue;
            }
        }

        if !consumed_suffix {
            break;
        }
    }

    (consumed_multiple_scalars || cluster_width != char_display_width(first, metrics))
        .then_some((cluster_width, index))
}

fn is_emoji_modifier(ch: char) -> bool {
    matches!(ch as u32, 0x1F3FB..=0x1F3FF)
}

fn is_emoji_variation_selector(ch: char) -> bool {
    matches!(ch as u32, 0xFE0E..=0xFE0F)
}

fn is_zwj(ch: char) -> bool {
    ch == '\u{200D}'
}

fn is_keycap_base(ch: char) -> bool {
    matches!(ch, '0'..='9' | '#' | '*')
}

fn is_keycap_encloser(ch: char) -> bool {
    ch == '\u{20E3}'
}

fn is_regional_indicator(ch: char) -> bool {
    matches!(ch as u32, 0x1F1E6..=0x1F1FF)
}

fn is_emoji_base(ch: char) -> bool {
    matches!(
        ch as u32,
        0x231A..=0x231B
            | 0x23E9..=0x23EC
            | 0x23F0
            | 0x23F3
            | 0x25FD..=0x25FE
            | 0x2614..=0x2615
            | 0x2648..=0x2653
            | 0x267F
            | 0x2693
            | 0x26A1
            | 0x26AA..=0x26AB
            | 0x26BD..=0x26BE
            | 0x26C4..=0x26C5
            | 0x26CE
            | 0x26D4
            | 0x26EA
            | 0x26F2..=0x26F3
            | 0x26F5
            | 0x26FA
            | 0x26FD
            | 0x2705
            | 0x270A..=0x270B
            | 0x2728
            | 0x274C
            | 0x274E
            | 0x2753..=0x2755
            | 0x2757
            | 0x2795..=0x2797
            | 0x27B0
            | 0x27BF
            | 0x1F004
            | 0x1F0CF
            | 0x1F18E
            | 0x1F191..=0x1F19A
            | 0x1F1E6..=0x1F1FF
            | 0x1F200..=0x1FAFF
    )
}

fn is_emoji_presentation_candidate(ch: char) -> bool {
    is_emoji_base(ch)
        || matches!(
            ch as u32,
            0x2194..=0x21AA | 0x231A..=0x3299 | 0x00A9 | 0x00AE | 0x203C | 0x2049
        )
}

fn is_zero_width_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0000..=0x001F
            | 0x007F..=0x009F
            | 0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x07EB..=0x07F3
            | 0x07FD
            | 0x0816..=0x0819
            | 0x081B..=0x0823
            | 0x0825..=0x0827
            | 0x0829..=0x082D
            | 0x0859..=0x085B
            | 0x0898..=0x089F
            | 0x08CA..=0x08E1
            | 0x08E3..=0x0902
            | 0x093A
            | 0x093C
            | 0x0941..=0x0948
            | 0x094D
            | 0x0951..=0x0957
            | 0x0962..=0x0963
            | 0x0981
            | 0x09BC
            | 0x09C1..=0x09C4
            | 0x09CD
            | 0x09E2..=0x09E3
            | 0x09FE
            | 0x0A01..=0x0A02
            | 0x0A3C
            | 0x0A41..=0x0A42
            | 0x0A47..=0x0A48
            | 0x0A4B..=0x0A4D
            | 0x0A51
            | 0x0A70..=0x0A71
            | 0x0A75
            | 0x0A81..=0x0A82
            | 0x0ABC
            | 0x0AC1..=0x0AC5
            | 0x0AC7..=0x0AC8
            | 0x0ACD
            | 0x0AE2..=0x0AE3
            | 0x0AFA..=0x0AFF
            | 0x0B01
            | 0x0B3C
            | 0x0B3F
            | 0x0B41..=0x0B44
            | 0x0B4D
            | 0x0B55..=0x0B56
            | 0x0B62..=0x0B63
            | 0x0B82
            | 0x0BC0
            | 0x0BCD
            | 0x0C00
            | 0x0C04
            | 0x0C3C
            | 0x0C3E..=0x0C40
            | 0x0C46..=0x0C48
            | 0x0C4A..=0x0C4D
            | 0x0C55..=0x0C56
            | 0x0C62..=0x0C63
            | 0x0C81
            | 0x0CBC
            | 0x0CBF
            | 0x0CC6
            | 0x0CCC..=0x0CCD
            | 0x0CE2..=0x0CE3
            | 0x0D00..=0x0D01
            | 0x0D3B..=0x0D3C
            | 0x0D41..=0x0D44
            | 0x0D4D
            | 0x0D62..=0x0D63
            | 0x0D81
            | 0x0DCA
            | 0x0DD2..=0x0DD4
            | 0x0DD6
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x0EB1
            | 0x0EB4..=0x0EBC
            | 0x0EC8..=0x0ECD
            | 0x0F18..=0x0F19
            | 0x0F35
            | 0x0F37
            | 0x0F39
            | 0x0F71..=0x0F7E
            | 0x0F80..=0x0F84
            | 0x0F86..=0x0F87
            | 0x0F8D..=0x0F97
            | 0x0F99..=0x0FBC
            | 0x0FC6
            | 0x102D..=0x1030
            | 0x1032..=0x1037
            | 0x1039..=0x103A
            | 0x103D..=0x103E
            | 0x1058..=0x1059
            | 0x105E..=0x1060
            | 0x1071..=0x1074
            | 0x1082
            | 0x1085..=0x1086
            | 0x108D
            | 0x109D
            | 0x135D..=0x135F
            | 0x1712..=0x1714
            | 0x1732..=0x1734
            | 0x1752..=0x1753
            | 0x1772..=0x1773
            | 0x17B4..=0x17B5
            | 0x17B7..=0x17BD
            | 0x17C6
            | 0x17C9..=0x17D3
            | 0x17DD
            | 0x180B..=0x180D
            | 0x180F
            | 0x1885..=0x1886
            | 0x18A9
            | 0x1920..=0x1922
            | 0x1927..=0x1928
            | 0x1932
            | 0x1939..=0x193B
            | 0x1A17..=0x1A18
            | 0x1A1B
            | 0x1A56
            | 0x1A58..=0x1A5E
            | 0x1A60
            | 0x1A62
            | 0x1A65..=0x1A6C
            | 0x1A73..=0x1A7C
            | 0x1A7F
            | 0x1AB0..=0x1ACE
            | 0x1B00..=0x1B03
            | 0x1B34
            | 0x1B36..=0x1B3A
            | 0x1B3C
            | 0x1B42
            | 0x1B6B..=0x1B73
            | 0x1B80..=0x1B81
            | 0x1BA2..=0x1BA5
            | 0x1BA8..=0x1BA9
            | 0x1BAB..=0x1BAD
            | 0x1BE6
            | 0x1BE8..=0x1BE9
            | 0x1BED
            | 0x1BEF..=0x1BF1
            | 0x1C2C..=0x1C33
            | 0x1C36..=0x1C37
            | 0x1CD0..=0x1CD2
            | 0x1CD4..=0x1CE0
            | 0x1CE2..=0x1CE8
            | 0x1CED
            | 0x1CF4
            | 0x1CF8..=0x1CF9
            | 0x1DC0..=0x1DFF
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x2066..=0x206F
            | 0x20D0..=0x20F0
            | 0x2CEF..=0x2CF1
            | 0x2D7F
            | 0x2DE0..=0x2DFF
            | 0x302A..=0x302F
            | 0x3099..=0x309A
            | 0xA66F..=0xA672
            | 0xA674..=0xA67D
            | 0xA69E..=0xA69F
            | 0xA6F0..=0xA6F1
            | 0xA802
            | 0xA806
            | 0xA80B
            | 0xA825..=0xA826
            | 0xA82C
            | 0xA8C4..=0xA8C5
            | 0xA8E0..=0xA8F1
            | 0xA8FF..=0xA8FF
            | 0xA926..=0xA92D
            | 0xA947..=0xA951
            | 0xA980..=0xA982
            | 0xA9B3
            | 0xA9B6..=0xA9B9
            | 0xA9BC..=0xA9BD
            | 0xA9E5
            | 0xAA29..=0xAA2E
            | 0xAA31..=0xAA32
            | 0xAA35..=0xAA36
            | 0xAA43
            | 0xAA4C
            | 0xAA7C
            | 0xAAB0
            | 0xAAB2..=0xAAB4
            | 0xAAB7..=0xAAB8
            | 0xAABE..=0xAABF
            | 0xAAC1
            | 0xAAEC..=0xAAED
            | 0xAAF6
            | 0xABE5
            | 0xABE8
            | 0xABED
            | 0xFB1E
            | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F
            | 0xFEFF
            | 0xFFF9..=0xFFFB
            | 0x101FD
            | 0x102E0
            | 0x10376..=0x1037A
            | 0x10A01..=0x10A03
            | 0x10A05..=0x10A06
            | 0x10A0C..=0x10A0F
            | 0x10A38..=0x10A3A
            | 0x10A3F
            | 0x10AE5..=0x10AE6
            | 0x10D24..=0x10D27
            | 0x10EAB..=0x10EAC
            | 0x10EFD..=0x10EFF
            | 0x10F46..=0x10F50
            | 0x10F82..=0x10F85
            | 0x11001
            | 0x11038..=0x11046
            | 0x11070
            | 0x11073..=0x11074
            | 0x1107F..=0x11081
            | 0x110B3..=0x110B6
            | 0x110B9..=0x110BA
            | 0x11100..=0x11102
            | 0x11127..=0x1112B
            | 0x1112D..=0x11134
            | 0x11173
            | 0x11180..=0x11181
            | 0x111B6..=0x111BE
            | 0x111C9..=0x111CC
            | 0x111CF
            | 0x1122F..=0x11231
            | 0x11234
            | 0x11236..=0x11237
            | 0x1123E
            | 0x11241
            | 0x112DF
            | 0x112E3..=0x112EA
            | 0x11300..=0x11301
            | 0x1133B..=0x1133C
            | 0x11340
            | 0x11366..=0x1136C
            | 0x11370..=0x11374
            | 0x11438..=0x1143F
            | 0x11442..=0x11444
            | 0x11446
            | 0x1145E
            | 0x114B3..=0x114B8
            | 0x114BA
            | 0x114BF..=0x114C0
            | 0x114C2..=0x114C3
            | 0x115B2..=0x115B5
            | 0x115BC..=0x115BD
            | 0x115BF..=0x115C0
            | 0x115DC..=0x115DD
            | 0x11633..=0x1163A
            | 0x1163D
            | 0x1163F..=0x11640
            | 0x116AB
            | 0x116AD
            | 0x116B0..=0x116B5
            | 0x116B7
            | 0x1171D..=0x1171F
            | 0x11722..=0x11725
            | 0x11727..=0x1172B
            | 0x1182F..=0x11837
            | 0x11839..=0x1183A
            | 0x1193B..=0x1193C
            | 0x1193E
            | 0x11943
            | 0x119D4..=0x119D7
            | 0x119DA..=0x119DB
            | 0x119E0
            | 0x11A01..=0x11A0A
            | 0x11A33..=0x11A38
            | 0x11A3B..=0x11A3E
            | 0x11A47
            | 0x11A51..=0x11A56
            | 0x11A59..=0x11A5B
            | 0x11A8A..=0x11A96
            | 0x11A98..=0x11A99
            | 0x11C30..=0x11C36
            | 0x11C38..=0x11C3D
            | 0x11C3F
            | 0x11C92..=0x11CA7
            | 0x11CAA..=0x11CB0
            | 0x11CB2..=0x11CB3
            | 0x11CB5..=0x11CB6
            | 0x11D31..=0x11D36
            | 0x11D3A
            | 0x11D3C..=0x11D3D
            | 0x11D3F..=0x11D45
            | 0x11D47
            | 0x11D90..=0x11D91
            | 0x11D95
            | 0x11D97
            | 0x11EF3..=0x11EF4
            | 0x11F00..=0x11F01
            | 0x11F36..=0x11F3A
            | 0x11F40
            | 0x11F42
            | 0x13430..=0x13440
            | 0x13447..=0x13455
            | 0x16AF0..=0x16AF4
            | 0x16B30..=0x16B36
            | 0x16F4F
            | 0x16F8F..=0x16F92
            | 0x16FE4
            | 0x1BC9D..=0x1BC9E
            | 0x1CF00..=0x1CF2D
            | 0x1CF30..=0x1CF46
            | 0x1D167..=0x1D169
            | 0x1D17B..=0x1D182
            | 0x1D185..=0x1D18B
            | 0x1D1AA..=0x1D1AD
            | 0x1D242..=0x1D244
            | 0x1DA00..=0x1DA36
            | 0x1DA3B..=0x1DA6C
            | 0x1DA75
            | 0x1DA84
            | 0x1DA9B..=0x1DA9F
            | 0x1DAA1..=0x1DAAF
            | 0x1E000..=0x1E006
            | 0x1E008..=0x1E018
            | 0x1E01B..=0x1E021
            | 0x1E023..=0x1E024
            | 0x1E026..=0x1E02A
            | 0x1E08F
            | 0x1E130..=0x1E136
            | 0x1E2AE
            | 0x1E2EC..=0x1E2EF
            | 0x1E4EC..=0x1E4EF
            | 0x1E8D0..=0x1E8D6
            | 0x1E944..=0x1E94A
            | 0xE0100..=0xE01EF
    )
}

fn is_wide_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x231A..=0x231B
            | 0x2329..=0x232A
            | 0x23E9..=0x23EC
            | 0x23F0
            | 0x23F3
            | 0x25FD..=0x25FE
            | 0x2614..=0x2615
            | 0x2648..=0x2653
            | 0x267F
            | 0x2693
            | 0x26A1
            | 0x26AA..=0x26AB
            | 0x26BD..=0x26BE
            | 0x26C4..=0x26C5
            | 0x26CE
            | 0x26D4
            | 0x26EA
            | 0x26F2..=0x26F3
            | 0x26F5
            | 0x26FA
            | 0x26FD
            | 0x2705
            | 0x270A..=0x270B
            | 0x2728
            | 0x274C
            | 0x274E
            | 0x2753..=0x2755
            | 0x2757
            | 0x2795..=0x2797
            | 0x27B0
            | 0x27BF
            | 0x2B1B..=0x2B1C
            | 0x2B50
            | 0x2B55
            | 0x2E80..=0x2E99
            | 0x2E9B..=0x2EF3
            | 0x2F00..=0x2FD5
            | 0x2FF0..=0x2FFB
            | 0x3000..=0x303E
            | 0x3041..=0x3096
            | 0x309B..=0x30FF
            | 0x3105..=0x312F
            | 0x3131..=0x318E
            | 0x3190..=0x31E5
            | 0x31EF..=0x321E
            | 0x3220..=0x3247
            | 0x3250..=0x32FE
            | 0x3300..=0x4DBF
            | 0x4E00..=0xA48C
            | 0xA490..=0xA4C6
            | 0xA960..=0xA97C
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE52
            | 0xFE54..=0xFE66
            | 0xFE68..=0xFE6B
            | 0xFF01..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x16FE0..=0x16FE3
            | 0x16FF0..=0x16FF1
            | 0x17000..=0x187F7
            | 0x18800..=0x18CD5
            | 0x18D00..=0x18D08
            | 0x1AFF0..=0x1AFF3
            | 0x1AFF5..=0x1AFFB
            | 0x1AFFD..=0x1AFFE
            | 0x1B000..=0x1B122
            | 0x1B132
            | 0x1B150..=0x1B152
            | 0x1B155..=0x1B155
            | 0x1B164..=0x1B167
            | 0x1F004
            | 0x1F0CF
            | 0x1F18E
            | 0x1F191..=0x1F19A
            | 0x1F200..=0x1F202
            | 0x1F210..=0x1F23B
            | 0x1F240..=0x1F248
            | 0x1F250..=0x1F251
            | 0x1F260..=0x1F265
            | 0x1F300..=0x1F64F
            | 0x1F680..=0x1F6FF
            | 0x1F700..=0x1F773
            | 0x1F780..=0x1F7D8
            | 0x1F7E0..=0x1F7EB
            | 0x1F7F0..=0x1F7F0
            | 0x1F800..=0x1F80B
            | 0x1F810..=0x1F847
            | 0x1F850..=0x1F859
            | 0x1F860..=0x1F887
            | 0x1F890..=0x1F8AD
            | 0x1F8B0..=0x1F8BB
            | 0x1F8C0..=0x1F8C1
            | 0x1F900..=0x1F978
            | 0x1F97A..=0x1F9CB
            | 0x1F9CD..=0x1FA53
            | 0x1FA60..=0x1FA6D
            | 0x1FA70..=0x1FA7C
            | 0x1FA80..=0x1FA89
            | 0x1FA90..=0x1FABD
            | 0x1FABF..=0x1FAC5
            | 0x1FACE..=0x1FADB
            | 0x1FAE0..=0x1FAE8
            | 0x1FAF0..=0x1FAF8
            | 0x20000..=0x2FFFD
            | 0x30000..=0x3FFFD
    )
}

fn measure_content_size<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> Size {
    match container_axis(host.kind) {
        None => Size::new(
            host.text
                .as_deref()
                .map(|text| measure_text_width(text, metrics))
                .unwrap_or(0),
            natural_intrinsic_block_height(host, metrics),
        ),
        Some(Axis::Vertical) => Size::new(
            host.children
                .iter()
                .map(|child| outer_inline_width(child, metrics))
                .max()
                .unwrap_or(0),
            measure_main_axis_extent(
                host.children
                    .iter()
                    .map(|child| outer_block_height(child, metrics)),
                host.layout_style.gap,
            ),
        ),
        Some(Axis::Horizontal) => Size::new(
            measure_main_axis_extent(
                host.children
                    .iter()
                    .map(|child| outer_inline_width(child, metrics)),
                host.layout_style.gap,
            ),
            host.children
                .iter()
                .map(|child| outer_block_height(child, metrics))
                .max()
                .unwrap_or(1),
        ),
    }
}

fn text_content_size<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> Size {
    apply_layout_size_floor(
        host.layout_style,
        host.layout_style
            .fixed_size
            .unwrap_or_else(|| measure_content_size(host, metrics)),
    )
}

fn desired_view_content_size<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> Size {
    let natural = measure_content_size(host, metrics);
    let measured = apply_layout_size_floor(
        host.layout_style,
        host.layout_style.fixed_size.unwrap_or(natural),
    );
    if host.layout_style.fixed_size.is_some() || host.layout_style.min_size.is_some() {
        let fixed_content_rect =
            host.layout_style
                .padding
                .inset_rect(Rect::new(0, 0, measured.width, measured.height));
        Size::new(
            natural.width.max(fixed_content_rect.width),
            natural.height.max(fixed_content_rect.height),
        )
    } else {
        natural
    }
}

fn apply_layout_size_floor(style: LayoutStyle, size: Size) -> Size {
    if let Some(min_size) = style.min_size {
        Size::new(
            size.width.max(min_size.width),
            size.height.max(min_size.height),
        )
    } else {
        size
    }
}

fn container_axis(kind: HostKind) -> Option<Axis> {
    match kind {
        HostKind::Text => None,
        HostKind::View { axis }
        | HostKind::ScrollView { axis }
        | HostKind::Clip { axis }
        | HostKind::Layer { axis, .. } => Some(axis),
    }
}

fn outer_inline_width<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> u16 {
    intrinsic_inline_width(host, metrics).saturating_add(host.layout_style.margin.horizontal())
}

fn outer_block_height<Action>(host: &HostNode<Action>, metrics: &BackendMetrics) -> u16 {
    intrinsic_block_height(host, metrics).saturating_add(host.layout_style.margin.vertical())
}

fn child_cross_axis_width<Action>(
    host: &HostNode<Action>,
    available_width: u16,
    metrics: &BackendMetrics,
    alignment: CrossAxisAlignment,
) -> u16 {
    match alignment {
        CrossAxisAlignment::Stretch => {
            if host.layout_style.fixed_size.is_some() {
                intrinsic_inline_width(host, metrics).min(available_width)
            } else {
                available_width
            }
        }
        CrossAxisAlignment::Start | CrossAxisAlignment::Center | CrossAxisAlignment::End => {
            intrinsic_inline_width(host, metrics).min(available_width)
        }
    }
}

fn child_cross_axis_height<Action>(
    host: &HostNode<Action>,
    available_height: u16,
    metrics: &BackendMetrics,
    alignment: CrossAxisAlignment,
) -> u16 {
    match alignment {
        CrossAxisAlignment::Stretch => {
            if host.layout_style.fixed_size.is_some() {
                intrinsic_block_height(host, metrics).min(available_height)
            } else {
                available_height
            }
        }
        CrossAxisAlignment::Start | CrossAxisAlignment::Center | CrossAxisAlignment::End => {
            intrinsic_block_height(host, metrics).min(available_height)
        }
    }
}

fn measure_main_axis_extent(values: impl Iterator<Item = u16>, gap: u16) -> u16 {
    let mut extent = 0u16;
    let mut count = 0u16;
    for value in values {
        extent = extent.saturating_add(value);
        count = count.saturating_add(1);
    }
    if count > 1 {
        extent = extent.saturating_add(gap.saturating_mul(count - 1));
    }
    extent
}

fn distribute_flex_main_axis<'a, Action>(
    children: impl Iterator<Item = &'a HostNode<Action>>,
    available_main_size: u16,
    gap: u16,
    base_outer_main_size: impl Fn(&HostNode<Action>) -> u16,
) -> Vec<u16>
where
    Action: 'a,
{
    let children: Vec<&HostNode<Action>> = children.collect();
    let mut assigned: Vec<u16> = children
        .iter()
        .map(|child| base_outer_main_size(child).min(available_main_size))
        .collect();
    let base_total = measure_main_axis_extent(assigned.iter().copied(), gap);
    if base_total >= available_main_size {
        return assigned;
    }

    let total_flex: u32 = children
        .iter()
        .map(|child| u32::from(child.layout_style.flex_grow))
        .sum();
    if total_flex == 0 {
        return assigned;
    }

    let remaining = available_main_size.saturating_sub(base_total);
    let mut distributed = 0u16;
    for (index, child) in children.iter().enumerate() {
        let flex = u32::from(child.layout_style.flex_grow);
        if flex == 0 {
            continue;
        }
        let extra = ((u32::from(remaining) * flex) / total_flex) as u16;
        assigned[index] = assigned[index].saturating_add(extra);
        distributed = distributed.saturating_add(extra);
    }

    let mut leftover = remaining.saturating_sub(distributed);
    while leftover > 0 {
        let mut advanced = false;
        for (index, child) in children.iter().enumerate() {
            if child.layout_style.flex_grow == 0 {
                continue;
            }
            assigned[index] = assigned[index].saturating_add(1);
            leftover = leftover.saturating_sub(1);
            advanced = true;
            if leftover == 0 {
                break;
            }
        }
        if !advanced {
            break;
        }
    }

    assigned
}

fn main_axis_alignment_offset(alignment: MainAxisAlignment, available: u16, occupied: u16) -> u16 {
    let remaining = available.saturating_sub(occupied);
    match alignment {
        MainAxisAlignment::Start => 0,
        MainAxisAlignment::Center => remaining / 2,
        MainAxisAlignment::End => remaining,
    }
}

fn cross_axis_alignment_offset(
    alignment: CrossAxisAlignment,
    available: u16,
    occupied: u16,
) -> u16 {
    let remaining = available.saturating_sub(occupied);
    match alignment {
        CrossAxisAlignment::Stretch | CrossAxisAlignment::Start => 0,
        CrossAxisAlignment::Center => remaining / 2,
        CrossAxisAlignment::End => remaining,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        column, lower_element, row, text, view, CrossAxisAlignment, MainAxisAlignment, Margin,
        Padding, ScrollOffset,
    };

    #[test]
    fn column_layout_stacks_children_vertically() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 5));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 10, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 1, 10, 1));
        assert_eq!(layout[1].clip_rect, Rect::new(0, 0, 10, 1));
        assert_eq!(layout[2].clip_rect, Rect::new(0, 1, 10, 1));
    }

    #[test]
    fn row_layout_stacks_children_horizontally() {
        let element = row((text::<()>("a"), text::<()>("bb")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 3));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 1, 3));
        assert_eq!(layout[2].rect, Rect::new(1, 0, 2, 3));
        assert_eq!(layout[2].content_rect, Rect::new(1, 0, 2, 1));
    }

    #[test]
    fn text_layout_uses_metrics_and_constraints_for_content_and_overflow() {
        let host = lower_element(&text::<()>("a\t中"));
        let metrics = BackendMetrics {
            line_height: 1,
            tab_width: 3,
            wide_char_width: 2,
            baseline: Some(0),
        };
        let layout =
            layout_tree_with_metrics(&host, LayoutConstraints::loose(Size::new(5, 1)), &metrics);

        assert_eq!(layout[0].rect, Rect::new(0, 0, 5, 1));
        assert_eq!(layout[0].measured_size, Size::new(6, 1));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 6, 1));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 5, 1));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: true,
                vertical: false,
            }
        );
    }

    #[test]
    fn text_node_records_text_metrics_by_default() {
        let host = lower_element(&text::<()>("hello"));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(5, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(
            layout[0].text_metrics,
            Some(TextMetrics {
                line_height: 1,
                baseline: Some(0),
            })
        );
    }

    #[test]
    fn custom_backend_metrics_reach_text_metrics_metadata() {
        let host = lower_element(&text::<()>("hello"));
        let metrics = BackendMetrics {
            line_height: 3,
            tab_width: 4,
            wide_char_width: 2,
            baseline: Some(2),
        };
        let layout =
            layout_tree_with_metrics(&host, LayoutConstraints::tight(Size::new(5, 3)), &metrics);

        assert_eq!(
            layout[0].text_metrics,
            Some(TextMetrics {
                line_height: 3,
                baseline: Some(2),
            })
        );
    }

    #[test]
    fn baseline_none_is_preserved_in_text_metrics_metadata() {
        let host = lower_element(&text::<()>("hello"));
        let metrics = BackendMetrics {
            line_height: 2,
            tab_width: 4,
            wide_char_width: 2,
            baseline: None,
        };
        let layout =
            layout_tree_with_metrics(&host, LayoutConstraints::tight(Size::new(5, 2)), &metrics);

        assert_eq!(
            layout[0].text_metrics,
            Some(TextMetrics {
                line_height: 2,
                baseline: None,
            })
        );
    }

    #[test]
    fn view_nodes_do_not_report_text_metrics() {
        let host = lower_element(&column((text::<()>("a"), text::<()>("b"))));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(4, 3)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].text_metrics, None);
    }

    #[test]
    fn text_metrics_metadata_does_not_change_layout_geometry() {
        let host = lower_element(&text::<()>("hello"));
        let with_baseline = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics {
                line_height: 1,
                tab_width: 4,
                wide_char_width: 2,
                baseline: Some(0),
            },
        );
        let without_baseline = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics {
                line_height: 1,
                tab_width: 4,
                wide_char_width: 2,
                baseline: None,
            },
        );

        assert_eq!(with_baseline[0].rect, without_baseline[0].rect);
        assert_eq!(
            with_baseline[0].measured_size,
            without_baseline[0].measured_size
        );
        assert_eq!(
            with_baseline[0].content_rect,
            without_baseline[0].content_rect
        );
        assert_eq!(with_baseline[0].clip_rect, without_baseline[0].clip_rect);
        assert_eq!(with_baseline[0].overflow, without_baseline[0].overflow);
    }

    #[test]
    fn display_width_counts_ascii_characters_as_single_cells() {
        let metrics = BackendMetrics::default();

        assert_eq!(measure_text_width("ASCII 123", &metrics), 9);
    }

    #[test]
    fn display_width_uses_backend_tab_width() {
        let metrics = BackendMetrics {
            tab_width: 6,
            ..BackendMetrics::default()
        };

        assert_eq!(measure_text_width("a\tb", &metrics), 8);
    }

    #[test]
    fn display_width_uses_wide_metrics_for_cjk_ranges() {
        let metrics = BackendMetrics {
            wide_char_width: 3,
            ..BackendMetrics::default()
        };

        assert_eq!(measure_text_width("中A界", &metrics), 7);
    }

    #[test]
    fn display_width_ignores_combining_marks_and_variation_selectors() {
        let metrics = BackendMetrics::default();

        assert_eq!(measure_text_width("e\u{0301}", &metrics), 1);
        assert_eq!(measure_text_width("\u{2B50}\u{FE0F}", &metrics), 2);
        assert_eq!(measure_text_width("a\u{200D}b", &metrics), 2);
    }

    #[test]
    fn display_width_keeps_ambiguous_non_cjk_characters_single_width() {
        let metrics = BackendMetrics {
            wide_char_width: 2,
            ..BackendMetrics::default()
        };

        assert_eq!(measure_text_width("·Ω", &metrics), 2);
    }

    #[test]
    fn display_width_treats_emoji_modifier_sequences_as_single_wide_cluster() {
        let metrics = BackendMetrics::default();

        assert_eq!(measure_text_width("👍🏽", &metrics), 2);
    }

    #[test]
    fn display_width_treats_zwj_emoji_family_as_single_wide_cluster() {
        let metrics = BackendMetrics::default();

        assert_eq!(measure_text_width("👨‍👩‍👧‍👦", &metrics), 2);
    }

    #[test]
    fn display_width_treats_keycap_sequence_as_single_wide_cluster() {
        let metrics = BackendMetrics::default();

        assert_eq!(measure_text_width("1️⃣", &metrics), 2);
    }

    #[test]
    fn row_layout_clips_children_against_horizontal_constraints() {
        let element = row((text::<()>("aa"), text::<()>("bbb")));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(4, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 4, 1));
        assert_eq!(layout[2].rect, Rect::new(2, 0, 2, 1));
        assert_eq!(layout[2].content_rect, Rect::new(2, 0, 3, 1));
        assert_eq!(layout[2].clip_rect, Rect::new(2, 0, 2, 1));
        assert!(layout[2].overflow.horizontal);
    }

    #[test]
    fn column_layout_uses_line_height_metrics_for_vertical_overflow() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);
        let metrics = BackendMetrics {
            line_height: 2,
            ..BackendMetrics::default()
        };
        let layout =
            layout_tree_with_metrics(&host, LayoutConstraints::tight(Size::new(3, 3)), &metrics);

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 3, 3));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: false,
                vertical: true,
            }
        );
        assert_eq!(layout[1].rect, Rect::new(0, 0, 3, 2));
        assert_eq!(layout[2].rect, Rect::new(0, 2, 3, 1));
        assert_eq!(layout[2].content_rect, Rect::new(0, 2, 1, 2));
        assert_eq!(layout[2].clip_rect, Rect::new(0, 2, 3, 1));
        assert!(layout[2].overflow.vertical);
    }

    #[test]
    fn row_layout_applies_gap_between_children() {
        let element = row((text::<()>("a"), text::<()>("bb"), text::<()>("c"))).gap(2);
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(12, 1));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(3, 0, 2, 1));
        assert_eq!(layout[3].rect, Rect::new(7, 0, 1, 1));
    }

    #[test]
    fn column_layout_applies_gap_between_children() {
        let element = column((text::<()>("a"), text::<()>("b"), text::<()>("c"))).gap(1);
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(4, 6));

        assert_eq!(layout[1].rect, Rect::new(0, 0, 4, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 2, 4, 1));
        assert_eq!(layout[3].rect, Rect::new(0, 4, 4, 1));
    }

    #[test]
    fn padding_shifts_content_rect_clip_rect_and_child_layout() {
        let element = view((text::<()>("a"),)).padding(Padding::new(1, 2, 0, 1));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 3)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].rect, Rect::new(0, 0, 6, 3));
        assert_eq!(layout[0].content_rect, Rect::new(1, 1, 3, 2));
        assert_eq!(layout[0].clip_rect, Rect::new(1, 1, 3, 2));
        assert_eq!(layout[1].rect, Rect::new(1, 1, 3, 1));
        assert_eq!(layout[1].clip_rect, Rect::new(1, 1, 3, 1));
    }

    #[test]
    fn row_child_margin_changes_placement_and_extent() {
        let element = row((
            text::<()>("a").margin(Margin::new(1, 2, 1, 1)),
            text::<()>("b"),
        ));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(5, 3)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 5, 3));
        assert_eq!(layout[1].rect, Rect::new(1, 1, 1, 1));
        assert_eq!(layout[1].content_rect, Rect::new(1, 1, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(4, 0, 1, 3));
    }

    #[test]
    fn column_child_margin_changes_placement_and_extent() {
        let element = column((
            text::<()>("a").margin(Margin::new(1, 1, 2, 1)),
            text::<()>("b"),
        ));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 5)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 3, 5));
        assert_eq!(layout[1].rect, Rect::new(1, 1, 1, 1));
        assert_eq!(layout[1].content_rect, Rect::new(1, 1, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 4, 3, 1));
    }

    #[test]
    fn padding_and_margin_preserve_child_content_rect_semantics() {
        let element = row((
            view((text::<()>("a"),))
                .margin(Margin::new(1, 1, 0, 2))
                .padding(Padding::new(1, 2, 0, 1)),
            text::<()>("b"),
        ));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(8, 3)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(2, 1, 4, 2));
        assert_eq!(layout[1].content_rect, Rect::new(3, 2, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(3, 2, 1, 1));
        assert_eq!(layout[3].rect, Rect::new(7, 0, 1, 3));
    }

    #[test]
    fn fixed_text_and_view_size_control_final_rect_and_content_rect() {
        let text_host = lower_element(&text::<()>("abcdef").fixed_size(Size::new(4, 2)));
        let text_layout = layout_tree_with_metrics(
            &text_host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(text_layout[0].rect, Rect::new(0, 0, 3, 1));
        assert_eq!(text_layout[0].measured_size, Size::new(4, 2));
        assert_eq!(text_layout[0].content_rect, Rect::new(0, 0, 4, 2));
        assert_eq!(text_layout[0].clip_rect, Rect::new(0, 0, 3, 1));
        assert_eq!(
            text_layout[0].overflow,
            LayoutOverflow {
                horizontal: true,
                vertical: true,
            }
        );

        let view_host = lower_element(
            &view((text::<()>("a"),))
                .padding(Padding::new(1, 1, 0, 1))
                .fixed_size(Size::new(5, 3)),
        );
        let view_layout = layout_tree_with_metrics(
            &view_host,
            LayoutConstraints::tight(Size::new(4, 2)),
            &BackendMetrics::default(),
        );

        assert_eq!(view_layout[0].rect, Rect::new(0, 0, 4, 2));
        assert_eq!(view_layout[0].measured_size, Size::new(5, 3));
        assert_eq!(view_layout[0].content_rect, Rect::new(1, 1, 2, 1));
        assert_eq!(view_layout[0].clip_rect, Rect::new(1, 1, 2, 1));
        assert!(view_layout[0].overflow.horizontal);
        assert!(view_layout[0].overflow.vertical);
    }

    #[test]
    fn min_size_affects_text_measured_size_and_content_rect() {
        let host = lower_element(&text::<()>("a").min_size(Size::new(4, 3)));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 4)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].rect, Rect::new(0, 0, 6, 4));
        assert_eq!(layout[0].measured_size, Size::new(4, 3));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 4, 3));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 6, 4));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: false,
                vertical: false,
            }
        );
    }

    #[test]
    fn min_size_affects_view_row_and_column_measured_size() {
        let view_host = lower_element(&view((text::<()>("a"),)).min_size(Size::new(5, 4)));
        let view_layout = layout_tree_with_metrics(
            &view_host,
            LayoutConstraints::tight(Size::new(8, 6)),
            &BackendMetrics::default(),
        );
        assert_eq!(view_layout[0].measured_size, Size::new(5, 4));
        assert_eq!(view_layout[0].rect, Rect::new(0, 0, 8, 6));
        assert_eq!(view_layout[0].content_rect, Rect::new(0, 0, 8, 6));
        assert_eq!(view_layout[1].rect, Rect::new(0, 0, 8, 1));

        let row_host = lower_element(
            &row((text::<()>("a"), text::<()>("b")))
                .gap(1)
                .min_size(Size::new(6, 3)),
        );
        let row_layout = layout_tree_with_metrics(
            &row_host,
            LayoutConstraints::tight(Size::new(8, 4)),
            &BackendMetrics::default(),
        );
        assert_eq!(row_layout[0].measured_size, Size::new(6, 3));
        assert_eq!(row_layout[0].rect, Rect::new(0, 0, 8, 4));
        assert_eq!(row_layout[2].rect, Rect::new(2, 0, 1, 4));

        let column_host = lower_element(
            &column((text::<()>("a"), text::<()>("b")))
                .gap(1)
                .min_size(Size::new(4, 5)),
        );
        let column_layout = layout_tree_with_metrics(
            &column_host,
            LayoutConstraints::tight(Size::new(6, 6)),
            &BackendMetrics::default(),
        );
        assert_eq!(column_layout[0].measured_size, Size::new(4, 5));
        assert_eq!(column_layout[0].rect, Rect::new(0, 0, 6, 6));
        assert_eq!(column_layout[2].rect, Rect::new(0, 2, 6, 1));
    }

    #[test]
    fn row_flex_grow_distributes_extra_width_to_one_child() {
        let host = lower_element(&row((text::<()>("a"), text::<()>("b").flex_grow(1))));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 6, 1));
        assert_eq!(layout[1].rect, Rect::new(0, 0, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(1, 0, 5, 1));
    }

    #[test]
    fn row_flex_grow_distributes_extra_width_proportionally_and_deterministically() {
        let host = lower_element(&row((
            text::<()>("a").flex_grow(1),
            text::<()>("b").flex_grow(2),
            text::<()>("c"),
        )));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(10, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(0, 0, 4, 1));
        assert_eq!(layout[2].rect, Rect::new(4, 0, 5, 1));
        assert_eq!(layout[3].rect, Rect::new(9, 0, 1, 1));
    }

    #[test]
    fn column_flex_grow_distributes_extra_height() {
        let host = lower_element(&column((text::<()>("a").flex_grow(1), text::<()>("b"))));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 5)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(0, 0, 3, 4));
        assert_eq!(layout[2].rect, Rect::new(0, 4, 3, 1));
    }

    #[test]
    fn row_main_axis_center_and_end_shift_starting_x_without_changing_measured_size() {
        let centered = lower_element(
            &row((text::<()>("a"), text::<()>("b"))).main_axis_alignment(MainAxisAlignment::Center),
        );
        let centered_layout = layout_tree_with_metrics(
            &centered,
            LayoutConstraints::tight(Size::new(6, 1)),
            &BackendMetrics::default(),
        );
        assert_eq!(centered_layout[0].measured_size, Size::new(2, 1));
        assert_eq!(centered_layout[1].rect, Rect::new(2, 0, 1, 1));
        assert_eq!(centered_layout[2].rect, Rect::new(3, 0, 1, 1));

        let ended = lower_element(
            &row((text::<()>("a"), text::<()>("b"))).main_axis_alignment(MainAxisAlignment::End),
        );
        let ended_layout = layout_tree_with_metrics(
            &ended,
            LayoutConstraints::tight(Size::new(6, 1)),
            &BackendMetrics::default(),
        );
        assert_eq!(ended_layout[0].measured_size, Size::new(2, 1));
        assert_eq!(ended_layout[1].rect, Rect::new(4, 0, 1, 1));
        assert_eq!(ended_layout[2].rect, Rect::new(5, 0, 1, 1));
    }

    #[test]
    fn column_main_axis_center_and_end_shift_starting_y() {
        let centered = lower_element(
            &column((text::<()>("a"), text::<()>("b")))
                .main_axis_alignment(MainAxisAlignment::Center),
        );
        let centered_layout = layout_tree_with_metrics(
            &centered,
            LayoutConstraints::tight(Size::new(3, 6)),
            &BackendMetrics::default(),
        );
        assert_eq!(centered_layout[1].rect, Rect::new(0, 2, 3, 1));
        assert_eq!(centered_layout[2].rect, Rect::new(0, 3, 3, 1));

        let ended = lower_element(
            &column((text::<()>("a"), text::<()>("b"))).main_axis_alignment(MainAxisAlignment::End),
        );
        let ended_layout = layout_tree_with_metrics(
            &ended,
            LayoutConstraints::tight(Size::new(3, 6)),
            &BackendMetrics::default(),
        );
        assert_eq!(ended_layout[1].rect, Rect::new(0, 4, 3, 1));
        assert_eq!(ended_layout[2].rect, Rect::new(0, 5, 3, 1));
    }

    #[test]
    fn row_cross_axis_alignment_positions_fixed_child_while_stretch_remains_default() {
        let stretched = lower_element(
            &row((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        );
        let stretched_layout = layout_tree_with_metrics(
            &stretched,
            LayoutConstraints::tight(Size::new(4, 5)),
            &BackendMetrics::default(),
        );
        assert_eq!(stretched_layout[1].rect, Rect::new(0, 0, 1, 1));

        let centered = lower_element(
            &row((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::Center),
        );
        let centered_layout = layout_tree_with_metrics(
            &centered,
            LayoutConstraints::tight(Size::new(4, 5)),
            &BackendMetrics::default(),
        );
        assert_eq!(centered_layout[1].rect, Rect::new(0, 2, 1, 1));

        let ended = lower_element(
            &row((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::End),
        );
        let ended_layout = layout_tree_with_metrics(
            &ended,
            LayoutConstraints::tight(Size::new(4, 5)),
            &BackendMetrics::default(),
        );
        assert_eq!(ended_layout[1].rect, Rect::new(0, 4, 1, 1));
    }

    #[test]
    fn column_cross_axis_alignment_positions_child_horizontally_while_stretch_remains_default() {
        let stretched = lower_element(
            &column((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::Stretch),
        );
        let stretched_layout = layout_tree_with_metrics(
            &stretched,
            LayoutConstraints::tight(Size::new(5, 3)),
            &BackendMetrics::default(),
        );
        assert_eq!(stretched_layout[1].rect, Rect::new(0, 0, 1, 1));

        let centered = lower_element(
            &column((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::Center),
        );
        let centered_layout = layout_tree_with_metrics(
            &centered,
            LayoutConstraints::tight(Size::new(5, 3)),
            &BackendMetrics::default(),
        );
        assert_eq!(centered_layout[1].rect, Rect::new(2, 0, 1, 1));

        let ended = lower_element(
            &column((text::<()>("a").fixed_size(Size::new(1, 1)),))
                .cross_axis_alignment(CrossAxisAlignment::End),
        );
        let ended_layout = layout_tree_with_metrics(
            &ended,
            LayoutConstraints::tight(Size::new(5, 3)),
            &BackendMetrics::default(),
        );
        assert_eq!(ended_layout[1].rect, Rect::new(4, 0, 1, 1));
    }

    #[test]
    fn main_axis_alignment_does_not_add_offset_after_flex_consumes_remaining_space() {
        let host = lower_element(
            &row((text::<()>("a").flex_grow(1), text::<()>("b")))
                .main_axis_alignment(MainAxisAlignment::Center),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(0, 0, 5, 1));
        assert_eq!(layout[2].rect, Rect::new(5, 0, 1, 1));
    }

    #[test]
    fn scroll_viewport_keeps_offscreen_logical_children_in_layout_tree() {
        let host = lower_element(
            &column((text::<()>("a"), text::<()>("b"), text::<()>("c")))
                .scroll_offset(ScrollOffset::new(0, 1)),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout.len(), 4);
        assert_eq!(layout[1].rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 1, 3, 1));
        assert_eq!(layout[3].rect, Rect::new(0, 2, 3, 1));
    }

    #[test]
    fn scroll_viewport_clip_and_overflow_preserve_logical_content() {
        let host = lower_element(
            &column((text::<()>("a"), text::<()>("b"), text::<()>("c")))
                .scroll_offset(ScrollOffset::new(0, 1)),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 3, 1));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: false,
                vertical: true,
            }
        );
        assert_eq!(layout[1].clip_rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[2].clip_rect, Rect::new(0, 1, 0, 0));
        assert_eq!(layout[3].clip_rect, Rect::new(0, 2, 0, 0));
    }

    #[test]
    fn flex_grow_keeps_existing_clipping_behavior_when_no_extra_space_exists() {
        let host = lower_element(&row((text::<()>("aaa").flex_grow(1), text::<()>("bbb"))));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(4, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[2].rect, Rect::new(3, 0, 1, 1));
        assert_eq!(layout[2].content_rect, Rect::new(3, 0, 3, 1));
        assert!(layout[2].overflow.horizontal);
    }

    #[test]
    fn flex_grow_interacts_with_gap_margin_min_size_and_fixed_size() {
        let host = lower_element(
            &row((
                text::<()>("a")
                    .flex_grow(1)
                    .margin(Margin::new(0, 1, 0, 1))
                    .min_size(Size::new(2, 1)),
                text::<()>("b").fixed_size(Size::new(3, 1)),
            ))
            .gap(1),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(10, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].measured_size, Size::new(2, 1));
        assert_eq!(layout[1].rect, Rect::new(1, 0, 4, 1));
        assert_eq!(layout[2].rect, Rect::new(7, 0, 3, 1));
    }

    #[test]
    fn text_measured_size_remains_distinct_from_clipped_rect_when_constrained() {
        let host = lower_element(&text::<()>("abcdef"));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].measured_size, Size::new(6, 1));
        assert_eq!(layout[0].rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 3, 1));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 6, 1));
    }

    #[test]
    fn clipped_measured_size_uses_display_width() {
        let host = lower_element(&text::<()>("中a"));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(2, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].measured_size, Size::new(3, 1));
        assert_eq!(layout[0].rect, Rect::new(0, 0, 2, 1));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 2, 1));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 3, 1));
        assert!(layout[0].overflow.horizontal);
    }

    #[test]
    fn min_size_under_tight_smaller_constraints_clips_and_reports_overflow() {
        let host = lower_element(&text::<()>("a").min_size(Size::new(4, 3)));
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(2, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].measured_size, Size::new(4, 3));
        assert_eq!(layout[0].rect, Rect::new(0, 0, 2, 1));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 4, 3));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 2, 1));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: true,
                vertical: true,
            }
        );
    }

    #[test]
    fn row_and_column_measured_size_reflect_children_gap_margin_and_padding() {
        let row_host = lower_element(
            &row((
                text::<()>("aa").margin(Margin::new(1, 1, 0, 2)),
                text::<()>("bbb"),
            ))
            .gap(2)
            .padding(Padding::new(1, 2, 3, 4)),
        );
        let row_layout = layout_tree_with_metrics(
            &row_host,
            LayoutConstraints::tight(Size::new(20, 10)),
            &BackendMetrics::default(),
        );

        assert_eq!(row_layout[0].measured_size, Size::new(16, 6));

        let column_host = lower_element(
            &column((
                text::<()>("aa"),
                text::<()>("b").margin(Margin::new(1, 2, 3, 4)),
            ))
            .gap(2)
            .padding(Padding::new(1, 2, 3, 4)),
        );
        let column_layout = layout_tree_with_metrics(
            &column_host,
            LayoutConstraints::tight(Size::new(20, 10)),
            &BackendMetrics::default(),
        );

        assert_eq!(column_layout[0].measured_size, Size::new(13, 12));
    }

    #[test]
    fn row_child_fixed_size_affects_placement_and_parent_overflow() {
        let element = row((
            text::<()>("a").fixed_size(Size::new(4, 2)),
            text::<()>("bbb"),
        ));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 1)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 6, 1));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: true,
                vertical: true,
            }
        );
        assert_eq!(layout[1].rect, Rect::new(0, 0, 4, 1));
        assert_eq!(layout[1].content_rect, Rect::new(0, 0, 4, 2));
        assert_eq!(layout[2].rect, Rect::new(4, 0, 2, 1));
        assert_eq!(layout[2].content_rect, Rect::new(4, 0, 3, 1));
        assert!(layout[2].overflow.horizontal);
    }

    #[test]
    fn fixed_size_coexists_with_padding_and_margin() {
        let element = row((
            view((text::<()>("a"),))
                .fixed_size(Size::new(6, 4))
                .margin(Margin::new(1, 1, 0, 2))
                .padding(Padding::new(1, 2, 1, 1)),
            text::<()>("b"),
        ));
        let host = lower_element(&element);
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(10, 4)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[1].rect, Rect::new(2, 1, 6, 3));
        assert_eq!(layout[1].content_rect, Rect::new(3, 2, 3, 1));
        assert_eq!(layout[1].clip_rect, Rect::new(3, 2, 3, 1));
        assert_eq!(layout[2].rect, Rect::new(3, 2, 3, 1));
        assert_eq!(layout[3].rect, Rect::new(9, 0, 1, 4));
    }

    #[test]
    fn fixed_size_smaller_than_min_size_uses_min_size_floor_and_overflow_when_clipped() {
        let host = lower_element(
            &text::<()>("abcdef")
                .fixed_size(Size::new(2, 1))
                .min_size(Size::new(4, 3)),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(3, 2)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].measured_size, Size::new(4, 3));
        assert_eq!(layout[0].rect, Rect::new(0, 0, 3, 2));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 4, 3));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 3, 2));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: true,
                vertical: true,
            }
        );
    }

    #[test]
    fn fixed_size_larger_than_min_size_preserves_fixed_size_measurement() {
        let host = lower_element(
            &text::<()>("abcdef")
                .fixed_size(Size::new(5, 4))
                .min_size(Size::new(2, 1)),
        );
        let layout = layout_tree_with_metrics(
            &host,
            LayoutConstraints::tight(Size::new(6, 5)),
            &BackendMetrics::default(),
        );

        assert_eq!(layout[0].measured_size, Size::new(5, 4));
        assert_eq!(layout[0].rect, Rect::new(0, 0, 6, 5));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 5, 4));
        assert_eq!(layout[0].clip_rect, Rect::new(0, 0, 6, 5));
        assert_eq!(
            layout[0].overflow,
            LayoutOverflow {
                horizontal: false,
                vertical: false,
            }
        );
    }

    #[test]
    fn default_behavior_is_unchanged_without_fixed_size() {
        let element = column((text::<()>("a"), text::<()>("b")));
        let host = lower_element(&element);
        let layout = layout_tree(&host, Size::new(10, 5));

        assert_eq!(layout[0].rect, Rect::new(0, 0, 10, 5));
        assert_eq!(layout[0].measured_size, Size::new(1, 2));
        assert_eq!(layout[0].content_rect, Rect::new(0, 0, 10, 5));
        assert_eq!(layout[1].rect, Rect::new(0, 0, 10, 1));
        assert_eq!(layout[1].measured_size, Size::new(1, 1));
        assert_eq!(layout[1].content_rect, Rect::new(0, 0, 1, 1));
        assert_eq!(layout[2].rect, Rect::new(0, 1, 10, 1));
        assert_eq!(layout[2].measured_size, Size::new(1, 1));
        assert_eq!(layout[2].content_rect, Rect::new(0, 1, 1, 1));
    }
}
