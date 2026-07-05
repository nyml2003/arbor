// Widget tests: Button + List + Tabs rendering.

use arbor_tui_render::theme::Theme;
use arbor_tui_widgets::button::{Button, ButtonStyle};
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::tabs::{TabDef, Tabs};
use arbor_tui_widgets::testing::WidgetHarness;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

// ── Button ────────────────────────────────────────────────────────

#[test]
fn button_renders_label() {
    let (wm, t) = wm_and_theme();
    let root = Button::new("Click Me").build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 1, &t);
    assert!(!h.find_text("Click Me").is_empty());
}

#[test]
fn button_primary_style() {
    let (wm, t) = wm_and_theme();
    let root = Button::new("OK").primary().build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 1, &t);
    let (col, row) = h.find_text("OK")[0];
    // Primary = theme.surface() text on theme.primary() bg
    assert_eq!(h.cell_at(col, row).bg.palette, t.primary().palette);
}

#[test]
fn button_danger_style() {
    let (wm, t) = wm_and_theme();
    let root = Button::new("Delete").danger().build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 1, &t);
    let (col, row) = h.find_text("Delete")[0];
    assert_eq!(h.cell_at(col, row).bg.palette, t.danger().palette);
}

#[test]
fn button_all_styles_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    for style in [
        ButtonStyle::Primary,
        ButtonStyle::Secondary,
        ButtonStyle::Danger,
        ButtonStyle::Default,
    ] {
        let root = Button::new("btn").style(style).build(&wm, &t);
        let h = WidgetHarness::render(&root, 20, 1, &t);
        h.assert_no_black_bg_on_text().unwrap();
    }
}

// ── List ──────────────────────────────────────────────────────────

#[test]
fn list_renders_items() {
    let (wm, t) = wm_and_theme();
    let root = List::new()
        .items(vec!["Alice".into(), "Bob".into(), "Charlie".into()])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    assert!(!h.find_text("Alice").is_empty());
    assert!(!h.find_text("Bob").is_empty());
    assert!(!h.find_text("Charlie").is_empty());
}

#[test]
fn list_items_are_spaced_vertically() {
    let (wm, t) = wm_and_theme();
    let root = List::new()
        .items(vec!["first".into(), "second".into()])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    let a = h.find_text("first")[0];
    let b = h.find_text("second")[0];
    assert!(b.1 > a.1, "second should be below first");
}

#[test]
fn list_empty() {
    let (wm, t) = wm_and_theme();
    let root = List::new().build(&wm, &t);
    let _h = WidgetHarness::render(&root, 40, 10, &t);
    // Should not panic; no items rendered
}

#[test]
fn list_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = List::new()
        .items(vec!["one".into(), "two".into()])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

// ── Tabs ──────────────────────────────────────────────────────────

#[test]
fn tabs_renders_headers() {
    let (wm, t) = wm_and_theme();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "General".into(),
                content: Text::new("general content").build(&wm, &t),
            },
            TabDef {
                label: "Advanced".into(),
                content: Text::new("advanced content").build(&wm, &t),
            },
        ])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 60, 10, &t);
    // Tab headers should be visible
    assert!(!h.find_text("General").is_empty());
    assert!(!h.find_text("Advanced").is_empty());
    // Active tab content should be shown
    assert!(!h.find_text("general content").is_empty());
}

#[test]
fn tabs_content_starts_below_header_and_separator() {
    let (wm, t) = wm_and_theme();
    let root = Tabs::new(0)
        .tabs(vec![TabDef {
            label: "General".into(),
            content: Text::new("body").build(&wm, &t),
        }])
        .build(&wm, &t);

    let h = WidgetHarness::render(&root, 30, 4, &t);
    let header = h.find_text("General")[0];
    let body = h.find_text("body")[0];

    assert_eq!(header.1, 0);
    assert_eq!(h.cell_at(0, 1).bg.palette, t.border().palette);
    assert_eq!(body.1, 2);
}

#[test]
fn tabs_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = Tabs::new(0)
        .tabs(vec![TabDef {
            label: "Tab1".into(),
            content: Text::new("content").fg(t.text()).build(&wm, &t),
        }])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 60, 10, &t);
    h.assert_no_black_bg_on_text().unwrap();
}
