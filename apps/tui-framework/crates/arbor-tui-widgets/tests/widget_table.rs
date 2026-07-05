use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::table::{ColumnDef, ColumnWidth, Table};
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

#[test]
fn flex_columns_use_declared_weight_ratio() {
    let (wm, t) = wm_and_theme();
    let table = Table::new()
        .columns(vec![
            ColumnDef {
                header: "Fix".to_string(),
                width: ColumnWidth::Fixed(4),
            },
            ColumnDef {
                header: "A".to_string(),
                width: ColumnWidth::Flex(1.0),
            },
            ColumnDef {
                header: "BBBBBBBB".to_string(),
                width: ColumnWidth::Flex(2.0),
            },
        ])
        .build(&wm, &t);

    let h = WidgetHarness::render(&table, 16, 3, &t);

    assert_eq!(
        h.cell_at(8, 0).ch,
        'B',
        "the 2x flex column should start after fixed 4 + flex 4 columns"
    );
}
