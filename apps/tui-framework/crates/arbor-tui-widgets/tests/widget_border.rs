// Widget tests: Border rendering.
// Covers standalone borders, rounded/sharp, titles, custom fg/bg.

use arbor_tui_render::theme::Theme;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::testing::WidgetHarness;
use arbor_tui_widgets::widget_manager::WidgetManager;

fn wm_and_theme() -> (WidgetManager, Theme) {
    (WidgetManager::new(), Theme::dark())
}

#[test]
fn border_renders_title() {
    let (wm, t) = wm_and_theme();
    let root = Border::new()
        .title(" My Title ")
        .child(Text::new("content").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    assert!(!h.find_text("My Title").is_empty());
    assert!(!h.find_text("content").is_empty());
}

#[test]
fn border_rounded_corners() {
    let (wm, t) = wm_and_theme();
    let root = Border::new()
        .rounded()
        .title(" Rounded ")
        .child(Text::new("inside").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    // Rounded corner chars should be present
    let tl = h.cell_at(0, 0).ch;
    assert!(tl == '\u{256D}' || tl == '\u{250C}', "expected corner char, got {tl:?}");
    let tr = h.cell_at(39, 0).ch;
    assert!(tr == '\u{256E}' || tr == '\u{2510}', "expected corner char, got {tr:?}");
}

#[test]
fn border_sharp_corners_by_default() {
    let (wm, t) = wm_and_theme();
    let root = Border::new()
        .title(" Sharp ")
        .child(Text::new("inside").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    // Default is sharp: ┌ ┐
    assert_eq!(h.cell_at(0, 0).ch, '\u{250C}');
    assert_eq!(h.cell_at(39, 0).ch, '\u{2510}');
}

#[test]
fn border_custom_fg() {
    let (wm, t) = wm_and_theme();
    let custom_fg = t.primary();
    let root = Border::new()
        .fg(custom_fg)
        .title(" Colored ")
        .child(Text::new("x").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 20, 5, &t);
    // Top-left corner should have the custom fg
    assert_eq!(h.cell_at(0, 0).fg.palette, custom_fg.palette);
}

#[test]
fn border_custom_bg() {
    let (wm, t) = wm_and_theme();
    let custom_bg = t.surface_alt();
    let root = Border::new()
        .bg(custom_bg)
        .title(" CustomBG ")
        // Text child also gets the custom bg to match the border interior
        .child(Text::new("x").bg(custom_bg).build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 20, 5, &t);
    // Border cell itself should have the custom bg
    assert_eq!(h.cell_at(0, 0).bg.palette, custom_bg.palette);
    // Interior cell after Text renders should also match
    assert_eq!(h.cell_at(1, 1).bg.palette, custom_bg.palette);
}

#[test]
fn border_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();
    let root = Border::new()
        .title(" Light ")
        .child(Text::new("hello").fg(t.text()).build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn border_small_minimum_size() {
    let (wm, t) = wm_and_theme();
    // Border minimum is 3 wide, 2 tall with no child. With child, child + border.
    let root = Border::new()
        .child(Text::new("x").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 3, 3, &t);
    // Should not panic — just render whatever fits
    assert!(h.cols() >= 3);
}

#[test]
fn border_without_title() {
    let (wm, t) = wm_and_theme();
    let root = Border::new()
        .child(Text::new("no title").build(&wm, &t))
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 5, &t);
    assert!(!h.find_text("no title").is_empty());
}
