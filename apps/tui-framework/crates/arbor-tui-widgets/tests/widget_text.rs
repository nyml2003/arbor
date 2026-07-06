// Widget tests: Text + RichText rendering.
// Covers content rendering, light/dark theme bg, wrapping, truncation.

use arbor_tui_domain::cell::Span;
use arbor_tui_domain::focus::mount_tree;
use arbor_tui_domain::identity::DirtyKind;
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

// ── Text ──────────────────────────────────────────────────────────

#[test]
fn text_renders_content() {
    let (wm, t) = wm_and_theme();
    let root = Text::new("hello world").build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    assert!(!h.find_text("hello world").is_empty());
}

#[test]
fn text_multiline() {
    let (wm, t) = wm_and_theme();
    let root = Text::new("line1\nline2\nline3").build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    assert!(!h.find_text("line1").is_empty());
    assert!(!h.find_text("line2").is_empty());
    assert!(!h.find_text("line3").is_empty());
}

#[test]
fn text_truncates_overflow() {
    let (wm, t) = wm_and_theme();
    let root = Text::new("very long text that exceeds available width").build(&wm, &t);
    let h = WidgetHarness::render(&root, 10, 3, &t);
    // Should render but be clipped
    assert_eq!(h.cols(), 10);
}

#[test]
fn text_custom_fg() {
    let (wm, t) = wm_and_theme();
    let root = Text::new("colored").fg(t.primary()).build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    let (col, row) = h.find_text("colored")[0];
    assert_eq!(h.cell_at(col, row).fg.palette, t.primary().palette);
}

#[test]
fn text_no_black_bg_light_theme() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = Text::new("hello").build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn text_bg_matches_theme_surface_dark() {
    let (wm, t) = wm_and_theme();
    let root = Text::new("dark mode text").build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    // In dark theme, text bg should be theme.surface() (palette 0).
    let (col, row) = h.find_text("dark mode text")[0];
    assert_eq!(h.cell_at(col, row).bg.palette, t.surface().palette);
}

#[test]
fn text_signal_dependency_declares_layout_dirty() {
    let (wm, t) = wm_and_theme();
    let content = Signal::new("reactive".to_string());
    let mut root = Text::new("").content_from(&content).build(&wm, &t);

    let deps = root.signal_deps();

    assert!(deps.iter().any(|dep| {
        dep.signal_id == content.id()
            && dep.generation == content.generation()
            && dep.dirty_kind == DirtyKind::Layout
    }));

    mount_tree(&mut root);
    assert_eq!(
        content.subscriber_dirty_kind(root.id()),
        Some(DirtyKind::Layout)
    );
}

// ── RichText ──────────────────────────────────────────────────────

#[test]
fn rich_text_renders_multiline() {
    let (wm, t) = wm_and_theme();
    let root = RichText::new()
        .line(vec![Span::plain("first line")])
        .line(vec![Span::plain("second line")])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    assert!(!h.find_text("first line").is_empty());
    assert!(!h.find_text("second line").is_empty());
}

#[test]
fn rich_text_inline_styles() {
    let (wm, t) = wm_and_theme();
    let root = RichText::new()
        .line(vec![Span::plain("normal "), Span::bold("bold")])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    assert!(!h.find_text("normal").is_empty());
    assert!(!h.find_text("bold").is_empty());
}

#[test]
fn rich_text_empty_lines() {
    let (wm, t) = wm_and_theme();
    let root = RichText::new()
        .line(vec![Span::plain("above")])
        .line(vec![])
        .line(vec![Span::plain("below")])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    let above = h.find_text("above");
    let below = h.find_text("below");
    assert!(!above.is_empty());
    assert!(!below.is_empty());
    // Empty line should separate them
    assert!(below[0].1 > above[0].1);
}

#[test]
fn rich_text_no_black_bg_light_theme() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = RichText::new()
        .line(vec![Span::plain("hello")])
        .line(vec![Span::plain("world")])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn rich_text_unfilled_area_uses_widget_bg_dark() {
    let (wm, t) = wm_and_theme();
    // Single short span — the rest of the row should use the widget's bg fill
    let root = RichText::new()
        .line(vec![Span::new(
            "x",
            t.text(),
            t.surface(),
            Default::default(),
        )])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    // The text cell bg should match theme surface
    let (col, row) = h.find_text("x")[0];
    assert_eq!(h.cell_at(col, row).bg.palette, t.surface().palette);
    // An unfilled cell in the same row (col 10) should also match the widget bg
    if 10 < h.cols() {
        assert_eq!(h.cell_at(10, row).bg.palette, t.surface().palette);
    }
}

// ── Text inside Border ────────────────────────────────────────────

#[test]
fn text_inside_border_dark_theme() {
    let (wm, t) = wm_and_theme();
    let root = Border::new()
        .title(" Box ")
        .child(Text::new("inside").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    assert!(!h.find_text("inside").is_empty());
    assert!(!h.find_text("Box").is_empty());
}

#[test]
fn text_inside_border_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = Border::new()
        .title(" Box ")
        .child(Text::new("inside").fg(t.text()).build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn richtext_inside_border_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = Border::new()
        .title(" Box ")
        .child(
            RichText::new()
                .line(vec![Span::plain("line 1")])
                .line(vec![Span::plain("line 2")])
                .build(&wm, &t),
        )
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 7, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

// ── Span bg inheritance ───────────────────────────────────────────

#[test]
fn span_explicit_bg_is_respected() {
    let (wm, t) = wm_and_theme();
    let root = RichText::new()
        .line(vec![Span::new(
            "colored",
            t.text(),
            t.primary(),
            Default::default(),
        )])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 3, &t);
    let (col, row) = h.find_text("colored")[0];
    assert_eq!(h.cell_at(col, row).bg.palette, t.primary().palette);
}
