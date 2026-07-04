// Smoke test — verifies the counter rendering pipeline end-to-end.
#[cfg(test)]
mod counter_smoke {
    use arbor_tui_primitives::cell::Attrs;
    use arbor_tui_primitives::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
    use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
    use arbor_tui_primitives::widget_id::WidgetId;
    use arbor_tui_render::backend::TerminalBackend;
    use arbor_tui_render::diff::{diff, merge_regions};
    use arbor_tui_render::screen::VirtualScreen;
    use arbor_tui_render::theme::Theme;
    use arbor_tui_reactive::signal::ReadSignal;
    use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
    use arbor_tui_widget::render::render_tree;
    use arbor_tui_widget::widget::WidgetNode;
    use arbor_tui_widgets::box_widget::BoxWidget;
    use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
    use arbor_tui_backend::simulated_backend::SimulatedBackend;

    fn build_ui(theme: &Theme, count: i32, cols: u16, rows: u16) -> WidgetNode {
        let bar = "\u{2588}".repeat((count.rem_euclid(40) + 1) as usize);
        WidgetNode::new(BoxWidget {
            id: WidgetId(0), props: LayoutProps { direction: Direction::Column, padding: RectOffset { top: 1, bottom: 1, left: 2, right: 2 }, width: Some(cols), height: Some(rows), ..Default::default() },
            children: vec![
                WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant("Title".into()), style: ReadSignal::constant(TextStyle { fg: theme.accent(), bg: theme.surface(), attrs: Attrs { bold: true, ..Default::default() } }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps { padding: RectOffset { left: 2, bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant(format!("Count: {count}")), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(3), props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant(bar), style: ReadSignal::constant(TextStyle { fg: theme.primary(), bg: theme.surface(), attrs: Attrs::default() }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(BoxWidget { id: WidgetId(4), props: LayoutProps { flex: 1.0, ..Default::default() }, children: vec![] }),
                WidgetNode::new(TextWidget { id: WidgetId(5), props: LayoutProps::default(), text: ReadSignal::constant("j/k:+/- ^C/q:quit".into()), style: ReadSignal::constant(TextStyle { fg: theme.text_dim(), bg: theme.surface(), attrs: Attrs::default() }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            ],
        })
    }

    #[test]
    fn counter_value_renders_on_screen() {
        let th = Theme::dark(); let (c,r)=(80,24);
        let rt = build_ui(&th,0,c,r); let cs = measure_tree(&rt,Size::new(c,r));
        let lo = layout_tree(Rect::new(0,0,c,r),&rt,&cs).unwrap();
        let sc = render_tree((c,r),&rt,&lo,&th,None);
        let mut f=false; for row in 0..sc.rows(){for col in 0..sc.cols(){
            if sc.cell_at(col,row).ch=='C'{let n:String=(0..7).filter_map(|i|{let ch=sc.cell_at(col+i,row).ch;if ch!=' '{Some(ch)}else{None}}).collect();if n.contains("Count:"){f=true;}}
        }} assert!(f);
    }

    #[test]
    fn counter_ui_produces_dirty_regions_on_first_frame() {
        let th=Theme::dark();let(c,r)=(80,24);
        let rt=build_ui(&th,0,c,r);let cs=measure_tree(&rt,Size::new(c,r));
        let lo=layout_tree(Rect::new(0,0,c,r),&rt,&cs).unwrap();
        let sc=render_tree((c,r),&rt,&lo,&th,None);
        let mut rs=diff(&VirtualScreen::new(c,r),&sc);merge_regions(&mut rs);
        assert!(!rs.is_empty());
        let mut be=SimulatedBackend::new(c,r);be.emit(&rs,&sc).unwrap();
        // Title is at row 1 (after Box padding top=1), col 2 (after padding left=2)
        assert!(be.screen().cell_at(2,1).ch!=' ', "expected title text at (2,1)");
    }

    #[test]
    fn counter_increment_produces_delta_regions() {
        let th=Theme::dark();let(c,r)=(80,24);
        let r1=build_ui(&th,0,c,r);let m1=measure_tree(&r1,Size::new(c,r));let l1=layout_tree(Rect::new(0,0,c,r),&r1,&m1).unwrap();let s1=render_tree((c,r),&r1,&l1,&th,None);
        let r2=build_ui(&th,1,c,r);let m2=measure_tree(&r2,Size::new(c,r));let l2=layout_tree(Rect::new(0,0,c,r),&r2,&m2).unwrap();let s2=render_tree((c,r),&r2,&l2,&th,None);
        let mut rs=diff(&s1,&s2);merge_regions(&mut rs);assert!(!rs.is_empty());
    }
}
