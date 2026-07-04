// Widget render implementations.
// Each function takes a WidgetNode + content_rect + theme and returns VirtualScreen.

use crate::cell::{Attrs, Cell};
use crate::layout::Rect;
use crate::layout_engine::LayoutResult;
use crate::screen::VirtualScreen;
use crate::text::{self, TruncateStrategy, WrapStrategy};
use crate::theme::Theme;
use crate::widget::{
    BoxWidget, ButtonStyle, ColumnWidth, InputWidget, ListWidget, ScrollViewWidget,
    TableWidget, TabsWidget, TextWidget, WidgetId, WidgetNode,
};

/// Render a WidgetNode into a VirtualScreen for the given content_rect.
pub fn render_node(node: &WidgetNode, rect: Rect, theme: &Theme) -> VirtualScreen {
    match node {
        WidgetNode::Box(w) => render_box(w, rect, theme),
        WidgetNode::Text(w) => render_text(w, rect, theme),
        WidgetNode::Input(w) => render_input(w, rect, theme),
        WidgetNode::Button(w) => render_button(w, rect, theme),
        WidgetNode::List(w) => render_list(w, rect, theme),
        WidgetNode::Table(w) => render_table(w, rect, theme),
        WidgetNode::Tabs(w) => render_tabs(w, rect, theme),
        WidgetNode::ScrollView(w) => render_scrollview(w, rect, theme),
    }
}

// ── Box ──────────────────────────────────────────────────────────

fn render_box(w: &BoxWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    // Box is a container — fill with surface color, then blit children on top.
    let mut screen = VirtualScreen::new(rect.w, rect.h);
    let bg_cell = Cell {
        bg: theme.surface(),
        ..Default::default()
    };
    screen.fill_rect(Rect::new(0, 0, rect.w, rect.h), &bg_cell);

    // Children are positioned by the layout engine — render each and blit.
    // Note: children positions are relative to parent content_rect, which
    // in a VirtualScreen maps to (0,0) as the origin.
    for child in &w.children {
        // Layout engine already computed child positions — for now,
        // default render. Full pipeline: layout_tree gives positions,
        // then render each child at its content_rect.
        let child_screen = render_node(child, Rect::new(0, 0, rect.w, rect.h), theme);
        screen.blit(Rect::new(0, 0, child_screen.cols(), child_screen.rows()), &child_screen);
    }

    screen
}

// ── Text ─────────────────────────────────────────────────────────

fn render_text(w: &TextWidget, rect: Rect, _theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
    let text_content = w.text.get();
    let expanded = text::expand_tabs(&text_content);

    match w.wrap {
        WrapStrategy::None => {
            // Split on explicit newlines, one line per \n
            for (i, line) in expanded.lines().enumerate() {
                if i as u16 >= rect.h { break; }
                let display = text::truncate(line, rect.w, w.truncate);
                screen.write_str(0, i as u16, &display, w.style.get().fg, w.style.get().bg, w.style.get().attrs);
            }
        }
        _ => {
            // Wrap to multiple lines
            let lines = text::wrap_lines(&expanded, rect.w, w.wrap);
            for (i, line) in lines.iter().enumerate() {
                if i as u16 >= rect.h {
                    break;
                }
                let display = text::truncate(line, rect.w, w.truncate);
                screen.write_str(0, i as u16, &display, w.style.get().fg, w.style.get().bg, w.style.get().attrs);
            }
        }
    }

    screen
}

// ── Input ────────────────────────────────────────────────────────

fn render_input(w: &InputWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), 1);
    let border_fg = theme.border();
    let bg = theme.surface_alt();
    let text_fg = theme.text();

    // Draw left border: "> "
    screen.write_str(0, 0, "> ", border_fg, bg, Attrs::default());

    let content_start: u16 = 2;
    let content_w = rect.w.saturating_sub(content_start);

    // Build display text
    let display = if w.password && !w.buffer.is_empty() {
        "●".repeat(w.buffer.chars().count())
    } else if w.buffer.is_empty() {
        w.placeholder.clone()
    } else {
        w.buffer.clone()
    };

    let truncated = text::truncate(&display, content_w, TruncateStrategy::End);
    screen.write_str(content_start, 0, &truncated, text_fg, bg, Attrs::default());

    screen
}

// ── Button ───────────────────────────────────────────────────────

fn render_button(w: &crate::widget::ButtonWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(3), 1);
    let (fg, bg) = match w.style {
        ButtonStyle::Primary => (theme.surface(), theme.primary()),
        ButtonStyle::Danger => (theme.surface(), theme.danger()),
        ButtonStyle::Secondary => (theme.text(), theme.surface_alt()),
        ButtonStyle::Default => (theme.text(), theme.surface_alt()),
    };
    let attrs = Attrs { bold: true, ..Default::default() };

    // "[ label ]" format
    let display = format!(" {} ", w.label.get());
    let truncated = text::truncate(&display, rect.w, TruncateStrategy::End);

    // Center the label
    let label_w = text::measure_width(&truncated);
    let offset = rect.w.saturating_sub(label_w) / 2;

    // Fill background
    let bg_cell = Cell { bg, ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, rect.w, 1), &bg_cell);

    screen.write_str(offset, 0, &truncated, fg, bg, attrs);
    screen
}

// ── List ─────────────────────────────────────────────────────────

fn render_list(w: &ListWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
    let bg = theme.surface();
    let _text_dim = theme.text_dim();
    let accent = theme.accent();
    let text = theme.text();

    // Fill background
    let bg_cell = Cell { bg, ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, rect.w, rect.h), &bg_cell);

    let visible_count = rect.h as usize;
    let start = w.scroll_offset;
    let end = (start + visible_count).min(w.items.len());

    for (i, item_idx) in (start..end).enumerate() {
        let row = i as u16;
        let is_selected = w.selected == Some(item_idx);

        let (fg, row_bg) = if is_selected {
            (theme.surface(), accent)
        } else {
            (text, bg)
        };

        // Row background for selected
        if is_selected {
            let sel_cell = Cell { bg: row_bg, ..Default::default() };
            screen.fill_rect(Rect::new(0, row, rect.w, 1), &sel_cell);
        }

        // Item text — use custom render_item if provided, else use pre-rendered items
        let item_text = if let Some(ref render) = w.render_item {
            render(item_idx, is_selected)
        } else {
            w.items[item_idx].clone()
        };
        let display = text::truncate(&item_text, rect.w, TruncateStrategy::End);
        screen.write_str(1, row, &display, fg, row_bg, Attrs::default());
    }

    // Scroll indicator
    if w.items.len() > visible_count {
        let pct = start as f64 / w.items.len() as f64;
        let bar_y = (pct * (rect.h - 1) as f64) as u16;
        if let Some(cell) = screen.cell_at_mut(rect.w.saturating_sub(1), bar_y) {
            cell.fg = accent;
            cell.bg = accent;
            cell.ch = ' ';
        }
    }

    screen
}

// ── Table ────────────────────────────────────────────────────────

fn render_table(w: &TableWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
    let bg = theme.surface();
    let header_bg = theme.surface_alt();
    let border_fg = theme.border();
    let text = theme.text();
    let accent = theme.accent();

    let bg_cell = Cell { bg, ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, rect.w, rect.h), &bg_cell);

    if rect.h == 0 { return screen; }

    // Header row
    let mut col_x: u16 = 0;
    for (ci, col) in w.columns.iter().enumerate() {
        let col_w = resolve_col_width(col.width, rect.w, &w.columns, ci);
        let header_text = text::truncate(&col.header, col_w, TruncateStrategy::End);
        screen.write_str(col_x, 0, &header_text, text, header_bg, Attrs { bold: true, ..Default::default() });

        // Header bg
        let hdr_cell = Cell { bg: header_bg, ..Default::default() };
        screen.fill_rect(Rect::new(col_x, 0, col_w, 1), &hdr_cell);
        screen.write_str(col_x, 0, &header_text, text, header_bg, Attrs { bold: true, ..Default::default() });

        col_x += col_w;
    }

    // Separator
    let sep_cell = Cell { bg: border_fg, ..Default::default() };
    screen.fill_rect(Rect::new(0, 1, rect.w, 1), &sep_cell);

    // Data rows
    let data_start: u16 = 2;
    let visible_rows = (rect.h.saturating_sub(data_start)) as usize;
    let start = w.scroll_offset;
    let end = (start + visible_rows).min(w.cells.len());

    for (i, row_idx) in (start..end).enumerate() {
        let screen_row = data_start + i as u16;
        let is_selected = w.selected == Some(row_idx);

        let row_bg = if is_selected { accent } else { bg };
        let row_fg = if is_selected { theme.surface() } else { text };

        let mut cx: u16 = 0;
        for ci in 0..w.columns.len() {
            let col_w = resolve_col_width(w.columns[ci].width, rect.w, &w.columns, ci);
            let cell_text = if let Some(ref render) = w.render_cell {
                render(row_idx, ci)
            } else if ci < w.cells.get(row_idx).map_or(0, |r| r.len()) {
                w.cells[row_idx][ci].clone()
            } else {
                String::new()
            };
            let display = text::truncate(&cell_text, col_w, TruncateStrategy::End);
            screen.write_str(cx, screen_row, &display, row_fg, row_bg, Attrs::default());
            cx += col_w;
        }
    }

    screen
}

fn resolve_col_width(col: ColumnWidth, total_w: u16, all_cols: &[crate::widget::ColumnDef], _idx: usize) -> u16 {
    match col {
        ColumnWidth::Fixed(w) => w,
        ColumnWidth::Flex(_) => {
            let fixed_total: u16 = all_cols.iter().filter_map(|c| match c.width {
                ColumnWidth::Fixed(w) => Some(w),
                _ => None,
            }).sum();
            let flex_count = all_cols.iter().filter(|c| matches!(c.width, ColumnWidth::Flex(_))).count() as u16;
            let remaining = total_w.saturating_sub(fixed_total);
            if flex_count > 0 { remaining / flex_count } else { 0 }
        }
    }
}

// ── Tabs ─────────────────────────────────────────────────────────

fn render_tabs(w: &TabsWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
    let _bg = theme.surface();
    let tab_bg = theme.surface_alt();
    let active_bg = theme.primary();
    let text = theme.text();
    let active_text = theme.surface();

    // Tab headers
    let mut cx: u16 = 0;
    let header_h: u16 = 1;
    let body_top: u16 = 2; // header + separator

    for (i, tab) in w.tabs.iter().enumerate() {
        let label = format!(" {} ", tab.label);
        let label_w = text::measure_width(&label);
        let is_active = i == w.active;

        let (fg, row_bg) = if is_active {
            (active_text, active_bg)
        } else {
            (text, tab_bg)
        };

        let cell = Cell { bg: row_bg, ..Default::default() };
        screen.fill_rect(Rect::new(cx, 0, label_w, 1), &cell);
        screen.write_str(cx, 0, &label, fg, row_bg, Attrs::default());
        cx += label_w;
    }

    // Separator
    let sep_cell = Cell { bg: theme.border(), ..Default::default() };
    screen.fill_rect(Rect::new(0, header_h, rect.w, 1), &sep_cell);

    // Active tab content
    if w.active < w.tabs.len() {
        let body_rect = Rect::new(0, body_top, rect.w, rect.h.saturating_sub(body_top));
        let content_screen = render_node(&w.tabs[w.active].content, body_rect, theme);
        screen.blit(Rect::new(0, body_top, content_screen.cols(), content_screen.rows()), &content_screen);
    }

    screen
}

// ── ScrollView ───────────────────────────────────────────────────

fn render_scrollview(w: &ScrollViewWidget, rect: Rect, theme: &Theme) -> VirtualScreen {
    let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));

    // Child renders at its full size; we blit the visible portion
    // offset by scroll_x, scroll_y
    let child_rect = Rect::new(0, 0, rect.w.max(100), rect.h.max(100));
    let child_screen = render_node(&w.child, child_rect, theme);

    // Blit visible viewport
    let _src_rect = Rect::new(w.scroll_x.get(), w.scroll_y.get(), rect.w, rect.h);
    // Clip to child_screen bounds
    let copy_w = rect.w.min(child_screen.cols().saturating_sub(w.scroll_x.get()));
    let copy_h = rect.h.min(child_screen.rows().saturating_sub(w.scroll_y.get()));

    for row in 0..copy_h {
        for col in 0..copy_w {
            let src_cell = child_screen.cell_at(w.scroll_x.get() + col, w.scroll_y.get() + row);
            if let Some(dest) = screen.cell_at_mut(col, row) {
                *dest = src_cell;
            }
        }
    }

    screen
}

// ── Tree-level render ────────────────────────────────────────────

/// Render the entire widget tree using layout results.
/// Walks the LayoutResult, renders each widget at its content_rect,
/// and blits it into a full-screen VirtualScreen.
pub fn render_tree(
    screen_size: (u16, u16),
    root: &WidgetNode,
    layout: &LayoutResult,
    theme: &Theme,
) -> VirtualScreen {
    let mut screen = VirtualScreen::new(screen_size.0, screen_size.1);

    // Fill background
    let bg_cell = Cell { bg: theme.surface(), ..Default::default() };
    screen.fill_rect(Rect::new(0, 0, screen_size.0, screen_size.1), &bg_cell);

    render_subtree(root, layout, theme, &mut screen);

    screen
}

/// Render a widget and its children into the shared screen at their layout positions.
fn render_subtree(
    node: &WidgetNode,
    layout: &LayoutResult,
    theme: &Theme,
    screen: &mut VirtualScreen,
) {
    // Get this widget's layout info
    let info = match layout.widgets.get(&node.id()) {
        Some(i) => i,
        None => return,
    };

    // Render this widget into a temp screen, then blit at content_rect
    match node {
        WidgetNode::Box(_) => {
            // Box is a container — no visual of its own, just render children
            for child in node.children() {
                render_subtree(child, layout, theme, screen);
            }
        }
        WidgetNode::ScrollView(_w) => {
            let child_screen = render_node(node, info.content_rect, theme);
            screen.blit(info.content_rect, &child_screen);
        }
        _ => {
            // Leaf widgets: render to temp screen, blit to position
            let child_screen = render_node(node, info.content_rect, theme);
            screen.blit(info.content_rect, &child_screen);

            // Recurse into children (Tabs, List have children)
            for child in node.children() {
                render_subtree(child, layout, theme, screen);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutProps;
    use crate::widget::{ButtonWidget, TextWidget, TextStyle};

    fn make_theme() -> Theme {
        Theme::dark()
    }

    fn text_signal(s: &str) -> crate::signal::ReadSignal<String> {
        crate::signal::ReadSignal::constant(s.to_string())
    }

    #[test]
    fn render_text_single_line() {
        let w = TextWidget {
            id: WidgetId(1),
            props: LayoutProps::default(),
            text: text_signal("hello"),
            style: crate::signal::ReadSignal::constant(TextStyle::default()),
            wrap: WrapStrategy::None,
            truncate: TruncateStrategy::End,
        };
        let node = WidgetNode::Text(w);
        let screen = render_node(&node, Rect::new(0, 0, 80, 1), &make_theme());
        assert_eq!(screen.cell_at(0, 0).ch, 'h');
        assert_eq!(screen.cell_at(4, 0).ch, 'o');
    }

    #[test]
    fn render_text_wraps_multi_line() {
        let w = TextWidget {
            id: WidgetId(1),
            props: LayoutProps::default(),
            text: text_signal("abcd efgh ijkl"),
            style: crate::signal::ReadSignal::constant(TextStyle::default()),
            wrap: WrapStrategy::Word,
            truncate: TruncateStrategy::End,
        };
        let node = WidgetNode::Text(w);
        let screen = render_node(&node, Rect::new(0, 0, 5, 10), &make_theme());
        // Should wrap at word boundaries within 5 cols
        assert!(screen.rows() >= 3);
    }

    #[test]
    fn render_tree_with_box_and_text() {
        use crate::layout::{LayoutProps, Size};
        use crate::layout_engine::{layout_tree, measure_tree};
        use crate::widget::{BoxWidget, TextWidget, TextStyle};

        let theme = make_theme();
        let root = WidgetNode::Box(BoxWidget {
            id: WidgetId(10),
            props: LayoutProps { direction: crate::layout::Direction::Column, ..Default::default() },
            children: vec![
                WidgetNode::Text(TextWidget {
                    id: WidgetId(1),
                    props: LayoutProps::default(),
                    text: text_signal("hello"),
                    style: crate::signal::ReadSignal::constant(TextStyle::default()),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::Text(TextWidget {
                    id: WidgetId(2),
                    props: LayoutProps::default(),
                    text: text_signal("world"),
                    style: crate::signal::ReadSignal::constant(TextStyle::default()),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
            ],
        });

        let constraints = measure_tree(&root, Size::new(80, 24));
        let layout = layout_tree(Rect::new(0, 0, 80, 24), &root, &constraints);
        let screen = render_tree((80, 24), &root, &layout, &theme);

        // Both texts should be visible
        assert_eq!(screen.cell_at(0, 0).ch, 'h');
        // Second text should be on a different row
        let t2_info = &layout.widgets[&WidgetId(2)];
        assert!(t2_info.content_rect.y > 0);
        assert_eq!(screen.cell_at(t2_info.content_rect.x, t2_info.content_rect.y).ch, 'w');
    }

    #[test]
    fn render_button_draws_label() {
        let w = ButtonWidget {
            id: WidgetId(2),
            props: LayoutProps::default(),
            label: crate::signal::ReadSignal::constant("OK".to_string()),
            style: ButtonStyle::Primary,
            on_click: None,
        };
        let node = WidgetNode::Button(w);
        let screen = render_node(&node, Rect::new(0, 0, 20, 1), &make_theme());
        // Screen should contain "OK" somewhere
        let mut found = false;
        for row in 0..screen.rows() {
            for col in 0..screen.cols() {
                if screen.cell_at(col, row).ch == 'O' {
                    found = true;
                }
            }
        }
        assert!(found);
    }
}
