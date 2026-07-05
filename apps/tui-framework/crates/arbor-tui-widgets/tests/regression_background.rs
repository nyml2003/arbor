// Regression test: background colors in light theme.
//
// Reproduces the bug where widgets forgot to fill their VirtualScreen,
// leaking Cell::default() (black bg) into a light-themed layout.
//
// Run: cargo test -p arbor-tui-widgets --test regression_background

use arbor_tui_domain::cell::{AnsiColor, PaletteColor, Span};
use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// The bug: in light theme, cells not explicitly written by a widget
/// retain Cell::default() background (palette 0 = black), creating dark
/// blotches inside stacks that use a lighter background.
#[test]
fn light_theme_no_black_bg_anywhere() {
    let wm = WidgetFactory::new();
    let t = Theme::light();

    // Build a realistic nested layout similar to layout_demo2
    let left = Border::new()
        .rounded()
        .fg(t.primary())
        .title(" Nav ")
        .child(
            RichText::new()
                .line(vec![Span::plain("  Home")])
                .line(vec![Span::plain("  Projects")])
                .line(vec![Span::plain("  Settings")])
                .build(&wm, &t),
        )
        .build(&wm, &t);

    let center = Border::new()
        .rounded()
        .fg(t.accent())
        .title(" Content ")
        .child(
            Text::new("Hello from the test harness!")
                .fg(t.text())
                .build(&wm, &t),
        )
        .build(&wm, &t);

    let body = Row::new()
        .children([left, Col::new().flex(1.0).children([center]).build(&wm, &t)])
        .build(&wm, &t);

    let root = Col::new().children([body]).build(&wm, &t);

    // Render with light theme
    let harness = WidgetHarness::render(&root, 80, 24, &t);

    // Sanity: text content is present
    let positions = harness.find_text("Hello from the test harness!");
    assert!(
        !positions.is_empty(),
        "text widget content should appear on screen"
    );

    // The critical assertion: no visible character should have black background
    if let Err(offenders) = harness.assert_no_black_bg_on_text() {
        panic!(
            "light theme leaked black background on {} cells. First 10: {:?}",
            offenders.len(),
            &offenders[..offenders.len().min(10)],
        );
    }
}

/// Border + Text nesting: the border interior should match the text background.
#[test]
fn border_interior_matches_text_bg_in_light_theme() {
    let wm = WidgetFactory::new();
    let t = Theme::light();

    let root = Border::new()
        .title(" Box ")
        .child(Text::new("inside").build(&wm, &t))
        .build(&wm, &t);

    let harness = WidgetHarness::render(&root, 40, 5, &t);

    // The interior cell immediately to the right of the left border (col=1, row=1)
    // should NOT be black. It should be theme.surface().
    let interior = harness.cell_at(1, 1);
    let black = PaletteColor(0);
    assert_ne!(
        interior.bg.palette, black,
        "Border interior cell should not be black in light theme. bg={:?}",
        interior.bg,
    );

    // The text itself should be visible
    let text_positions = harness.find_text("inside");
    assert!(!text_positions.is_empty(), "text 'inside' should be found");
}

/// Input widget in light theme — its bg should be surface_alt, not black.
#[test]
fn input_no_black_bg_in_light_theme() {
    use arbor_tui_widgets::input::Input;

    let wm = WidgetFactory::new();
    let t = Theme::light();

    let root = Input::new().placeholder("type here").build(&wm, &t);

    let harness = WidgetHarness::render(&root, 40, 1, &t);

    // The prompt "> " and placeholder are visible characters.
    if let Err(offenders) = harness.assert_no_black_bg_on_text() {
        panic!(
            "input in light theme leaked black background on {} cells. First 10: {:?}",
            offenders.len(),
            &offenders[..offenders.len().min(10)],
        );
    }
}

/// A comprehensive snapshot: measure bg coverage for every visible cell.
/// In light theme, the dominant background should be the theme's surface
/// color (palette 7 = white), not black (palette 0).
#[test]
fn light_theme_surface_is_dominant_bg() {
    let wm = WidgetFactory::new();
    let t = Theme::light();

    // Build a full layout
    let header = Border::new()
        .title(" Header ")
        .child(Text::new("subtitle").fg(t.text_dim()).build(&wm, &t))
        .build(&wm, &t);

    let body = Row::new()
        .children([
            Border::new()
                .title(" Left ")
                .child(Text::new("item 1\nitem 2").build(&wm, &t))
                .build(&wm, &t),
            Border::new()
                .flex(1.0)
                .title(" Center ")
                .child(
                    RichText::new()
                        .line(vec![Span::plain("content goes here")])
                        .build(&wm, &t),
                )
                .build(&wm, &t),
        ])
        .build(&wm, &t);

    let root = Col::new().children([header, body]).build(&wm, &t);

    let harness = WidgetHarness::render(&root, 80, 20, &t);

    // Count backgrounds
    let black = AnsiColor::from_palette(0);
    let white = AnsiColor::from_palette(7); // light theme surface

    let black_count = harness.count_bg(black);
    let white_count = harness.count_bg(white);

    // Black should be minimal — only in unused portions of the screen
    // (widgets don't always fill the full terminal width/height).
    assert!(
        black_count < white_count,
        "Expected dominant bg to be white (palette 7), but black (palette 0) has {black_count} cells vs white's {white_count}. Light theme is leaking black backgrounds."
    );
}
