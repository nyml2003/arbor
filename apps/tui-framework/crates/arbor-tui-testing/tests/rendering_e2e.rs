mod common;

use arbor_tui_domain::cell::{AnsiColor, Attrs, Span};
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::table::{ColumnDef, ColumnWidth, Table};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, mounted};

#[test]
fn initial_text_render_emits_output_and_updates_screen() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("hello e2e").build(&factory, &theme);
    let driver = mounted(root, 20, 1, theme);

    assert_has_text(&driver, "hello e2e");
    assert!(driver.output_contains("CSI"));
    assert!(driver.output_len() > 0);
}

#[test]
fn initial_render_reports_frame_stats_waterline() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("hello e2e").build(&factory, &theme);
    let driver = mounted(root, 20, 1, theme);
    let stats = driver.last_frame_stats();

    assert_eq!(stats.frame_seq, 1);
    assert!(
        stats.dirty_regions > 0,
        "initial render should report emitted dirty regions"
    );
}

#[test]
fn idle_tick_after_stable_render_does_not_emit_more_output() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("stable").build(&factory, &theme);
    let mut driver = mounted(root, 20, 1, theme);
    let output_len = driver.output_len();

    let step = driver.tick([]).unwrap();

    assert!(!step.should_render);
    assert_eq!(driver.output_len(), output_len);
}

#[test]
fn light_theme_visible_text_has_no_default_black_background() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Border::new()
        .title(" Card ")
        .child(Text::new("light text").build(&factory, &theme))
        .build(&factory, &theme);
    let driver = mounted(root, 30, 4, theme);

    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn dark_theme_text_uses_theme_surface_background() {
    let theme = Theme::dark();
    let surface = theme.surface();
    let factory = WidgetFactory::new();
    let root = Text::new("dark text").build(&factory, &theme);
    let driver = mounted(root, 20, 1, theme);

    let (col, row) = driver.find_text("dark text")[0];
    assert_eq!(driver.cell_at(col, row).bg, surface);
}

#[test]
fn border_title_and_content_render_together() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Border::new()
        .title(" Panel ")
        .child(Text::new("inside").build(&factory, &theme))
        .build(&factory, &theme);
    let driver = mounted(root, 30, 4, theme);

    assert_has_text(&driver, "Panel");
    assert_has_text(&driver, "inside");
}

#[test]
fn row_three_columns_render_in_order() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Row::new()
        .children([
            Text::new("left").build(&factory, &theme),
            Text::new("middle").build(&factory, &theme),
            Text::new("right").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let driver = mounted(root, 40, 1, theme);

    let left = driver.find_text("left")[0].0;
    let middle = driver.find_text("middle")[0].0;
    let right = driver.find_text("right")[0].0;
    assert!(left < middle && middle < right);
}

#[test]
fn col_header_body_footer_render_vertically() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Col::new()
        .children([
            Text::new("header").build(&factory, &theme),
            Text::new("body").build(&factory, &theme),
            Text::new("footer").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let driver = mounted(root, 20, 3, theme);

    assert_eq!(driver.find_text("header")[0].1, 0);
    assert_eq!(driver.find_text("body")[0].1, 1);
    assert_eq!(driver.find_text("footer")[0].1, 2);
}

#[test]
fn rich_text_keeps_span_style_on_screen_cells() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let fg = AnsiColor::from_palette(10);
    let bg = theme.surface();
    let root = RichText::new()
        .line(vec![Span::new(
            "green",
            fg,
            bg,
            Attrs {
                bold: true,
                ..Default::default()
            },
        )])
        .build(&factory, &theme);
    let driver = mounted(root, 20, 1, theme);

    let cell = driver.cell_at(0, 0);
    assert_eq!(cell.ch, 'g');
    assert_eq!(cell.fg, fg);
    assert!(cell.attrs.bold);
}

#[test]
fn table_header_separator_and_data_render() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Table::new()
        .columns(vec![
            ColumnDef {
                header: "Name".to_string(),
                width: ColumnWidth::Fixed(8),
            },
            ColumnDef {
                header: "State".to_string(),
                width: ColumnWidth::Fixed(8),
            },
        ])
        .cells(vec![vec!["Job".to_string(), "Ready".to_string()]])
        .build(&factory, &theme);
    let driver = mounted(root, 20, 4, theme.clone());

    assert_has_text(&driver, "Name");
    assert_has_text(&driver, "Ready");
    assert_eq!(driver.cell_at(0, 1).bg, theme.border());
}

#[test]
fn one_by_one_viewport_renders_without_panic() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Text::new("x").build(&factory, &theme);
    let driver = mounted(root, 1, 1, theme);

    assert_eq!(driver.screen().cols(), 1);
    assert_eq!(driver.screen().rows(), 1);
    assert_eq!(driver.cell_at(0, 0).ch, 'x');
}
