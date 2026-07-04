// Render engine — generic tree rendering via the Widget trait.
// Zero per-type dispatch. Each widget's render() method handles its own visuals.
// Focus info is passed through so widgets can show cursor/selection.

use arbor_tui_primitives::cell::Cell;
use arbor_tui_primitives::layout::Rect;
use arbor_tui_primitives::widget_id::{WidgetId, WidgetLayoutInfo};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use crate::widget::WidgetNode;

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
    let bg_cell = Cell { bg: theme.surface(), ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);
    render_subtree(root, layout, theme, focused, &mut screen);
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

// Extend WidgetNode with a focus-aware render method.
// This avoids changing the Widget trait signature while allowing
// individual widgets to check focus.
impl WidgetNode {
    fn render_with_focus(&self, rect: Rect, theme: &Theme, focused: Option<WidgetId>) -> VirtualScreen {
        // Check if this node is the focused one, and use a specialized
        // render path for widgets that support focus indication.
        if focused == Some(self.id()) {
            self.inner().render_focused(rect, theme)
        } else {
            self.render(rect, theme)
        }
    }
}
