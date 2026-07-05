use arbor_tui_domain::cell::Attrs;
use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::divider::Divider;
use arbor_tui_widgets::widget_factory::WidgetFactory;

#[test]
fn divider_renders_cornered_separator_across_width() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Divider::new().width(10).build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 10, 1, &theme);

    assert_row(&harness, 0, "╭--------╯");
    assert_eq!(harness.cell_at(0, 0).fg, theme.border());
    assert_eq!(harness.cell_at(0, 0).bg, theme.surface());
}

#[test]
fn divider_can_customize_glyphs_and_style() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Divider::new()
        .left('<')
        .fill('=')
        .right('>')
        .width(6)
        .fg(theme.accent())
        .bg(theme.surface_alt())
        .bold()
        .build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 6, 1, &theme);

    assert_row(&harness, 0, "<====>");
    for col in 0..6 {
        let cell = harness.cell_at(col, 0);
        assert_eq!(cell.fg, theme.accent());
        assert_eq!(cell.bg, theme.surface_alt());
        assert_eq!(
            cell.attrs,
            Attrs {
                bold: true,
                ..Default::default()
            }
        );
    }
}

#[test]
fn divider_clips_safely_in_narrow_widths() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();

    let one = Divider::new().width(1).build(&factory, &theme);
    let one = WidgetHarness::render(&one, 1, 1, &theme);
    assert_row(&one, 0, "╭");

    let two = Divider::new().width(2).build(&factory, &theme);
    let two = WidgetHarness::render(&two, 2, 1, &theme);
    assert_row(&two, 0, "╭╯");
}

fn assert_row(harness: &WidgetHarness, row: u16, expected: &str) {
    let actual = (0..harness.cols())
        .map(|col| harness.cell_at(col, row).ch)
        .collect::<String>();
    assert_eq!(actual, expected);
}
