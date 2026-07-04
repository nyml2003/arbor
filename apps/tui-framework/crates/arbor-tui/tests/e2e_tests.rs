// End-to-end tests — simulate real user interactions through the full pipeline.
// Each test: build tree → mount → perform actions → render → verify screen content.

use arbor_tui_primitives::cell::{Attrs, Cell, Span};
use arbor_tui_primitives::input::KeyHandleResult;
use arbor_tui_primitives::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_primitives::widget_id::{WidgetAction, WidgetId};
use arbor_tui_render::diff::{diff, merge_regions};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_widget::focus::{mount_tree, FocusManager, find_widget_mut};
use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
use arbor_tui_widget::render::render_tree;
use arbor_tui_widget::widget::{Widget, WidgetNode};
use arbor_tui_widgets::box_widget::BoxWidget;
use arbor_tui_widgets::button_widget::{ButtonStyle, ButtonWidget};
use arbor_tui_widgets::input_widget::InputWidget;
use arbor_tui_widgets::list_widget::ListWidget;
use arbor_tui_widgets::rich_text::RichTextWidget;
use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
use arbor_tui_backend::simulated_backend::SimulatedBackend;

// ── Helpers ────────────────────────────────────────────────────────

fn render_and_emit(root: &WidgetNode, size: (u16, u16), backend: &mut SimulatedBackend, focus: Option<WidgetId>) {
    let cs = measure_tree(root, Size::new(size.0, size.1));
    let lo = layout_tree(Rect::new(0, 0, size.0, size.1), root, &cs).unwrap();
    let sc = render_tree(size, root, &lo, &Theme::dark(), focus);
    let mut rs = diff(&VirtualScreen::new(size.0, size.1), &sc);
    merge_regions(&mut rs);
    if !rs.is_empty() {
        backend.emit(&rs, &sc).unwrap();
    }
}

fn screen_contains(screen: &VirtualScreen, needle: &str) -> bool {
    for row in 0..screen.rows() {
        let line: String = (0..screen.cols()).map(|c| screen.cell_at(c, row).ch).collect();
        if line.contains(needle) { return true; }
    }
    false
}

fn cell_at(screen: &VirtualScreen, col: u16, row: u16) -> Cell {
    screen.cell_at(col, row)
}

// ── Input E2E ──────────────────────────────────────────────────────

#[test]
fn input_typing_renders_on_screen() {
    let mut input = InputWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        buffer: String::new(), cursor: 0,
        placeholder: "type here".into(), password: false,
        on_change: None, on_submit: None,
    };
    // Simulate typing "hi"
    assert_eq!(input.perform(&WidgetAction::TypeChar('h')), KeyHandleResult::Handled);
    assert_eq!(input.perform(&WidgetAction::TypeChar('i')), KeyHandleResult::Handled);
    assert_eq!(input.buffer, "hi");
    assert_eq!(input.cursor, 2);

    // Render and verify
    let root = WidgetNode::new(input);
    let mut be = SimulatedBackend::new(40, 5);
    render_and_emit(&root, (40, 5), &mut be, Some(WidgetId(1)));
    assert!(screen_contains(be.screen(), "> hi"), "screen should show '> hi'");
}

#[test]
fn input_backspace_deletes_char() {
    let mut input = InputWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        buffer: "abc".into(), cursor: 3,
        placeholder: "".into(), password: false,
        on_change: None, on_submit: None,
    };
    assert_eq!(input.perform(&WidgetAction::Backspace), KeyHandleResult::Handled);
    assert_eq!(input.buffer, "ab");
    assert_eq!(input.cursor, 2);
}

#[test]
fn input_cursor_shows_on_focused_render() {
    let input = InputWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        buffer: "x".into(), cursor: 1,
        placeholder: "".into(), password: false,
        on_change: None, on_submit: None,
    };
    let root = WidgetNode::new(input);
    let mut be = SimulatedBackend::new(40, 5);
    // Pass focus — render_focused should show cursor with inverted colors
    render_and_emit(&root, (40, 5), &mut be, Some(WidgetId(1)));
    // "> x" — 'x' is at col 2 (after "> ")
    let cell = cell_at(be.screen(), 2, 0);
    assert_eq!(cell.ch, 'x', "typed char should be at col 2");
    // Cursor at position 1 (past the char) — col 3 should be a space with cursor bg
    let cursor_cell = cell_at(be.screen(), 3, 0);
    assert_eq!(cursor_cell.bg.palette.0, Theme::dark().primary().palette.0, "cursor bg should be primary color");
}

#[test]
fn input_unfocused_no_cursor() {
    let input = InputWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        buffer: "x".into(), cursor: 1,
        placeholder: "".into(), password: false,
        on_change: None, on_submit: None,
    };
    let root = WidgetNode::new(input);
    let mut be = SimulatedBackend::new(40, 5);
    // No focus — regular render, no cursor highlight
    render_and_emit(&root, (40, 5), &mut be, None);
    // 'x' at col 2 should have surface_alt bg (default input bg)
    let cell = cell_at(be.screen(), 2, 0);
    assert_eq!(cell.ch, 'x');
    assert_eq!(cell.bg.palette.0, Theme::dark().surface_alt().palette.0, "unfocused bg should be surface_alt");
}

// ── Button E2E ─────────────────────────────────────────────────────

#[test]
fn button_click_fires_callback() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let clicked = Rc::new(RefCell::new(false));
    let c = clicked.clone();
    let mut btn = ButtonWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        label: ReadSignal::constant("OK".into()),
        style: ButtonStyle::Primary,
        on_click: Some(Box::new(move || *c.borrow_mut() = true)),
    };
    assert_eq!(btn.perform(&WidgetAction::Activate), KeyHandleResult::Handled);
    assert!(*clicked.borrow());
}

#[test]
fn button_renders_label() {
    let btn = ButtonWidget {
        id: WidgetId(1), props: LayoutProps { width: Some(20), ..Default::default() },
        label: ReadSignal::constant("OK".into()),
        style: ButtonStyle::Primary,
        on_click: None,
    };
    let root = WidgetNode::new(btn);
    let mut be = SimulatedBackend::new(40, 5);
    render_and_emit(&root, (40, 5), &mut be, None);
    assert!(screen_contains(be.screen(), "OK"), "button should show label");
}

// ── List E2E ───────────────────────────────────────────────────────

#[test]
fn list_navigate_down_selects_next() {
    let mut list = ListWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        items: vec!["a".into(), "b".into(), "c".into()],
        selected: None, scroll_offset: 0,
        on_select: None, on_scroll: None, render_item: None,
    };
    assert_eq!(list.perform(&WidgetAction::NavigateDown), KeyHandleResult::Handled);
    assert_eq!(list.selected, Some(0));
    assert_eq!(list.perform(&WidgetAction::NavigateDown), KeyHandleResult::Handled);
    assert_eq!(list.selected, Some(1));
}

#[test]
fn list_navigate_up_from_none_does_nothing() {
    let mut list = ListWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        items: vec!["a".into(), "b".into()],
        selected: None, scroll_offset: 0,
        on_select: None, on_scroll: None, render_item: None,
    };
    assert_eq!(list.perform(&WidgetAction::NavigateUp), KeyHandleResult::Handled);
    assert_eq!(list.selected, None);
}

// ── Focus Manager E2E ──────────────────────────────────────────────

#[test]
fn focus_tab_cycles_through_inputs() {
    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps::default(),
        children: vec![
            WidgetNode::new(InputWidget {
                id: WidgetId(10), props: LayoutProps::default(),
                buffer: String::new(), cursor: 0,
                placeholder: "a".into(), password: false,
                on_change: None, on_submit: None,
            }),
            WidgetNode::new(InputWidget {
                id: WidgetId(20), props: LayoutProps::default(),
                buffer: String::new(), cursor: 0,
                placeholder: "b".into(), password: false,
                on_change: None, on_submit: None,
            }),
        ],
    });
    let mut fm = FocusManager::new();
    fm.rebuild(&root);
    assert_eq!(fm.len(), 2);
    assert_eq!(fm.next().unwrap(), Some(WidgetId(10)));
    assert_eq!(fm.next().unwrap(), Some(WidgetId(20)));
    assert_eq!(fm.next().unwrap(), Some(WidgetId(10))); // wrap around
}

#[test]
fn focus_prev_wraps_to_last() {
    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps::default(),
        children: vec![
            WidgetNode::new(InputWidget {
                id: WidgetId(10), props: LayoutProps::default(),
                buffer: String::new(), cursor: 0,
                placeholder: "".into(), password: false,
                on_change: None, on_submit: None,
            }),
            WidgetNode::new(InputWidget {
                id: WidgetId(20), props: LayoutProps::default(),
                buffer: String::new(), cursor: 0,
                placeholder: "".into(), password: false,
                on_change: None, on_submit: None,
            }),
        ],
    });
    let mut fm = FocusManager::new();
    fm.rebuild(&root);
    assert_eq!(fm.prev().unwrap(), Some(WidgetId(20)));
}

// ── Rich Text E2E ──────────────────────────────────────────────────

#[test]
fn rich_text_renders_spans_with_colors() {
    let spans = vec![
        Span::new("Hello", arbor_tui_primitives::cell::AnsiColor::from_palette(196), arbor_tui_primitives::cell::AnsiColor::from_palette(0), Attrs { bold: true, ..Default::default() }),
        Span::new(" World", arbor_tui_primitives::cell::AnsiColor::from_palette(46), arbor_tui_primitives::cell::AnsiColor::from_palette(0), Attrs::default()),
    ];
    let widget = RichTextWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        lines: vec![spans],
        clip: false,
    };
    let root = WidgetNode::new(widget);
    let mut be = SimulatedBackend::new(40, 5);
    render_and_emit(&root, (40, 5), &mut be, None);
    // "Hello" in red (196) should be visible
    let h_cell = cell_at(be.screen(), 0, 0);
    assert_eq!(h_cell.fg.palette.0, 196, "'H' should be red (palette 196)");
    assert!(h_cell.attrs.bold, "'H' should be bold");
    // " World" in green (46)
    let w_cell = cell_at(be.screen(), 5, 0);
    assert_eq!(w_cell.fg.palette.0, 46, "'W' should be green (palette 46)");
}

// ── Focus dispatch E2E ─────────────────────────────────────────────

#[test]
fn dispatch_action_to_focused_input() {
    let mut root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps::default(),
        children: vec![
            WidgetNode::new(InputWidget {
                id: WidgetId(1), props: LayoutProps::default(),
                buffer: String::new(), cursor: 0,
                placeholder: "".into(), password: false,
                on_change: None, on_submit: None,
            }),
        ],
    });
    let mut fm = FocusManager::new();
    fm.rebuild(&root);
    assert_eq!(fm.next().unwrap(), Some(WidgetId(1)));

    // Dispatch TypeChar to focused widget
    let target = fm.current().unwrap();
    if let Some(w) = find_widget_mut(&mut root, target) {
        assert_eq!(w.perform(&WidgetAction::TypeChar('X')), KeyHandleResult::Handled);
    }
    // Verify buffer changed
    if let Some(_w) = find_widget_mut(&mut root, WidgetId(1)) {
        // verified via render below
    }
    // Render and verify
    let mut be = SimulatedBackend::new(40, 5);
    render_and_emit(&root, (40, 5), &mut be, Some(WidgetId(1)));
    assert!(screen_contains(be.screen(), "> X"), "typed 'X' should appear on screen");
}

// ── Full pipeline: layout positions are consistent ─────────────────

#[test]
fn layout_positions_descend_in_column() {
    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps { direction: Direction::Column, padding: RectOffset::all(1), ..Default::default() },
        children: vec![
            WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("Line1".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps::default(), text: ReadSignal::constant("Line2".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        ],
    });
    let cs = measure_tree(&root, Size::new(80, 24));
    let lo = layout_tree(Rect::new(0, 0, 80, 24), &root, &cs).unwrap();
    let y1 = lo[&WidgetId(1)].content_rect.y;
    let y2 = lo[&WidgetId(2)].content_rect.y;
    assert!(y2 > y1, "second child should be below first: {y1} vs {y2}");
}

#[test]
fn flex_child_gets_remaining_space() {
    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps { direction: Direction::Column, width: Some(80), height: Some(20), ..Default::default() },
        children: vec![
            WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("top".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps { flex: 1.0, ..Default::default() }, text: ReadSignal::constant("fill".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(3), props: LayoutProps::default(), text: ReadSignal::constant("bottom".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        ],
    });
    let cs = measure_tree(&root, Size::new(80, 20));
    let lo = layout_tree(Rect::new(0, 0, 80, 20), &root, &cs).unwrap();
    let flex_h = lo[&WidgetId(2)].content_rect.h;
    assert!(flex_h > 1, "flex child should get >1 row, got {flex_h}");
}

// ── Mount/unmount lifecycle ────────────────────────────────────────

#[test]
fn mount_tree_calls_on_mount_for_every_widget() {
    use std::rc::Rc;
    use std::cell::Cell;
    let count = Rc::new(Cell::new(0u32));
    struct CountingWidget { id: WidgetId, props: LayoutProps, count: Rc<Cell<u32>> }
    impl Widget for CountingWidget {
        fn id(&self) -> WidgetId { self.id }
        fn layout_props(&self) -> &LayoutProps { &self.props }
        fn on_mount(&mut self) { self.count.set(self.count.get() + 1); }
    }
    let c = count.clone();
    let mut root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps::default(),
        children: vec![
            WidgetNode::new(CountingWidget { id: WidgetId(1), props: Default::default(), count: c.clone() }),
            WidgetNode::new(CountingWidget { id: WidgetId(2), props: Default::default(), count: c.clone() }),
        ],
    });
    mount_tree(&mut root);
    assert_eq!(count.get(), 2, "both children should be mounted");
}

// ── Table render ───────────────────────────────────────────────────

#[test]
fn table_renders_header_and_data() {
    use arbor_tui_widgets::table_widget::{ColumnDef, ColumnWidth, TableWidget};
    let tbl = TableWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        columns: vec![
            ColumnDef { header: "Name".into(), width: ColumnWidth::Fixed(10) },
            ColumnDef { header: "Age".into(), width: ColumnWidth::Fixed(5) },
        ],
        cells: vec![vec!["Alice".into(), "30".into()], vec!["Bob".into(), "25".into()]],
        selected: None, scroll_offset: 0,
        on_select: None, on_scroll: None, render_cell: None,
    };
    let root = WidgetNode::new(tbl);
    let mut be = SimulatedBackend::new(40, 10);
    render_and_emit(&root, (40, 10), &mut be, None);
    assert!(screen_contains(be.screen(), "Name"), "table header should show 'Name'");
    assert!(screen_contains(be.screen(), "Alice"), "table should show 'Alice'");
}

// ── Resize: blank screen produces full coverage ────────────────────

#[test]
fn resize_to_blank_then_render_covers_all_content() {
    let root = WidgetNode::new(TextWidget {
        id: WidgetId(1), props: LayoutProps::default(),
        text: ReadSignal::constant("hello world".into()),
        style: ReadSignal::constant(TextStyle::default()),
        wrap: WrapStrategy::None, truncate: TruncateStrategy::End,
    });
    let cs = measure_tree(&root, Size::new(80, 24));
    let lo = layout_tree(Rect::new(0, 0, 80, 24), &root, &cs).unwrap();
    let sc = render_tree((80, 24), &root, &lo, &Theme::dark(), None);
    // Simulate resize: blank canvas
    let blank = VirtualScreen::new(80, 24);
    let mut regions = diff(&blank, &sc);
    merge_regions(&mut regions);
    assert!(!regions.is_empty(), "blank→content must produce dirty regions");
    // Every cell of "hello world" at (0,0) should be in a dirty region
    let text_cols_dirty = regions.iter().any(|r| r.row == 0 && r.start_col <= 0 && 0 < r.end_col);
    assert!(text_cols_dirty, "first char should be in dirty region");
}

// ── Diff: same screens produce no regions ──────────────────────────

#[test]
fn diff_identical_screens_is_empty() {
    let sc = VirtualScreen::new(80, 24);
    let regions = diff(&sc, &sc);
    assert!(regions.is_empty(), "identical screens should have no diff");
}

#[test]
fn diff_single_char_change_is_detected() {
    let old = VirtualScreen::new(10, 5);
    let mut new = VirtualScreen::new(10, 5);
    new.cell_at_mut(3, 1).unwrap().ch = 'X';
    let mut rs = diff(&old, &new);
    merge_regions(&mut rs);
    assert_eq!(rs.len(), 1);
    assert_eq!(rs[0].row, 1);
    assert_eq!(rs[0].start_col, 3);
    assert_eq!(rs[0].end_col, 4);
}
