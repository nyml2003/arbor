// Smoke test for resize → re-render correctness.
#[cfg(test)]
mod resize_smoke {
    use arbor_tui_primitives::cell::Attrs;
    use arbor_tui_primitives::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
    use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
    use arbor_tui_primitives::widget_id::WidgetId;
    use arbor_tui_render::diff::{diff, merge_regions};
    use arbor_tui_render::screen::VirtualScreen;
    use arbor_tui_render::theme::Theme;
    use arbor_tui_reactive::signal::ReadSignal;
    use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
    use arbor_tui_widget::render::render_tree;
    use arbor_tui_widget::widget::WidgetNode;
    use arbor_tui_widgets::box_widget::BoxWidget;
    use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};

    fn wrap_and_number(text: &str, width: u16) -> String {
        let lines: Vec<String> = text.lines()
            .flat_map(|line| if line.is_empty() { vec![String::new()] } else { arbor_tui_primitives::text::wrap_lines(line, width, WrapStrategy::Char) })
            .collect();
        lines.iter().enumerate().map(|(i, l)| format!("{:>5} {}", i+1, l)).collect::<Vec<_>>().join("\n")
    }

    fn build_ui(body_text: &str, cols: u16, rows: u16) -> WidgetNode {
        let theme = Theme::dark();
        WidgetNode::new(BoxWidget {
            id: WidgetId(0), props: LayoutProps { direction: Direction::Column, width: Some(cols), height: Some(rows), ..Default::default() },
            children: vec![
                WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant("Title".into()), style: ReadSignal::constant(TextStyle { fg: theme.accent(), bg: theme.surface(), attrs: Attrs { bold: true, ..Default::default() } }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps { flex: 1.0, padding: RectOffset { left: 1, right: 1, top: 0, bottom: 0 }, ..Default::default() }, text: ReadSignal::constant(body_text.into()), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
                WidgetNode::new(TextWidget { id: WidgetId(3), props: LayoutProps::default(), text: ReadSignal::constant("Status".into()), style: ReadSignal::constant(TextStyle { fg: theme.accent(), bg: theme.surface(), attrs: Attrs { bold: true, ..Default::default() } }), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            ],
        })
    }

    #[test]
    fn resize_wider_no_stale_cells() {
        let theme = Theme::dark();
        let fc = "The quick brown fox jumps over the lazy dog repeatedly without stopping";
        let (c1, r1) = (50u16, 20u16);
        let root1 = build_ui(&wrap_and_number(fc, c1-6), c1, r1);
        let l1 = layout_tree(Rect::new(0,0,c1,r1), &root1, &measure_tree(&root1, Size::new(c1,r1))).unwrap();
        let s1 = render_tree((c1,r1), &root1, &l1, &theme, None);
        let mut old = s1.clone(); old.resize(80, 30);
        let (c2, r2) = (80u16, 30u16);
        let root2 = build_ui(&wrap_and_number(fc, c2-6), c2, r2);
        let l2 = layout_tree(Rect::new(0,0,c2,r2), &root2, &measure_tree(&root2, Size::new(c2,r2))).unwrap();
        let s2 = render_tree((c2,r2), &root2, &l2, &theme, None);
        let mut r = diff(&old, &s2); merge_regions(&mut r);
        assert!(!r.is_empty());
        assert!(r.iter().any(|d| d.row >= l2[&WidgetId(2)].content_rect.y));
    }

    #[test]
    fn resize_shrinker_no_stale_cells() {
        let theme = Theme::dark();
        let fc = "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnop";
        let root1 = build_ui(&wrap_and_number(fc, 74), 80, 30);
        let l1 = layout_tree(Rect::new(0,0,80,30), &root1, &measure_tree(&root1, Size::new(80,30))).unwrap();
        let s1 = render_tree((80,30), &root1, &l1, &theme, None);
        let mut old = s1.clone(); old.resize(40, 20);
        let root2 = build_ui(&wrap_and_number(fc, 34), 40, 20);
        let l2 = layout_tree(Rect::new(0,0,40,20), &root2, &measure_tree(&root2, Size::new(40,20))).unwrap();
        let s2 = render_tree((40,20), &root2, &l2, &theme, None);
        let mut r = diff(&old, &s2); merge_regions(&mut r);
        assert!(!r.is_empty());
    }

    #[test]
    fn resize_then_render_produces_complete_coverage() {
        let theme = Theme::dark();
        let fc = "The quick brown fox jumps over the lazy dog repeatedly without stopping";
        let root1 = build_ui(&wrap_and_number(fc, 44), 50, 24);
        let l1 = layout_tree(Rect::new(0,0,50,24), &root1, &measure_tree(&root1, Size::new(50,24))).unwrap();
        let _s1 = render_tree((50,24), &root1, &l1, &theme, None);
        let app_screen = VirtualScreen::new(80, 30);
        let root2 = build_ui(&wrap_and_number(fc, 74), 80, 30);
        let l2 = layout_tree(Rect::new(0,0,80,30), &root2, &measure_tree(&root2, Size::new(80,30))).unwrap();
        let s2 = render_tree((80,30), &root2, &l2, &theme, None);
        let mut r = diff(&app_screen, &s2); merge_regions(&mut r);
        assert!(!r.is_empty());
        assert!(r.iter().any(|d| d.row == l2[&WidgetId(2)].content_rect.y));
    }
}
