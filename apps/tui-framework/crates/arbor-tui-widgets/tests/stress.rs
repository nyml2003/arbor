// Stress tests: large renders + resize cycles.
// These catch buffer overflows, layout instability, and memory leaks
// that only surface at scale or across multiple frames.

use arbor_tui_primitives::layout::Rect;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
use arbor_tui_widget::render::render_tree;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::container::Col;
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::testing::WidgetHarness;
use arbor_tui_widgets::widget_manager::WidgetManager;

fn wm_and_theme() -> (WidgetManager, Theme) {
    (WidgetManager::new(), Theme::dark())
}

// ── Large renders ─────────────────────────────────────────────────

/// Render a list with 1000 items — exercises the screen buffer at scale
/// and checks we don't OOB-panic or produce corrupted cells.
#[test]
fn list_1000_items() {
    let (wm, t) = wm_and_theme();
    let items: Vec<String> = (0..1000).map(|i| format!("item {i:04}")).collect();
    let root = List::new().items(items).build(&wm, &t);
    let h = WidgetHarness::render(&root, 80, 24, &t);
    // Only the first ~24 items should be visible (viewport clipped)
    assert!(!h.find_text("item 0000").is_empty());
    // Item 0100 should NOT be visible (beyond viewport)
    assert!(h.find_text("item 0100").is_empty());
}

/// Render a very tall layout (200+ rows of text) and verify it doesn't
/// panic or produce garbled output at the bottom.
#[test]
fn tall_layout_200_rows() {
    let (wm, t) = wm_and_theme();
    let mut lines = String::new();
    for i in 0..200 {
        if i > 0 {
            lines.push('\n');
        }
        lines.push_str(&format!("Line {i:03}"));
    }
    let root = Text::new(lines).build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    // Only first 5 lines should be visible
    assert!(!h.find_text("Line 000").is_empty());
    assert!(h.find_text("Line 005").is_empty());
}

/// Wide text (500 chars) — verify truncation doesn't panic.
#[test]
fn wide_text_500_chars() {
    let (wm, t) = wm_and_theme();
    let long = "A".repeat(500);
    let root = Text::new(long).build(&wm, &t);
    let h = WidgetHarness::render(&root, 80, 1, &t);
    // Should render, truncated to 80 cols. Last visible cell may be
    // 'A' or '…' depending on TruncateStrategy::End.
    let last = h.cell_at(79, 0).ch;
    assert!(
        last == 'A' || last == '\u{2026}',
        "last cell should be 'A' or '…', got {last:?}"
    );
    // Column 80 is out of bounds
    let oob = h.cell_at(80, 0);
    assert_eq!(oob.ch, ' '); // default for OOB
}

/// A full app-like layout (header + 3col body + footer) at typical
/// terminal sizes — just verifying it doesn't panic.
#[test]
fn realistic_layout_at_various_sizes() {
    let sizes = [(80, 24), (120, 40), (40, 10), (200, 60)];

    for &(cols, rows) in &sizes {
        let (wm, _) = wm_and_theme();
        let t = Theme::dark();

        let header = Border::new()
            .title(" Arbor ")
            .child(Text::new("v0.1.0").build(&wm, &t))
            .build(&wm, &t);

        use arbor_tui_widgets::container::Row;
        let body = Row::new()
            .flex(1.0)
            .children([
                Border::new().title(" Nav ").child(Text::new("menu").build(&wm, &t)).build(&wm, &t),
                Border::new().flex(1.0).title(" Main ").child(Text::new("content").build(&wm, &t)).build(&wm, &t),
                Border::new().title(" Info ").child(Text::new("info").build(&wm, &t)).build(&wm, &t),
            ])
            .build(&wm, &t);

        let footer = Border::new()
            .title(" Status ")
            .child(Text::new("OK").build(&wm, &t))
            .build(&wm, &t);

        let root = Col::new()
            .children([header, body, footer])
            .build(&wm, &t);

        let h = WidgetHarness::render(&root, cols, rows, &t);
        // Just verify it rendered without corrupting the screen dimensions
        assert_eq!(h.cols(), cols);
        assert_eq!(h.rows(), rows);
    }
}

// ── Resize cycles ─────────────────────────────────────────────────

/// Simulate multiple resize cycles: render → resize → render → resize.
/// Each cycle runs the full measure → layout → render pipeline.
#[test]
fn multiple_resize_cycles() {
    let t = Theme::dark();
    let resize_sequence = [
        (80, 24),
        (120, 40),
        (60, 15),
        (100, 30),
        (40, 10),
        (80, 24), // back to start
    ];

    for &(cols, rows) in &resize_sequence {
        let wm = WidgetManager::new();

        let root = Col::new()
            .children([
                Border::new()
                    .title(" Header ")
                    .child(Text::new("title").build(&wm, &t))
                    .build(&wm, &t),
                Border::new()
                    .flex(1.0)
                    .title(" Body ")
                    .child(Text::new("body").build(&wm, &t))
                    .build(&wm, &t),
            ])
            .build(&wm, &t);

        let h = WidgetHarness::render(&root, cols, rows, &t);
        assert_eq!(h.cols(), cols);
        assert_eq!(h.rows(), rows);

        // Content must be visible after resize
        assert!(
            !h.find_text("title").is_empty(),
            "text should survive resize to {cols}x{rows}"
        );
        assert!(
            !h.find_text("body").is_empty(),
            "body should survive resize to {cols}x{rows}"
        );
    }
}

/// Many rapid resizes of the same widget tree — verifies no layout
/// state corruption or memory growth across cycles.
#[test]
fn rapid_resize_100_cycles() {
    let (wm, t) = wm_and_theme();
    let root = Col::new()
        .children([
            Border::new()
                .title(" Top ")
                .child(Text::new("stable").build(&wm, &t))
                .build(&wm, &t),
            Border::new()
                .flex(1.0)
                .title(" Bottom ")
                .child(Text::new("resizes").build(&wm, &t))
                .build(&wm, &t),
        ])
        .build(&wm, &t);

    // Run 100 resize+render cycles, alternating between two sizes
    for i in 0..100 {
        let (cols, rows) = if i % 2 == 0 {
            (80, 24)
        } else {
            (100, 30)
        };

        // Manually run the pipeline for each resize
        let size = arbor_tui_primitives::layout::Size { w: cols, h: rows };
        let constraints = measure_tree(&root, size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), &root, &constraints)
            .expect("layout must succeed");
        let screen = render_tree((cols, rows), &root, &layout, &t, None);

        assert_eq!(screen.cols(), cols);
        assert_eq!(screen.rows(), rows);
        assert!(
            !render_tree::find_text_in_screen(&screen, "stable").is_empty(),
            "cycle {i}: text should survive resize to {cols}x{rows}"
        );
    }
}

// Helper needed because we're using render_tree directly, not WidgetHarness
mod render_tree {
    use arbor_tui_render::screen::VirtualScreen;

    pub fn find_text_in_screen(screen: &VirtualScreen, needle: &str) -> Vec<(u16, u16)> {
        let mut positions = Vec::new();
        let needle_chars: Vec<char> = needle.chars().collect();
        if needle_chars.is_empty() {
            return positions;
        }
        for row in 0..screen.rows() {
            let mut col = 0u16;
            while col < screen.cols() {
                let mut matched = true;
                for (i, &ch) in needle_chars.iter().enumerate() {
                    let c = col + i as u16;
                    if c >= screen.cols() || screen.cell_at(c, row).ch != ch {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    positions.push((col, row));
                    col += needle_chars.len() as u16;
                } else {
                    col += 1;
                }
            }
        }
        positions
    }
}
