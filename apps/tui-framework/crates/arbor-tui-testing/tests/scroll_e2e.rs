mod common;

use arbor_tui_domain::input::Key;
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::table::{ColumnDef, ColumnWidth, Table};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, assert_not_text, mounted, numbered};

fn multiline(prefix: &str, count: usize) -> String {
    numbered(prefix, count).join("\n")
}

fn rows(count: usize) -> Vec<Vec<String>> {
    (0..count)
        .map(|i| vec![format!("row {i:02}"), format!("value {i:02}")])
        .collect()
}

#[test]
fn scroll_initial_position_shows_top_content() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Scroll::new()
        .content_h(8)
        .child(Text::new(multiline("line", 8)).build(&factory, &theme))
        .build(&factory, &theme);
    let driver = mounted(root, 20, 3, theme);

    assert_has_text(&driver, "line 00");
    assert_has_text(&driver, "line 01");
}

#[test]
fn scroll_signal_moves_viewport_down() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let scroll_y = Signal::new(0u16);
    let root = Scroll::new()
        .content_h(8)
        .scroll_y(scroll_y.read_only())
        .child(Text::new(multiline("line", 8)).build(&factory, &theme))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 3, theme);

    driver.update_signal(&scroll_y, 3).unwrap();

    assert_has_text(&driver, "line 03");
    assert_not_text(&driver, "line 00");
}

#[test]
fn scroll_signal_beyond_content_does_not_panic() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let scroll_y = Signal::new(0u16);
    let root = Scroll::new()
        .content_h(3)
        .scroll_y(scroll_y.read_only())
        .child(Text::new(multiline("line", 3)).build(&factory, &theme))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 3, theme);

    driver.update_signal(&scroll_y, 99).unwrap();

    assert_eq!(driver.screen().cols(), 20);
    assert_eq!(driver.screen().rows(), 3);
    assert_not_text(&driver, "line 00");
}

#[test]
fn scroll_inside_light_border_has_no_black_visible_text() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Border::new()
        .title(" Log ")
        .child(
            Scroll::new()
                .content_h(4)
                .child(Text::new(multiline("entry", 4)).build(&factory, &theme))
                .build(&factory, &theme),
        )
        .build(&factory, &theme);
    let driver = mounted(root, 30, 5, theme);

    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn scroll_renders_transparent_column_child() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let child = Col::new()
        .children([
            Text::new("alpha").build(&factory, &theme),
            Text::new("beta").build(&factory, &theme),
            Text::new("gamma").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let root = Scroll::new()
        .content_h(3)
        .child(child)
        .build(&factory, &theme);
    let driver = mounted(root, 20, 2, theme);

    assert_has_text(&driver, "alpha");
    assert_has_text(&driver, "beta");
}

#[test]
fn list_arrow_down_selects_first_item() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = List::new()
        .items(numbered("item", 5))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 3, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();

    assert_has_text(&driver, "item 00");
    let (col, row) = driver.find_text("item 00")[0];
    assert_eq!(driver.cell_at(col, row).bg, Theme::dark().accent());
}

#[test]
fn list_selection_scrolls_down_to_visible_item() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = List::new()
        .items(numbered("item", 12))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 3, theme);

    driver.focus_next().unwrap();
    for _ in 0..8 {
        driver.send_key(Key::ArrowDown).unwrap();
    }

    assert_has_text(&driver, "item 07");
    assert_not_text(&driver, "item 00");
}

#[test]
fn list_selection_scrolls_up_to_visible_item() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = List::new()
        .items(numbered("item", 12))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 3, theme);

    driver.focus_next().unwrap();
    for _ in 0..8 {
        driver.send_key(Key::ArrowDown).unwrap();
    }
    for _ in 0..5 {
        driver.send_key(Key::ArrowUp).unwrap();
    }

    assert_has_text(&driver, "item 02");
}

#[test]
fn table_arrow_down_selects_first_row() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Table::new()
        .columns(vec![ColumnDef {
            header: "Name".to_string(),
            width: ColumnWidth::Fixed(10),
        }])
        .cells(rows(5))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 5, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();

    assert_has_text(&driver, "row 00");
    let (col, row) = driver.find_text("row 00")[0];
    assert_eq!(driver.cell_at(col, row).bg, Theme::dark().accent());
}

#[test]
fn table_selection_scrolls_down_to_visible_row() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Table::new()
        .columns(vec![ColumnDef {
            header: "Name".to_string(),
            width: ColumnWidth::Fixed(10),
        }])
        .cells(rows(12))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 5, theme);

    driver.focus_next().unwrap();
    for _ in 0..8 {
        driver.send_key(Key::ArrowDown).unwrap();
    }

    assert_has_text(&driver, "row 07");
    assert_not_text(&driver, "row 00");
}

#[test]
fn table_selection_scrolls_up_to_visible_row() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Table::new()
        .columns(vec![ColumnDef {
            header: "Name".to_string(),
            width: ColumnWidth::Fixed(10),
        }])
        .cells(rows(12))
        .build(&factory, &theme);
    let mut driver = mounted(root, 20, 5, theme);

    driver.focus_next().unwrap();
    for _ in 0..8 {
        driver.send_key(Key::ArrowDown).unwrap();
    }
    for _ in 0..5 {
        driver.send_key(Key::ArrowUp).unwrap();
    }

    assert_has_text(&driver, "row 02");
}

#[test]
fn long_list_scroll_indicator_does_not_hide_text() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = List::new()
        .items(numbered("item", 40))
        .build(&factory, &theme);
    let driver = mounted(root, 20, 4, theme);

    assert_has_text(&driver, "item 00");
    assert_eq!(driver.cell_at(1, 0).ch, 'i');
}
