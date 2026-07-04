// Integration test — full pipeline: measure → layout → render → diff → emit.
// Uses SimulatedBackend, no real terminal needed.

use arbor_tui_core::diff::{diff, merge_regions};
use arbor_tui_core::dirty::DirtyTracker;
use arbor_tui_core::focus::mount_tree;
use arbor_tui_core::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
use arbor_tui_core::layout_engine::{layout_tree, measure_tree};
use arbor_tui_core::render::render_tree;
use arbor_tui_core::signal::{ReadSignal, Signal};
use arbor_tui_core::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::{
    BoxWidget, TextWidget, TextStyle, WidgetId, WidgetNode,
};
use arbor_tui_core::backend::TerminalBackend;
use arbor_tui_core::screen::VirtualScreen;
use arbor_tui_backend::simulated_backend::SimulatedBackend;

fn text_signal(s: &str) -> ReadSignal<String> {
    ReadSignal::constant(s.to_string())
}

#[test]
fn full_pipeline_box_with_two_texts() {
    let theme = Theme::dark();
    let (cols, rows) = (80u16, 24u16);

    // Build component tree
    let root = WidgetNode::Box(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps {
            direction: Direction::Column,
            padding: RectOffset::all(1),
            ..Default::default()
        },
        children: vec![
            WidgetNode::Text(TextWidget {
                id: WidgetId(1),
                props: LayoutProps::default(),
                text: text_signal("Hello World"),
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            WidgetNode::Text(TextWidget {
                id: WidgetId(2),
                props: LayoutProps::default(),
                text: text_signal("Second line"),
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    });

    // Pipeline
    let constraints = measure_tree(&root, Size::new(cols, rows));
    let layout = layout_tree(Rect::new(0, 0, cols, rows), &root, &constraints);
    let screen = render_tree((cols, rows), &root, &layout, &theme);

    // Verify output
    assert_eq!(screen.cols(), cols);
    assert_eq!(screen.rows(), rows);
    // First text at its layout position (Box has padding, don't assume 0,0)
    let t1_info = &layout.widgets[&WidgetId(1)];
    assert_eq!(screen.cell_at(t1_info.content_rect.x, t1_info.content_rect.y).ch, 'H');
}

#[test]
fn diff_detects_change_after_counter_increment() {
    let theme = Theme::dark();
    let (cols, rows) = (80u16, 24u16);

    let make_text = |id: u64, text: &str| -> WidgetNode {
        WidgetNode::Text(TextWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            text: text_signal(text),
            style: ReadSignal::constant(TextStyle::default()),
            wrap: WrapStrategy::None,
            truncate: TruncateStrategy::End,
        })
    };

    // Frame 1: Count: 0
    let root1 = make_text(1, "Count: 0");
    let c1 = measure_tree(&root1, Size::new(cols, rows));
    let l1 = layout_tree(Rect::new(0, 0, cols, rows), &root1, &c1);
    let screen1 = render_tree((cols, rows), &root1, &l1, &theme);

    // Frame 2: Count: 1
    let root2 = make_text(1, "Count: 1");
    let c2 = measure_tree(&root2, Size::new(cols, rows));
    let l2 = layout_tree(Rect::new(0, 0, cols, rows), &root2, &c2);
    let screen2 = render_tree((cols, rows), &root2, &l2, &theme);

    // Diff should detect the change
    let mut regions = diff(&screen1, &screen2);
    merge_regions(&mut regions);
    assert!(!regions.is_empty(), "diff should detect the counter change");

    // The changed region should include col 7 (where '0'→'1')
    let changed_cols: Vec<_> = regions.iter()
        .filter(|r| r.row == 0)
        .flat_map(|r| r.start_col..r.end_col)
        .collect();
    assert!(changed_cols.contains(&7), "col 7 should be in the dirty region");
}

#[test]
fn backend_emits_changes() {
    let theme = Theme::dark();
    let mut backend = SimulatedBackend::new(80, 24);
    backend.enter_alternate_screen();

    let make_text = |id: u64, text: &str| -> WidgetNode {
        WidgetNode::Text(TextWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            text: text_signal(text),
            style: ReadSignal::constant(TextStyle::default()),
            wrap: WrapStrategy::None,
            truncate: TruncateStrategy::End,
        })
    };

    // Initial render
    let root = make_text(1, "hello");
    let c = measure_tree(&root, Size::new(80, 24));
    let l = layout_tree(Rect::new(0, 0, 80, 24), &root, &c);
    let screen = render_tree((80, 24), &root, &l, &theme);
    let mut regions = diff(&VirtualScreen::new(80, 24), &screen);
    merge_regions(&mut regions);
    backend.emit(&regions, &screen);

    // After emit, internal screen should have the content
    assert_eq!(backend.screen().cell_at(0, 0).ch, 'h');
}

// ── Signal-subscription E2E tests ────────────────────────────────

#[test]
fn signal_set_marks_subscriber_dirty_and_renders_new_value() {
    // 1. Create a reactive signal
    let text_signal = Signal::new("before".to_string());
    let read_signal = text_signal.read_only();

    // 2. Build widget tree with the signal
    let widget_id = WidgetId(1);
    let mut root = WidgetNode::Text(TextWidget {
        id: widget_id,
        props: LayoutProps::default(),
        text: read_signal,
        style: ReadSignal::constant(TextStyle::default()),
        wrap: WrapStrategy::None,
        truncate: TruncateStrategy::End,
    });

    // 3. Mount — triggers on_mount → subscribe
    mount_tree(&mut root);

    // 4. Verify subscription: set signal → widget marked dirty
    let mut dirty = DirtyTracker::new();
    text_signal.set("after".to_string(), &mut dirty);
    assert!(dirty.is_dirty(widget_id), "widget should be dirty after signal change");

    // 5. Render and verify new value appears
    let theme = Theme::dark();
    let constraints = measure_tree(&root, Size::new(80, 24));
    let layout = layout_tree(Rect::new(0, 0, 80, 24), &root, &constraints);
    let screen = render_tree((80, 24), &root, &layout, &theme);

    assert_eq!(screen.cell_at(0, 0).ch, 'a'); // 'after'
    assert_eq!(screen.cell_at(1, 0).ch, 'f');
}

#[test]
fn signal_same_value_does_not_mark_dirty() {
    let text_signal = Signal::new("unchanged".to_string());
    let read_signal = text_signal.read_only();
    let widget_id = WidgetId(1);

    let mut root = WidgetNode::Text(TextWidget {
        id: widget_id,
        props: LayoutProps::default(),
        text: read_signal,
        style: ReadSignal::constant(TextStyle::default()),
        wrap: WrapStrategy::None,
        truncate: TruncateStrategy::End,
    });

    mount_tree(&mut root);

    // Set to same value — should NOT mark dirty
    let mut dirty = DirtyTracker::new();
    text_signal.set("unchanged".to_string(), &mut dirty);
    assert!(!dirty.is_dirty(widget_id), "same value should not mark widget dirty");
}

#[test]
fn multiple_subscribers_all_marked_dirty() {
    let text_signal = Signal::new("shared".to_string());
    let r1 = text_signal.read_only();
    let r2 = text_signal.read_only();
    let id1 = WidgetId(10);
    let id2 = WidgetId(20);

    let mut root = WidgetNode::Box(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps::default(),
        children: vec![
            WidgetNode::Text(TextWidget {
                id: id1,
                props: LayoutProps::default(),
                text: r1,
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            WidgetNode::Text(TextWidget {
                id: id2,
                props: LayoutProps::default(),
                text: r2,
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    });

    mount_tree(&mut root);

    let mut dirty = DirtyTracker::new();
    text_signal.set("updated".to_string(), &mut dirty);
    assert!(dirty.is_dirty(id1), "subscriber 1 should be dirty");
    assert!(dirty.is_dirty(id2), "subscriber 2 should be dirty");
}
