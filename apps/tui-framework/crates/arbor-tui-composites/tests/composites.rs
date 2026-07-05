use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_composites::{
    ContentBlock, FuzzyPanel, FuzzyPanelSelection, Panel, PromptBar, ScrollColumn, StatusLine,
};
use arbor_tui_domain::input::Key;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::{TuiTestDriver, WidgetHarness};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

#[test]
fn panel_renders_title_child_and_colors() {
    let (factory, theme) = wm_and_theme();
    let body = Text::new("panel body").build(&factory, &theme);
    let root = Panel::new(body)
        .title(" Panel ")
        .rounded()
        .fg(theme.primary())
        .bg(theme.surface_alt())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 30, 5, &theme);

    assert!(!harness.find_text("Panel").is_empty());
    assert!(!harness.find_text("panel body").is_empty());
    assert_eq!(harness.cell_at(0, 0).fg.palette, theme.primary().palette);
    assert_eq!(
        harness.cell_at(0, 0).bg.palette,
        theme.surface_alt().palette
    );
}

#[test]
fn panel_clips_overflowing_child_to_interior() {
    let (factory, theme) = wm_and_theme();
    let body = Text::new("row1\nrow2\nrow3").build(&factory, &theme);
    let root = Panel::new(body).build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 10, 4, &theme);

    assert!(!harness.find_text("row1").is_empty());
    assert!(!harness.find_text("row2").is_empty());
    assert!(harness.find_text("row3").is_empty());
    assert_eq!(harness.cell_at(0, 3).ch, '\u{2514}');
    assert_eq!(harness.cell_at(9, 3).ch, '\u{2518}');
}

#[test]
fn content_block_keeps_widget_with_declared_line_count() {
    let (factory, theme) = wm_and_theme();
    let block = ContentBlock::new(Text::new("x").build(&factory, &theme), 3);

    assert_eq!(block.line_count(), 3);
}

#[test]
fn scroll_column_uses_summed_block_line_counts_for_scroll_height() {
    let (factory, theme) = wm_and_theme();
    let scroll_y = Signal::new(1u16);
    let blocks = [
        ContentBlock::new(Text::new("top").build(&factory, &theme), 1),
        ContentBlock::new(Text::new("middle").build(&factory, &theme), 1),
        ContentBlock::new(Text::new("bottom").build(&factory, &theme), 1),
    ];
    let root = ScrollColumn::new()
        .blocks(blocks)
        .scroll_y(scroll_y.read_only())
        .flex(1.0)
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 20, 2, &theme);

    assert!(harness.find_text("top").is_empty());
    assert!(!harness.find_text("middle").is_empty());
    assert!(!harness.find_text("bottom").is_empty());
}

#[test]
fn status_line_renders_with_padding_and_colors() {
    let (factory, theme) = wm_and_theme();
    let root = StatusLine::new("ready")
        .fg(theme.warning())
        .bg(theme.surface_alt())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 20, 1, &theme);

    assert_eq!(harness.cell_at(0, 0).ch, ' ');
    assert!(!harness.find_text("ready").is_empty());
    assert_eq!(harness.cell_at(1, 0).fg.palette, theme.warning().palette);
    assert_eq!(
        harness.cell_at(1, 0).bg.palette,
        theme.surface_alt().palette
    );
}

#[test]
fn status_line_allows_padding_override() {
    let (factory, theme) = wm_and_theme();
    let root = StatusLine::new("ready")
        .padding(RectOffset::default())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 20, 1, &theme);

    assert_eq!(harness.find_text("ready"), vec![(0, 0)]);
}

#[test]
fn prompt_bar_renders_placeholder_inside_border() {
    let (factory, theme) = wm_and_theme();
    let root = PromptBar::new()
        .title(" Prompt ")
        .rounded()
        .placeholder("Type here")
        .fg(theme.primary())
        .bg(theme.surface())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 30, 3, &theme);

    assert!(!harness.find_text("Type here").is_empty());
    assert!(!harness.find_text("Prompt").is_empty());
    assert_eq!(harness.cell_at(0, 0).fg.palette, theme.primary().palette);
}

#[test]
fn prompt_bar_submit_callback_uses_nested_input_event_path() {
    let (factory, theme) = wm_and_theme();
    let submitted = Rc::new(RefCell::new(String::new()));
    let submitted_for_cb = Rc::clone(&submitted);
    let root = PromptBar::new()
        .placeholder("Type")
        .on_submit(move |value| {
            *submitted_for_cb.borrow_mut() = value;
        })
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 30, 3, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("deploy").unwrap();
    driver.send_key(Key::Enter).unwrap();

    assert_eq!(submitted.borrow().as_str(), "deploy");
}

#[test]
fn fuzzy_panel_renders_prompt_items_and_status() {
    let (factory, theme) = wm_and_theme();
    let root = FuzzyPanel::new(["src/main.rs", "README.md", "Cargo.toml"])
        .title(" Files ")
        .rounded()
        .placeholder("Search files")
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 48, 8, &theme);

    assert!(!harness.find_text("Files").is_empty());
    assert!(!harness.find_text("Search files").is_empty());
    assert!(!harness.find_text("src/main.rs").is_empty());
    assert!(!harness.find_text("README.md").is_empty());
    assert!(!harness.find_text("1/3 matches").is_empty());
}

#[test]
fn fuzzy_panel_filters_items_from_typed_query() {
    let (factory, theme) = wm_and_theme();
    let root = FuzzyPanel::new(["src/main.rs", "README.md", "Cargo.toml"])
        .placeholder("Search files")
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 48, 8, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("read").unwrap();

    assert!(!driver.find_text("read").is_empty());
    assert!(!driver.find_text("README.md").is_empty());
    assert!(driver.find_text("src/main.rs").is_empty());
    assert!(!driver.find_text("1/1 matches").is_empty());
}

#[test]
fn fuzzy_panel_renders_empty_text_when_no_items_match() {
    let (factory, theme) = wm_and_theme();
    let root = FuzzyPanel::new(["src/main.rs", "README.md"])
        .empty_text("Nothing found")
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 48, 8, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("zzz").unwrap();

    assert!(!driver.find_text("Nothing found").is_empty());
    assert!(!driver.find_text("0/0 matches").is_empty());
}

#[test]
fn fuzzy_panel_submits_selected_original_item() {
    let (factory, theme) = wm_and_theme();
    let selected = Rc::new(RefCell::new(None::<FuzzyPanelSelection>));
    let selected_for_cb = Rc::clone(&selected);
    let root = FuzzyPanel::new(["alpha", "beta", "gamma"])
        .on_submit(move |selection| {
            *selected_for_cb.borrow_mut() = Some(selection);
        })
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 48, 8, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_key(Key::ArrowDown).unwrap();
    driver.send_key(Key::Enter).unwrap();

    assert_eq!(
        selected.borrow().as_ref(),
        Some(&FuzzyPanelSelection {
            index: 1,
            item: "beta".to_string(),
        })
    );
}

#[test]
fn fuzzy_panel_emits_query_changes() {
    let (factory, theme) = wm_and_theme();
    let queries = Rc::new(RefCell::new(Vec::<String>::new()));
    let queries_for_cb = Rc::clone(&queries);
    let root = FuzzyPanel::new(["alpha", "beta"])
        .on_query_change(move |query| {
            queries_for_cb.borrow_mut().push(query);
        })
        .build(&factory, &theme);
    let mut driver = TuiTestDriver::new(root, 48, 8, theme);

    driver.render_initial().unwrap();
    driver.focus_next().unwrap();
    driver.send_chars("ab").unwrap();
    driver.send_key(Key::Backspace).unwrap();

    assert_eq!(
        queries.borrow().as_slice(),
        ["a".to_string(), "ab".to_string(), "a".to_string()]
    );
}
