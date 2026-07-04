// Smoke test — verifies the counter rendering pipeline end-to-end.

#[cfg(test)]
mod counter_smoke {
    use arbor_tui_render::backend::TerminalBackend;
    use arbor_tui_primitives::cell::Attrs;
    use arbor_tui_render::diff::{diff, merge_regions};
    use arbor_tui_primitives::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
    use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
    use arbor_tui_widget::render::render_tree;
    use arbor_tui_render::screen::VirtualScreen;
    use arbor_tui_reactive::signal::ReadSignal;
    use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
    use arbor_tui_render::theme::Theme;
    use arbor_tui_widget::widget::{WidgetId, WidgetNode};
    use arbor_tui_widgets::box_widget::BoxWidget;
    use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
    use arbor_tui_backend::simulated_backend::SimulatedBackend;

    fn build_ui(theme: &Theme, count: i32, cols: u16, rows: u16) -> WidgetNode {
        let bar_text = "█".repeat((count.rem_euclid(40) + 1) as usize);
        WidgetNode::new(BoxWidget {
            id: WidgetId(0),
            props: LayoutProps { direction: Direction::Column, padding: RectOffset { top: 1, bottom: 1, left: 2, right: 2 }, width: Some(cols), height: Some(rows), ..Default::default() },
            children: vec![
                WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant("Arbor TUI — Counter".into()), style: ReadSignal::constant(TextStyle { fg: theme.accent(), bg: theme.surface(), attrs: Attrs { bold: true, ..Default::default() } }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps { padding: RectOffset { left: 2, bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant(format!("Count: {count}")), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(3), props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() }, text: ReadSignal::constant(bar_text), style: ReadSignal::constant(TextStyle { fg: theme.primary(), bg: theme.surface(), attrs: Attrs::default() }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(BoxWidget { id: WidgetId(4), props: LayoutProps { flex: 1.0, ..Default::default() }, children: vec![] }),
                WidgetNode::new(TextWidget { id: WidgetId(5), props: LayoutProps::default(), text: ReadSignal::constant("j/k: +/-  |  ^C/q: quit".into()), style: ReadSignal::constant(TextStyle { fg: theme.text_dim(), bg: theme.surface(), attrs: Attrs::default() }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            ],
        })
    }

    #[test]
    fn counter_value_renders_on_screen() {
        let theme = Theme::dark();
        let (cols, rows) = (80u16, 24u16);
        let root = build_ui(&theme, 0, cols, rows);
        let constraints = measure_tree(&root, Size::new(cols, rows));
        let layout = layout_tree(Rect::new(0, 0, cols, rows), &root, &constraints).unwrap();
        let screen = render_tree((cols, rows), &root, &layout, &theme);

        let mut found = false;
        for row in 0..screen.rows() {
            for col in 0..screen.cols() {
                if screen.cell_at(col, row).ch == 'C' {
                    let next: String = (0..7).filter_map(|i| {
                        let c = screen.cell_at(col + i, row);
                        if c.ch != ' ' { Some(c.ch) } else { None }
                    }).collect();
                    if next.contains("Count:") { found = true; }
                }
            }
        }
        assert!(found, "Counter text 'Count: 0' should appear on screen");
    }

    #[test]
    fn counter_ui_produces_dirty_regions_on_first_frame() {
        let theme = Theme::dark();
        let (cols, rows) = (80u16, 24u16);
        let root = build_ui(&theme, 0, cols, rows);
        let constraints = measure_tree(&root, Size::new(cols, rows));
        let layout = layout_tree(Rect::new(0, 0, cols, rows), &root, &constraints).unwrap();
        let screen = render_tree((cols, rows), &root, &layout, &theme);

        let blank = VirtualScreen::new(cols, rows);
        let mut regions = diff(&blank, &screen);
        merge_regions(&mut regions);
        assert!(!regions.is_empty(), "First frame should produce dirty regions");

        let mut backend = SimulatedBackend::new(cols, rows);
        backend.emit(&regions, &screen).unwrap();

        let mut found = false;
        for row in 0..backend.screen().rows() {
            for col in 0..backend.screen().cols() {
                if backend.screen().cell_at(col, row).ch == 'C' { found = true; }
            }
        }
        assert!(found, "Counter should be emitted to backend");
    }

    #[test]
    fn counter_increment_produces_delta_regions() {
        let theme = Theme::dark();
        let (cols, rows) = (80u16, 24u16);

        let root1 = build_ui(&theme, 0, cols, rows);
        let c1 = measure_tree(&root1, Size::new(cols, rows));
        let l1 = layout_tree(Rect::new(0, 0, cols, rows), &root1, &c1).unwrap();
        let screen1 = render_tree((cols, rows), &root1, &l1, &theme);

        let root2 = build_ui(&theme, 1, cols, rows);
        let c2 = measure_tree(&root2, Size::new(cols, rows));
        let l2 = layout_tree(Rect::new(0, 0, cols, rows), &root2, &c2).unwrap();
        let screen2 = render_tree((cols, rows), &root2, &l2, &theme);

        let mut regions = diff(&screen1, &screen2);
        merge_regions(&mut regions);
        assert!(!regions.is_empty(), "Incrementing counter should produce dirty regions");
    }
}
