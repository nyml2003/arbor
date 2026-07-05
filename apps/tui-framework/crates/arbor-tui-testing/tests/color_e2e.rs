mod common;

use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell, Span};
use arbor_tui_domain::input::Key;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::table::{ColumnDef, ColumnWidth, Table};
use arbor_tui_widgets::tabs::{TabDef, Tabs};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use common::{assert_has_text, assert_not_text, mounted, numbered};

fn mounted_ansi(
    root: WidgetNode,
    cols: u16,
    rows: u16,
    theme: Theme,
) -> arbor_tui_testing::AnsiTuiTestDriver {
    let mut driver = arbor_tui_testing::AnsiTuiTestDriver::new(root, cols, rows, theme);
    driver.render_initial().unwrap();
    driver
}

fn assert_row_bg_is_one_of(
    driver: &arbor_tui_testing::TuiTestDriver,
    row: u16,
    start_col: u16,
    end_col: u16,
    allowed: &[AnsiColor],
) {
    for col in start_col..end_col {
        let cell = driver.cell_at(col, row);
        assert!(
            allowed.contains(&cell.bg),
            "cell ({col},{row}) had unexpected bg {:?}; row={:?}",
            cell.bg,
            driver.row_text(row)
        );
    }
}

fn assert_ansi_row_bg_is_one_of(
    driver: &arbor_tui_testing::AnsiTuiTestDriver,
    row: u16,
    start_col: u16,
    end_col: u16,
    allowed: &[AnsiColor],
) {
    for col in start_col..end_col {
        let cell = driver.cell_at(col, row);
        assert!(
            allowed.contains(&cell.bg),
            "cell ({col},{row}) had unexpected bg {:?}; row={:?}",
            cell.bg,
            driver.row_text(row)
        );
    }
}

#[test]
fn nested_light_input_after_typing_keeps_full_input_row_colored() {
    let theme = Theme::light();
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
    let mut driver = mounted(root, 50, 7, theme.clone());

    driver.focus_next().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("alice").unwrap();

    let (text_col, row) = driver.find_text("alice")[0];
    let prompt_col = text_col.saturating_sub(2);
    assert_row_bg_is_one_of(
        &driver,
        row,
        prompt_col,
        48,
        &[theme.surface_alt(), theme.primary()],
    );
    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn placeholder_tail_is_repainted_with_input_background_after_typing() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Input::new()
        .placeholder("placeholder")
        .build(&factory, &theme);
    let mut driver = mounted(root, 24, 1, theme.clone());

    driver.focus_next().unwrap();
    driver.send_chars("x").unwrap();

    assert_has_text(&driver, "x");
    assert_not_text(&driver, "placeholder");
    for col in 2..24 {
        let bg = driver.cell_at(col, 0).bg;
        assert!(
            bg == theme.surface_alt() || bg == theme.primary(),
            "col {col} had unexpected input bg {bg:?}"
        );
    }
}

#[test]
fn ansi_replay_placeholder_tail_is_repainted_with_input_background_after_typing() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Input::new()
        .placeholder("placeholder")
        .build(&factory, &theme);
    let mut driver = mounted_ansi(root, 24, 1, theme.clone());

    driver.focus_next().unwrap();
    driver.send_chars("x").unwrap();

    assert!(
        !driver.find_text("x").is_empty(),
        "expected replayed screen to contain typed text\n{}",
        driver.visible_text()
    );
    assert!(
        driver.find_text("placeholder").is_empty(),
        "expected replayed screen to remove placeholder\n{}",
        driver.visible_text()
    );
    assert!(driver.output_contains("\x1b[48;5;15m"));
    assert!(!driver.output_contains("\x1b[K"));
    for col in 2..24 {
        let bg = driver.cell_at(col, 0).bg;
        assert!(
            bg == theme.surface_alt() || bg == theme.primary(),
            "col {col} had unexpected input bg {bg:?}"
        );
    }
}

#[test]
fn ansi_replay_nested_light_input_after_typing_keeps_full_input_row_colored() {
    let theme = Theme::light();
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
    let mut driver = mounted_ansi(root, 50, 7, theme.clone());

    driver.focus_next().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("alice").unwrap();

    let (text_col, row) = driver.find_text("alice")[0];
    let prompt_col = text_col.saturating_sub(2);
    assert_ansi_row_bg_is_one_of(
        &driver,
        row,
        prompt_col,
        48,
        &[theme.surface_alt(), theme.primary()],
    );
    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn complex_light_dashboard_selected_rows_and_footer_have_forced_colors() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
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
            Text::new("dashboard").build(&factory, &theme),
            Row::new()
                .flex(1.0)
                .children([list, table])
                .build(&factory, &theme),
            footer,
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 80, 10, theme.clone());

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("deploy").unwrap();

    let (job_col, job_row) = driver.find_text("job 00")[0];
    let (api_col, api_row) = driver.find_text("api")[0];
    let (deploy_col, deploy_row) = driver.find_text("deploy")[0];
    assert_eq!(driver.cell_at(job_col, job_row).bg, theme.accent());
    assert_eq!(driver.cell_at(api_col, api_row).bg, theme.accent());
    assert_eq!(
        driver.cell_at(deploy_col, deploy_row).bg,
        theme.surface_alt()
    );
    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn ansi_replay_complex_light_dashboard_selected_rows_and_footer_have_forced_colors() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
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
            Text::new("dashboard").build(&factory, &theme),
            Row::new()
                .flex(1.0)
                .children([list, table])
                .build(&factory, &theme),
            footer,
        ])
        .build(&factory, &theme);
    let mut driver = mounted_ansi(root, 80, 10, theme.clone());

    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("deploy").unwrap();

    let (job_col, job_row) = driver.find_text("job 00")[0];
    let (api_col, api_row) = driver.find_text("api")[0];
    let (deploy_col, deploy_row) = driver.find_text("deploy")[0];
    assert_eq!(driver.cell_at(job_col, job_row).bg, theme.accent());
    assert_eq!(driver.cell_at(api_col, api_row).bg, theme.accent());
    assert_eq!(
        driver.cell_at(deploy_col, deploy_row).bg,
        theme.surface_alt()
    );
    driver.assert_no_default_black_on_visible_text().unwrap();
}

#[test]
fn rich_text_custom_background_survives_unrelated_input_update() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let rich_bg = AnsiColor::from_palette(13);
    let rich = RichText::new()
        .line(vec![Span::new(
            "status",
            theme.text(),
            rich_bg,
            Attrs::default(),
        )])
        .bg(Cell {
            bg: rich_bg,
            ..Default::default()
        })
        .build(&factory, &theme);
    let root = Col::new()
        .children([
            rich,
            Input::new().placeholder("cmd").build(&factory, &theme),
        ])
        .build(&factory, &theme);
    let mut driver = mounted(root, 30, 2, theme.clone());

    driver.focus_next().unwrap();
    driver.send_chars("run").unwrap();

    let (status_col, status_row) = driver.find_text("status")[0];
    let (run_col, run_row) = driver.find_text("run")[0];
    assert_eq!(driver.cell_at(status_col, status_row).bg, rich_bg);
    assert_eq!(driver.cell_at(run_col, run_row).bg, theme.surface_alt());
}

#[test]
fn focused_input_cursor_uses_primary_background_in_nested_layout() {
    let theme = Theme::light();
    let factory = WidgetFactory::new();
    let root = Border::new()
        .title(" Command ")
        .child(Input::new().placeholder("cmd").build(&factory, &theme))
        .build(&factory, &theme);
    let mut driver = mounted(root, 32, 3, theme.clone());

    driver.focus_next().unwrap();
    driver.send_chars("go").unwrap();

    let (go_col, row) = driver.find_text("go")[0];
    let cursor_col = go_col + 2;
    assert_eq!(driver.cell_at(cursor_col, row).bg, theme.primary());
    assert_eq!(driver.cell_at(cursor_col, row).fg, theme.surface());
}
