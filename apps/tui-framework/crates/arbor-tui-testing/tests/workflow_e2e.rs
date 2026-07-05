mod common;

use arbor_tui_domain::input::Key;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::table::{ColumnDef, ColumnWidth, Table};
use arbor_tui_widgets::tabs::{TabDef, Tabs};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, assert_not_text, mounted, numbered};

#[test]
fn tabs_arrow_right_switches_active_content() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "General".to_string(),
                content: Text::new("general content").build(&factory, &theme),
            },
            TabDef {
                label: "Advanced".to_string(),
                content: Text::new("advanced content").build(&factory, &theme),
            },
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 40, 5, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowRight).unwrap();

    assert_has_text(&driver, "advanced content");
    assert_not_text(&driver, "general content");
}

#[test]
fn tabs_arrow_left_wraps_from_first_to_last() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "One".to_string(),
                content: Text::new("one body").build(&factory, &theme),
            },
            TabDef {
                label: "Two".to_string(),
                content: Text::new("two body").build(&factory, &theme),
            },
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 40, 5, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowLeft).unwrap();

    assert_has_text(&driver, "two body");
    assert_not_text(&driver, "one body");
}

#[test]
fn active_tab_child_input_can_receive_focus_and_text() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "Read".to_string(),
                content: Text::new("read only").build(&factory, &theme),
            },
            TabDef {
                label: "Edit".to_string(),
                content: Input::new().placeholder("edit").build(&factory, &theme),
            },
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 40, 5, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowRight).unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("typed").unwrap();

    assert_has_text(&driver, "typed");
    assert_not_text(&driver, "edit");
}

#[test]
fn inactive_tab_child_text_is_not_visible() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Tabs::new(0)
        .tabs(vec![
            TabDef {
                label: "Shown".to_string(),
                content: Text::new("visible body").build(&factory, &theme),
            },
            TabDef {
                label: "Hidden".to_string(),
                content: Text::new("hidden body").build(&factory, &theme),
            },
        ])
        .build(&factory, &theme);
    let driver = mounted(root, 40, 5, theme);

    assert_has_text(&driver, "visible body");
    assert_not_text(&driver, "hidden body");
}

#[test]
fn border_tabs_input_composition_keeps_layout_while_typing() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let root = Border::new()
        .title(" Settings ")
        .child(
            Tabs::new(0)
                .tabs(vec![TabDef {
                    label: "Form".to_string(),
                    content: Input::new().placeholder("name").build(&factory, &theme),
                }])
                .build(&factory, &theme),
        )
        .build(&factory, &theme);
    let mut driver = mounted(root, 50, 7, theme);

    driver.focus_next().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("alice").unwrap();

    assert_has_text(&driver, "Settings");
    assert_has_text(&driver, "Form");
    assert_has_text(&driver, "alice");
}

#[test]
fn dashboard_script_updates_list_table_and_footer_input() {
    let theme = Theme::dark();
    let factory = WidgetFactory::new();
    let header = Border::new()
        .title(" Header ")
        .child(Text::new("dashboard").build(&factory, &theme))
        .build(&factory, &theme);
    let list = Border::new()
        .title(" Jobs ")
        .flex(1.0)
        .child(
            List::new()
                .flex(1.0)
                .items(numbered("job", 8))
                .build(&factory, &theme),
        )
        .build(&factory, &theme);
    let table = Border::new()
        .title(" Details ")
        .flex(2.0)
        .child(
            Table::new()
                .flex(1.0)
                .columns(vec![
                    ColumnDef {
                        header: "Name".to_string(),
                        width: ColumnWidth::Fixed(10),
                    },
                    ColumnDef {
                        header: "State".to_string(),
                        width: ColumnWidth::Fixed(10),
                    },
                ])
                .cells(vec![
                    vec!["api".to_string(), "ready".to_string()],
                    vec!["worker".to_string(), "busy".to_string()],
                ])
                .build(&factory, &theme),
        )
        .build(&factory, &theme);
    let footer = Input::new().placeholder("command").build(&factory, &theme);
    let root = Col::new()
        .children([
            header,
            Row::new()
                .flex(1.0)
                .children([list, table])
                .build(&factory, &theme),
            footer,
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 80, 12, theme);

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("deploy").unwrap();

    assert_has_text(&driver, "dashboard");
    assert_has_text(&driver, "job 00");
    assert_has_text(&driver, "api");
    assert_has_text(&driver, "deploy");
}
