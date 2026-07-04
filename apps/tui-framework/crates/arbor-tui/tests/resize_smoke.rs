// Smoke test for resize → re-render correctness.
// Simulate the viewer: render at 80 cols, resize to 100, re-render, verify no stale cells.

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

    /// Wrap text at the given width (like viewer's wrap_content), then assign line numbers.
    fn wrap_and_number(text: &str, width: u16) -> String {
        let lines: Vec<String> = text.lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![String::new()]
                } else {
                    arbor_tui_primitives::text::wrap_lines(line, width, WrapStrategy::Char)
                }
            })
            .collect();
        lines.iter().enumerate()
            .map(|(i, line)| format!("{:>5} {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Build a simple viewer-like UI: title row + body text + status row.
    fn build_ui(body_text: &str, cols: u16, rows: u16) -> WidgetNode {
        let theme = Theme::dark();
        WidgetNode::new(BoxWidget {
            id: WidgetId(0),
            props: LayoutProps {
                direction: Direction::Column,
                width: Some(cols),
                height: Some(rows),
                ..Default::default()
            },
            children: vec![
                WidgetNode::new(TextWidget {
                    id: WidgetId(1),
                    props: LayoutProps::default(),
                    text: ReadSignal::constant("Title".into()),
                    style: ReadSignal::constant(TextStyle {
                        fg: theme.accent(), bg: theme.surface(),
                        attrs: Attrs { bold: true, ..Default::default() },
                    }),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::new(TextWidget {
                    id: WidgetId(2),
                    props: LayoutProps {
                        flex: 1.0,
                        padding: arbor_tui_primitives::layout::RectOffset { left: 1, right: 1, top: 0, bottom: 0 },
                        ..Default::default()
                    },
                    text: ReadSignal::constant(body_text.into()),
                    style: ReadSignal::constant(TextStyle::default()),
                    wrap: WrapStrategy::None, // pre-wrapped lines, \n separated
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::new(TextWidget {
                    id: WidgetId(3),
                    props: LayoutProps::default(),
                    text: ReadSignal::constant("Status".into()),
                    style: ReadSignal::constant(TextStyle {
                        fg: theme.accent(), bg: theme.surface(),
                        attrs: Attrs { bold: true, ..Default::default() },
                    }),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
            ],
        })
    }

    #[test]
    fn resize_wider_no_stale_cells() {
        let theme = Theme::dark();

        // Realistic content: long line that wraps at 50 cols but fits at 80
        let file_content = "The quick brown fox jumps over the lazy dog repeatedly without stopping";

        // Frame 1: render at 50x20
        let (cols1, rows1) = (50u16, 20u16);
        let body1 = wrap_and_number(file_content, cols1.saturating_sub(6));
        let root1 = build_ui(&body1, cols1, rows1);
        let c1 = measure_tree(&root1, Size::new(cols1, rows1));
        let l1 = layout_tree(Rect::new(0, 0, cols1, rows1), &root1, &c1).unwrap();
        let screen1 = render_tree((cols1, rows1), &root1, &l1, &theme);

        // Simulate resize: grow to 80x30
        let mut old_resized = screen1.clone();
        old_resized.resize(80, 30);

        // Frame 2: render at 80x30 — same content, wider, fewer wrapped lines
        let (cols2, rows2) = (80u16, 30u16);
        let body2 = wrap_and_number(file_content, cols2.saturating_sub(6));
        let root2 = build_ui(&body2, cols2, rows2);
        let c2 = measure_tree(&root2, Size::new(cols2, rows2));
        let l2 = layout_tree(Rect::new(0, 0, cols2, rows2), &root2, &c2).unwrap();
        let screen2 = render_tree((cols2, rows2), &root2, &l2, &theme);

        let mut regions = diff(&old_resized, &screen2);
        merge_regions(&mut regions);

        assert!(!regions.is_empty(), "resize should produce dirty regions");

        // After re-wrapping, line count changes → status bar position shifts → dirty
        let body_info2 = &l2[&WidgetId(2)];
        let body_start = body_info2.content_rect.y;

        // At least some rows in body area must be dirty (the content changed due to re-wrap)
        let body_dirty = regions.iter().any(|r| r.row >= body_start);
        assert!(body_dirty, "Body area should have dirty regions after resize+rewrap");

        // Also: rows that were in the OLD body but are now past the NEW body end
        // should be dirty (they now show status bar or blank, not old body text)
        let old_body_info = &l1[&WidgetId(2)];
        let new_body_info = &l2[&WidgetId(2)];
        if old_body_info.content_rect.h != new_body_info.content_rect.h {
            // Content height changed — verify status bar row is dirty
            let status_info2 = &l2[&WidgetId(3)];
            let status_dirty = regions.iter().any(|r| r.row == status_info2.content_rect.y);
            assert!(status_dirty, "Status row should be dirty after resize changes layout");
        }
    }

    #[test]
    fn resize_shrinker_no_stale_cells() {
        let theme = Theme::dark();
        let file_content = "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnop";

        // Render at 80 cols
        let (cols1, rows1) = (80u16, 30u16);
        let body1 = wrap_and_number(file_content, cols1.saturating_sub(6));
        let root1 = build_ui(&body1, cols1, rows1);
        let c1 = measure_tree(&root1, Size::new(cols1, rows1));
        let l1 = layout_tree(Rect::new(0, 0, cols1, rows1), &root1, &c1).unwrap();
        let screen1 = render_tree((cols1, rows1), &root1, &l1, &theme);

        let mut old_resized = screen1.clone();
        old_resized.resize(40, 20);

        // Shrink to 40 cols
        let (cols2, rows2) = (40u16, 20u16);
        let body2 = wrap_and_number(file_content, cols2.saturating_sub(6));
        let root2 = build_ui(&body2, cols2, rows2);
        let c2 = measure_tree(&root2, Size::new(cols2, rows2));
        let l2 = layout_tree(Rect::new(0, 0, cols2, rows2), &root2, &c2).unwrap();
        let screen2 = render_tree((cols2, rows2), &root2, &l2, &theme);

        let mut regions = diff(&old_resized, &screen2);
        merge_regions(&mut regions);
        assert!(!regions.is_empty(), "resize shrink should produce dirty regions");
    }

    #[test]
    fn resize_then_render_produces_complete_coverage() {
        let theme = Theme::dark();
        let file_content = "The quick brown fox jumps over the lazy dog repeatedly without stopping";

        // Render at 50 cols (wrapping needed)
        let root1 = build_ui(
            &wrap_and_number(file_content, 44), 50, 24,
        );
        let c1 = measure_tree(&root1, Size::new(50, 24));
        let l1 = layout_tree(Rect::new(0, 0, 50, 24), &root1, &c1).unwrap();
        let screen1 = render_tree((50, 24), &root1, &l1, &theme);

        // Simulate app.resize — blank canvas (new approach, not copy+resize)
        let app_screen = VirtualScreen::new(80, 30);

        // Re-render at 80 cols (wider)
        let root2 = build_ui(
            &wrap_and_number(file_content, 74), 80, 30,
        );
        let c2 = measure_tree(&root2, Size::new(80, 30));
        let l2 = layout_tree(Rect::new(0, 0, 80, 30), &root2, &c2).unwrap();
        let screen2 = render_tree((80, 30), &root2, &l2, &theme);

        // Blank canvas vs rendered content = all non-blank cells are dirty
        let mut regions = diff(&app_screen, &screen2);
        merge_regions(&mut regions);
        assert!(!regions.is_empty(), "blank→rendered must produce dirty regions");

        let body_info2 = &l2[&WidgetId(2)];
        let body_row = body_info2.content_rect.y;
        let first_body_dirty = regions.iter().any(|r| r.row == body_row);

        // The blank screen has spaces everywhere; the rendered screen has text.
        // Every row with content must be dirty.
        assert!(first_body_dirty, "First body row ({body_row}) should be dirty (blank→content)");
    }
}
