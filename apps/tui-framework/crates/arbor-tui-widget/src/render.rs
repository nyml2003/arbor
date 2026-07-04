// Render engine — generic tree rendering via the Widget trait.
// Zero per-type dispatch. Each widget's render() method handles its own visuals.

use arbor_tui_primitives::cell::Cell;
use arbor_tui_primitives::layout::Rect;
use arbor_tui_primitives::widget_id::{WidgetId, WidgetLayoutInfo};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use crate::widget::WidgetNode;

use std::collections::HashMap;

/// The result of the layout phase, mapping WidgetId → layout info.
pub type LayoutResult = HashMap<WidgetId, WidgetLayoutInfo>;

/// Render the entire widget tree using layout results.
/// Walks the tree, calls each widget's render(), and blits into a full-screen buffer.
pub fn render_tree(
    screen_size: (u16, u16),
    root: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
) -> VirtualScreen {
    let mut screen = VirtualScreen::new(screen_size.0, screen_size.1);
    let bg_cell = Cell { bg: theme.surface(), ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);
    render_subtree(root, layout, theme, &mut screen);
    screen
}

/// Recursively render a widget and its children into the shared screen.
fn render_subtree(
    node: &WidgetNode,
    layout: &HashMap<WidgetId, WidgetLayoutInfo>,
    theme: &Theme,
    screen: &mut VirtualScreen,
) {
    let info = match layout.get(&node.id()) {
        Some(i) => i,
        None => return,
    };

    // Render self (unless transparent container)
    if !node.is_transparent() {
        let child_screen = node.render(info.content_rect, theme);
        screen.blit(info.content_rect, &child_screen);
    }

    // Recurse into children (unless widget renders them itself, e.g. ScrollView)
    if !node.renders_children() {
        for child in node.children() {
            render_subtree(child, layout, theme, screen);
        }
    }
}
