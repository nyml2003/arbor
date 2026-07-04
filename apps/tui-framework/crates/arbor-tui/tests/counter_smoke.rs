// Quick sanity check — runs the counter rendering pipeline through SimulatedBackend
// to verify the counter text actually appears on screen.

#[cfg(test)]
mod counter_smoke {
    use arbor_tui_core::backend::TerminalBackend;
    use arbor_tui_core::cell::Attrs;
    use arbor_tui_core::layout::{Direction, LayoutProps, Rect, RectOffset, Size};
    use arbor_tui_core::layout_engine::{layout_tree, measure_tree};
    use arbor_tui_core::render::render_tree;
    use arbor_tui_core::screen::VirtualScreen;
    use arbor_tui_core::signal::ReadSignal;
    use arbor_tui_core::text::{TruncateStrategy, WrapStrategy};
    use arbor_tui_core::theme::Theme;
    use arbor_tui_core::widget::{
        BoxWidget, TextWidget, TextStyle, WidgetId, WidgetNode,
    };
    use arbor_tui_core::diff::{diff, merge_regions};
    use arbor_tui_backend::simulated_backend::SimulatedBackend;

    fn build_ui(theme: &Theme, count: i32, cols: u16, rows: u16) -> WidgetNode {
        let bar_w = ((count % 40 + 40) % 40) as u16 + 1;
        let bar_text = "█".repeat(bar_w as usize);

        WidgetNode::Box(BoxWidget {
            id: WidgetId(0),
            props: LayoutProps {
                direction: Direction::Column,
                padding: RectOffset { top: 1, bottom: 1, left: 2, right: 2 },
                width: Some(cols),
                height: Some(rows),
                ..Default::default()
            },
            children: vec![
                WidgetNode::Text(TextWidget {
                    id: WidgetId(1),
                    props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() },
                    text: ReadSignal::constant("Arbor TUI — Counter".to_string()),
                    style: ReadSignal::constant(TextStyle {
                        fg: theme.accent(), bg: theme.surface(),
                        attrs: Attrs { bold: true, ..Default::default() },
                    }),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::Text(TextWidget {
                    id: WidgetId(2),
                    props: LayoutProps { padding: RectOffset { left: 2, bottom: 1, ..Default::default() }, ..Default::default() },
                    text: ReadSignal::constant(format!("Count: {}", count)),
                    style: ReadSignal::constant(TextStyle::default()),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::Text(TextWidget {
                    id: WidgetId(3),
                    props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() },
                    text: ReadSignal::constant(bar_text),
                    style: ReadSignal::constant(TextStyle {
                        fg: theme.primary(), bg: theme.surface(),
                        attrs: Attrs::default(),
                    }),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
                WidgetNode::Box(BoxWidget {
                    id: WidgetId(4),
                    props: LayoutProps { flex: 1.0, ..Default::default() },
                    children: vec![],
                }),
                WidgetNode::Text(TextWidget {
                    id: WidgetId(5),
                    props: LayoutProps::default(),
                    text: ReadSignal::constant("j/k: +/-  |  ^C/q: quit".to_string()),
                    style: ReadSignal::constant(TextStyle { fg: theme.text_dim(), bg: theme.surface(), attrs: Attrs::default() }),
                    wrap: WrapStrategy::None,
                    truncate: TruncateStrategy::End,
                }),
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

        // Find where "Count: 0" appears on screen
        let count_info = &layout.widgets[&WidgetId(2)];
        eprintln!("Count widget content_rect: {:?}", count_info.content_rect);

        let mut found = false;
        for row in 0..screen.rows() {
            for col in 0..screen.cols() {
                let cell = screen.cell_at(col, row);
                if cell.ch == 'C' {
                    // Check if this is the start of "Count:"
                    let next: String = (0..7).filter_map(|i| {
                        let c = screen.cell_at(col + i, row);
                        if c.ch != ' ' { Some(c.ch) } else { None }
                    }).collect();
                    eprintln!("Found 'C' at ({},{}): followed by '{}'", col, row, next);
                    if next.contains("Count:") {
                        found = true;
                    }
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

        eprintln!("Dirty regions count: {}", regions.len());
        for r in &regions {
            eprintln!("  row={} cols=[{}, {})", r.row, r.start_col, r.end_col);
        }

        assert!(!regions.is_empty(), "First frame should produce dirty regions");

        // Simulated backend: emit should write to internal screen
        let mut backend = SimulatedBackend::new(cols, rows);
        backend.emit(&regions, &screen).unwrap();

        // After emit, internal screen should have the content
        let mut found_count = false;
        let mut found_title = false;
        for row in 0..backend.screen().rows() {
            for col in 0..backend.screen().cols() {
                let ch = backend.screen().cell_at(col, row).ch;
                if ch == 'A' { found_title = true; }
                if ch == 'C' { found_count = true; }
            }
        }
        assert!(found_title, "Title should be emitted to backend");
        assert!(found_count, "Counter should be emitted to backend");
    }

    #[test]
    fn counter_increment_produces_delta_regions() {
        let theme = Theme::dark();
        let (cols, rows) = (80u16, 24u16);

        // Frame 1: count=0
        let root1 = build_ui(&theme, 0, cols, rows);
        let c1 = measure_tree(&root1, Size::new(cols, rows));
        let l1 = layout_tree(Rect::new(0, 0, cols, rows), &root1, &c1).unwrap();
        let screen1 = render_tree((cols, rows), &root1, &l1, &theme);

        // Frame 2: count=1
        let root2 = build_ui(&theme, 1, cols, rows);
        let c2 = measure_tree(&root2, Size::new(cols, rows));
        let l2 = layout_tree(Rect::new(0, 0, cols, rows), &root2, &c2).unwrap();
        let screen2 = render_tree((cols, rows), &root2, &l2, &theme);

        let mut regions = diff(&screen1, &screen2);
        merge_regions(&mut regions);

        eprintln!("Delta regions after increment: {}", regions.len());
        for r in &regions {
            eprintln!("  row={} cols=[{}, {})", r.row, r.start_col, r.end_col);
        }

        assert!(!regions.is_empty(), "Incrementing counter should produce dirty regions");

        // Verify '0'→'1' is in a dirty region
        let count_info = &l2.widgets[&WidgetId(2)];
        let count_row = count_info.content_rect.y;
        // "Count: 0" — '0' is at offset 7 from the start
        let digit_col = count_info.content_rect.x + 7;

        let digit_in_region = regions.iter().any(|r| {
            r.row == count_row && r.start_col <= digit_col && digit_col < r.end_col
        });
        assert!(digit_in_region, "The changed digit '0'→'1' should be in a dirty region at row={} col={}", count_row, digit_col);
    }
}
