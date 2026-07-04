// Integration test — full pipeline: measure → layout → render → diff → emit.

use arbor_tui_render::diff::{diff, merge_regions};
use arbor_tui_reactive::dirty::DirtyTracker;
use arbor_tui_widget::focus::mount_tree;
use arbor_tui_primitives::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
use arbor_tui_widget::render::render_tree;
use arbor_tui_reactive::signal::{ReadSignal, Signal};
use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{WidgetId, WidgetNode};
use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_widgets::box_widget::BoxWidget;
use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
use arbor_tui_backend::simulated_backend::SimulatedBackend;

#[test]
fn full_pipeline_box_with_two_texts() {
    let theme = Theme::dark();
    let (cols, rows) = (80u16, 24u16);

    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps { direction: Direction::Column, padding: RectOffset::all(1), ..Default::default() },
        children: vec![
            WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("Hello World".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps::default(), text: ReadSignal::constant("Second line".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        ],
    });

    let constraints = measure_tree(&root, Size::new(cols, rows));
    let layout = layout_tree(Rect::new(0, 0, cols, rows), &root, &constraints).unwrap();
    let screen = render_tree((cols, rows), &root, &layout, &theme);

    assert_eq!(screen.cols(), cols);
    assert_eq!(screen.rows(), rows);
    let t1_info = &layout[&WidgetId(1)];
    assert_eq!(screen.cell_at(t1_info.content_rect.x, t1_info.content_rect.y).ch, 'H');
}

#[test]
fn diff_detects_change_after_counter_increment() {
    let theme = Theme::dark();
    let (cols, rows) = (80u16, 24u16);

    fn make_text(id: u64, text: &str) -> WidgetNode {
        WidgetNode::new(TextWidget { id: WidgetId(id), props: LayoutProps::default(), text: ReadSignal::constant(text.into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End })
    }

    let root1 = make_text(1, "Count: 0");
    let c1 = measure_tree(&root1, Size::new(cols, rows));
    let l1 = layout_tree(Rect::new(0, 0, cols, rows), &root1, &c1).unwrap();
    let screen1 = render_tree((cols, rows), &root1, &l1, &theme);

    let root2 = make_text(1, "Count: 1");
    let c2 = measure_tree(&root2, Size::new(cols, rows));
    let l2 = layout_tree(Rect::new(0, 0, cols, rows), &root2, &c2).unwrap();
    let screen2 = render_tree((cols, rows), &root2, &l2, &theme);

    let mut regions = diff(&screen1, &screen2);
    merge_regions(&mut regions);
    assert!(!regions.is_empty());
}

#[test]
fn backend_emits_changes() {
    let theme = Theme::dark();
    let mut backend = SimulatedBackend::new(80, 24);
    backend.enter_alternate_screen().unwrap();

    let root = WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("hello".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });
    let c = measure_tree(&root, Size::new(80, 24));
    let l = layout_tree(Rect::new(0, 0, 80, 24), &root, &c).unwrap();
    let screen = render_tree((80, 24), &root, &l, &theme);
    let mut regions = diff(&VirtualScreen::new(80, 24), &screen);
    merge_regions(&mut regions);
    backend.emit(&regions, &screen).unwrap();
    assert_eq!(backend.screen().cell_at(0, 0).ch, 'h');
}

#[test]
fn signal_set_marks_subscriber_dirty_and_renders_new_value() {
    let text_signal = Signal::new("before".to_string());
    let read_signal = text_signal.read_only();
    let widget_id = WidgetId(1);
    let mut root = WidgetNode::new(TextWidget { id: widget_id, props: LayoutProps::default(), text: read_signal, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });

    mount_tree(&mut root);
    let mut dirty = DirtyTracker::new();
    text_signal.set("after".to_string(), &mut dirty);
    assert!(dirty.is_dirty(widget_id));

    let theme = Theme::dark();
    let constraints = measure_tree(&root, Size::new(80, 24));
    let layout = layout_tree(Rect::new(0, 0, 80, 24), &root, &constraints).unwrap();
    let screen = render_tree((80, 24), &root, &layout, &theme);

    assert_eq!(screen.cell_at(0, 0).ch, 'a');
    assert_eq!(screen.cell_at(1, 0).ch, 'f');
}

#[test]
fn signal_same_value_does_not_mark_dirty() {
    let text_signal = Signal::new("unchanged".to_string());
    let read_signal = text_signal.read_only();
    let widget_id = WidgetId(1);
    let mut root = WidgetNode::new(TextWidget { id: widget_id, props: LayoutProps::default(), text: read_signal, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });

    mount_tree(&mut root);
    let mut dirty = DirtyTracker::new();
    text_signal.set("unchanged".to_string(), &mut dirty);
    assert!(!dirty.is_dirty(widget_id));
}

#[test]
fn multiple_subscribers_all_marked_dirty() {
    let text_signal = Signal::new("shared".to_string());
    let r1 = text_signal.read_only();
    let r2 = text_signal.read_only();
    let id1 = WidgetId(10);
    let id2 = WidgetId(20);

    let mut root = WidgetNode::new(BoxWidget { id: WidgetId(0), props: LayoutProps::default(), children: vec![
        WidgetNode::new(TextWidget { id: id1, props: LayoutProps::default(), text: r1, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        WidgetNode::new(TextWidget { id: id2, props: LayoutProps::default(), text: r2, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
    ]});

    mount_tree(&mut root);
    let mut dirty = DirtyTracker::new();
    text_signal.set("updated".to_string(), &mut dirty);
    assert!(dirty.is_dirty(id1));
    assert!(dirty.is_dirty(id2));
}
