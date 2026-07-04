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
    let th = Theme::dark(); let (c,r) = (80u16,24u16);
    let root = WidgetNode::new(BoxWidget {
        id: WidgetId(0), props: LayoutProps { direction: Direction::Column, padding: RectOffset::all(1), ..Default::default() },
        children: vec![
            WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("Hello World".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps::default(), text: ReadSignal::constant("Second line".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        ],
    });
    let cs = measure_tree(&root, Size::new(c,r));
    let lo = layout_tree(Rect::new(0,0,c,r), &root, &cs).unwrap();
    let sc = render_tree((c,r), &root, &lo, &th, None);
    assert_eq!(sc.cols(), c); assert_eq!(sc.rows(), r);
    assert_eq!(sc.cell_at(lo[&WidgetId(1)].content_rect.x, lo[&WidgetId(1)].content_rect.y).ch, 'H');
}

#[test]
fn diff_detects_change_after_counter_increment() {
    let th = Theme::dark(); let (c,r) = (80u16,24u16);
    fn mt(id: u64, t: &str) -> WidgetNode { WidgetNode::new(TextWidget { id: WidgetId(id), props: LayoutProps::default(), text: ReadSignal::constant(t.into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }) }
    let r1=mt(1,"Count: 0"); let c1=measure_tree(&r1,Size::new(c,r)); let l1=layout_tree(Rect::new(0,0,c,r),&r1,&c1).unwrap(); let s1=render_tree((c,r),&r1,&l1,&th,None);
    let r2=mt(1,"Count: 1"); let c2=measure_tree(&r2,Size::new(c,r)); let l2=layout_tree(Rect::new(0,0,c,r),&r2,&c2).unwrap(); let s2=render_tree((c,r),&r2,&l2,&th,None);
    let mut rs=diff(&s1,&s2); merge_regions(&mut rs); assert!(!rs.is_empty());
}

#[test]
fn backend_emits_changes() {
    let th = Theme::dark(); let mut be = SimulatedBackend::new(80,24); be.enter_alternate_screen().unwrap();
    let rt = WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("hello".into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });
    let cs = measure_tree(&rt, Size::new(80,24)); let lo = layout_tree(Rect::new(0,0,80,24),&rt,&cs).unwrap();
    let sc = render_tree((80,24),&rt,&lo,&th,None);
    let mut rs = diff(&VirtualScreen::new(80,24),&sc); merge_regions(&mut rs);
    be.emit(&rs,&sc).unwrap(); assert_eq!(be.screen().cell_at(0,0).ch,'h');
}

#[test]
fn signal_set_marks_subscriber_dirty_and_renders_new_value() {
    let sig = Signal::new("before".to_string()); let rs = sig.read_only(); let id = WidgetId(1);
    let mut rt = WidgetNode::new(TextWidget { id, props: LayoutProps::default(), text: rs, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });
    mount_tree(&mut rt); let mut dt = DirtyTracker::new();
    sig.set("after".to_string(), &mut dt); assert!(dt.is_dirty(id));
    let th = Theme::dark(); let cs = measure_tree(&rt, Size::new(80,24)); let lo = layout_tree(Rect::new(0,0,80,24),&rt,&cs).unwrap();
    let sc = render_tree((80,24),&rt,&lo,&th,None);
    assert_eq!(sc.cell_at(0,0).ch,'a'); assert_eq!(sc.cell_at(1,0).ch,'f');
}

#[test]
fn signal_same_value_does_not_mark_dirty() {
    let sig = Signal::new("unchanged".to_string()); let rs = sig.read_only(); let id = WidgetId(1);
    let mut rt = WidgetNode::new(TextWidget { id, props: LayoutProps::default(), text: rs, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End });
    mount_tree(&mut rt); let mut dt = DirtyTracker::new();
    sig.set("unchanged".to_string(), &mut dt); assert!(!dt.is_dirty(id));
}

#[test]
fn multiple_subscribers_all_marked_dirty() {
    let sig = Signal::new("shared".to_string()); let r1=sig.read_only(); let r2=sig.read_only();
    let id1=WidgetId(10); let id2=WidgetId(20);
    let mut rt = WidgetNode::new(BoxWidget { id: WidgetId(0), props: LayoutProps::default(), children: vec![
        WidgetNode::new(TextWidget { id: id1, props: LayoutProps::default(), text: r1, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        WidgetNode::new(TextWidget { id: id2, props: LayoutProps::default(), text: r2, style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
    ]});
    mount_tree(&mut rt); let mut dt = DirtyTracker::new();
    sig.set("updated".to_string(), &mut dt); assert!(dt.is_dirty(id1)); assert!(dt.is_dirty(id2));
}
