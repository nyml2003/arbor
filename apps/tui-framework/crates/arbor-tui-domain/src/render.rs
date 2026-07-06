// Render engine — generic tree rendering via the Widget trait.
// Zero per-type dispatch. Each widget's render() method handles its own visuals.
// Focus info is passed through so widgets can show cursor/selection.

use crate::cell::Cell;
use crate::layout::Rect;
use crate::screen::VirtualScreen;
use crate::theme::Theme;
use crate::widget::WidgetNode;
use crate::widget_id::{WidgetId, WidgetLayoutInfo};

use std::collections::HashMap;

/// Render the entire widget tree using layout results.
pub fn render_tree(
    screen_size: (u16, u16),
    root: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
) -> VirtualScreen {
    let mut screen = VirtualScreen::new(screen_size.0, screen_size.1);
    let bg_cell = Cell {
        bg: theme.surface(),
        ..Default::default()
    };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);
    render_subtree(root, layout, theme, focused, &mut screen);
    screen
}

/// Render only the portion of a tree that intersects `viewport`.
///
/// Layout coordinates remain in the child's full content space. The returned
/// screen uses viewport-local coordinates, so callers can render a large child
/// tree into a small scroll window without allocating the full child height.
pub fn render_tree_viewport(
    screen_size: (u16, u16),
    root: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
    viewport: Rect,
) -> VirtualScreen {
    let mut screen = VirtualScreen::new(screen_size.0, screen_size.1);
    let bg_cell = Cell {
        bg: theme.surface(),
        ..Default::default()
    };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);
    render_subtree_viewport(root, layout, theme, focused, viewport, &mut screen);
    screen
}

fn render_subtree(
    node: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
    screen: &mut VirtualScreen,
) {
    let info = match layout.get(&node.id()) {
        Some(i) => i,
        None => return,
    };

    if !node.is_transparent() {
        // Check if this widget is focused — pass focus hint via internal method.
        // Widgets that need focus info use the focused WidgetId to compare.
        let child_screen = node.render_with_focus(info.content_rect, theme, focused);
        screen.blit(info.content_rect, &child_screen);
    }

    if !node.renders_children() {
        for child in node.children() {
            render_subtree(child, layout, theme, focused, screen);
        }
    }
}

fn render_subtree_viewport(
    node: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
    viewport: Rect,
    screen: &mut VirtualScreen,
) {
    let info = match layout.get(&node.id()) {
        Some(i) => i,
        None => return,
    };

    if !node.is_transparent() {
        if let Some(visible) = intersect(info.content_rect, viewport) {
            let child_screen = node.render_with_focus(info.content_rect, theme, focused);
            let dest = Rect::new(
                visible.x.saturating_sub(viewport.x),
                visible.y.saturating_sub(viewport.y),
                visible.w,
                visible.h,
            );
            let source_origin = (
                visible.x.saturating_sub(info.content_rect.x),
                visible.y.saturating_sub(info.content_rect.y),
            );
            screen.blit_region(dest, &child_screen, source_origin);
        }
    }

    if !node.renders_children() {
        for child in node.children() {
            render_subtree_viewport(child, layout, theme, focused, viewport, screen);
        }
    }
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.x.saturating_add(a.w).min(b.x.saturating_add(b.w));
    let y1 = a.y.saturating_add(a.h).min(b.y.saturating_add(b.h));

    if x0 >= x1 || y0 >= y1 {
        return None;
    }

    Some(Rect::new(x0, y0, x1 - x0, y1 - y0))
}
