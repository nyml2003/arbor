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

/// Render the entire widget tree and expose each fresh widget fragment.
///
/// This is used by shadow render-cache instrumentation. The observer is called
/// during the normal render pass, so cache validation does not render widgets a
/// second time.
pub fn render_tree_with_fragments(
    screen_size: (u16, u16),
    root: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
    mut on_fragment: impl FnMut(&WidgetNode, Rect, &VirtualScreen),
) -> VirtualScreen {
    let mut screen = VirtualScreen::new(screen_size.0, screen_size.1);
    let bg_cell = Cell {
        bg: theme.surface(),
        ..Default::default()
    };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);
    render_subtree_with_fragments(root, layout, theme, focused, &mut screen, &mut on_fragment);
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

fn render_subtree_with_fragments(
    node: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    focused: Option<WidgetId>,
    screen: &mut VirtualScreen,
    on_fragment: &mut impl FnMut(&WidgetNode, Rect, &VirtualScreen),
) {
    let info = match layout.get(&node.id()) {
        Some(i) => i,
        None => return,
    };

    if !node.is_transparent() {
        // Check if this widget is focused — pass focus hint via internal method.
        // Widgets that need focus info use the focused WidgetId to compare.
        let child_screen = node.render_with_focus(info.content_rect, theme, focused);
        on_fragment(node, info.content_rect, &child_screen);
        screen.blit(info.content_rect, &child_screen);
    }

    if !node.renders_children() {
        for child in node.children() {
            render_subtree_with_fragments(child, layout, theme, focused, screen, on_fragment);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::AnsiColor;
    use crate::layout::{LayoutProps, Size};
    use crate::widget::Widget;

    struct TestWidget {
        id: WidgetId,
        props: LayoutProps,
    }

    impl Widget for TestWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }

        fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
            let mut screen = VirtualScreen::new(rect.w, rect.h);
            screen.write_str(
                0,
                0,
                "x",
                AnsiColor::from_palette(1),
                AnsiColor::from_palette(0),
                Default::default(),
            );
            screen
        }
    }

    #[test]
    fn fragment_observer_does_not_change_render_output() {
        let root = WidgetNode::new(TestWidget {
            id: WidgetId(1),
            props: LayoutProps::default(),
        });
        let theme = Theme::dark();
        let constraints = crate::layout_engine::measure_tree(&root, Size::new(5, 1));
        let layout =
            crate::layout_engine::layout_tree(Rect::new(0, 0, 5, 1), &root, &constraints).unwrap();
        let expected = render_tree((5, 1), &root, &layout, &theme, None);
        let mut fragments = 0usize;

        let actual = render_tree_with_fragments((5, 1), &root, &layout, &theme, None, |_, _, _| {
            fragments += 1;
        });

        assert_eq!(actual, expected);
        assert_eq!(fragments, 1);
    }
}
