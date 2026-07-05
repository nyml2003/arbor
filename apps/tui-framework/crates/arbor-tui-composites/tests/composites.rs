use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_composites::{
    ContentBlock, DividerBlock, FuzzyPanel, FuzzyPanelSelection, Panel, PromptBar, ScrollColumn,
    SectionDivider, SectionedPanel, SectionedPanelSection, StatusLine, Transcript,
    TranscriptMessage, TranscriptNotice,
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
fn transcript_renders_empty_text() {
    let (factory, theme) = wm_and_theme();
    let root = Transcript::new()
        .empty_text("No messages")
        .build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 40, 3, &theme);

    assert!(!harness.find_text("No messages").is_empty());
    assert_eq!(
        Transcript::new()
            .empty_text("No messages")
            .line_count(&theme),
        1
    );
}

#[test]
fn transcript_renders_markdown_message_and_code_block() {
    let (factory, theme) = wm_and_theme();
    let message = TranscriptMessage::new(
        "Aster",
        theme.primary(),
        "hello **world**\n```rust\nlet x = 1;\n```",
    );
    let root = Transcript::new()
        .messages([message.clone()])
        .build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 60, 10, &theme);

    assert!(!harness.find_text("Aster:").is_empty());
    assert!(!harness.find_text("hello").is_empty());
    assert!(!harness.find_text("world").is_empty());
    assert!(!harness.find_text("rust").is_empty());
    assert!(!harness.find_text("let x = 1;").is_empty());
    assert_eq!(Transcript::new().messages([message]).line_count(&theme), 8);
}

#[test]
fn transcript_renders_notice_after_messages() {
    let (factory, theme) = wm_and_theme();
    let message = TranscriptMessage::new("Aster", theme.primary(), "done");
    let notice = TranscriptNotice::new("Error: timeout", "Submit another message.", theme.danger());
    let root = Transcript::new()
        .messages([message.clone()])
        .notice(Some(notice.clone()))
        .build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 60, 6, &theme);

    assert!(!harness.find_text("Error: timeout").is_empty());
    assert!(!harness.find_text("Submit another message.").is_empty());
    assert_eq!(
        Transcript::new()
            .messages([message])
            .notice(Some(notice))
            .line_count(&theme),
        5
    );
}

#[test]
fn transcript_uses_theme_background_in_light_theme() {
    let factory = WidgetFactory::new();
    let theme = Theme::light();
    let root = Transcript::new()
        .messages([TranscriptMessage::new(
            "Aster",
            theme.primary(),
            "hello **world**",
        )])
        .build(&factory, &theme);
    let harness = WidgetHarness::render(&root, 40, 4, &theme);

    assert!(!harness.find_text("world").is_empty());
    harness.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn section_divider_renders_separator_and_label() {
    let (factory, theme) = wm_and_theme();
    let root = SectionDivider::new("Files")
        .divider_width(8)
        .bg(theme.surface_alt())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 20, 1, &theme);

    assert_eq!(row_text(&harness, 0, 14), "╭------╯ Files");
    assert_eq!(harness.cell_at(0, 0).fg, theme.border());
    assert_eq!(harness.cell_at(9, 0).fg, theme.text_dim());
    assert_eq!(harness.cell_at(9, 0).bg, theme.surface_alt());
}

#[test]
fn divider_block_stacks_section_divider_and_body() {
    let (factory, theme) = wm_and_theme();
    let body = Text::new("body").build(&factory, &theme);
    let root = DividerBlock::new("Meta", body)
        .divider_width(6)
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 24, 2, &theme);

    assert_eq!(row_text(&harness, 0, 11), "╭----╯ Meta");
    assert_eq!(harness.find_text("body"), vec![(0, 1)]);
}

#[test]
fn sectioned_panel_draws_connected_sections() {
    let (factory, theme) = wm_and_theme();
    let root = SectionedPanel::new([
        SectionedPanelSection::new("上方主信息区")
            .line("系统名称：TUI 控制面板")
            .line("连接状态：在线"),
        SectionedPanelSection::new("下方详情分区").line("CPU 占用：27%"),
    ])
    .width(36)
    .fg(theme.border())
    .bg(theme.surface_alt())
    .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 36, 8, &theme);

    assert_eq!(
        row_text(&harness, 0, 36),
        "╭──────────────────────────────────╮"
    );
    assert_eq!(
        row_text(&harness, 4, 36),
        "╰─────────────────────────────────╭╯"
    );
    assert_eq!(
        row_text(&harness, 7, 36),
        "╰──────────────────────────────────╯"
    );
    let visible_text = visible_screen_text(&harness);
    assert!(visible_text.contains("【上方主信息区】"));
    assert!(visible_text.contains("系统名称：TUI 控制面板"));
    assert!(visible_text.contains("【下方详情分区】"));
    assert_eq!(harness.cell_at(0, 4).fg, theme.border());
    assert_eq!(harness.cell_at(1, 1).bg, theme.surface_alt());
}

#[test]
fn sectioned_panel_respects_layout_padding_once() {
    let (factory, theme) = wm_and_theme();
    let root = SectionedPanel::new([SectionedPanelSection::new("Meta").line("ready")])
        .padding(RectOffset::all(1))
        .bg(theme.surface_alt())
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 14, 5, &theme);

    assert_eq!(harness.cell_at(0, 0).ch, ' ');
    assert_eq!(harness.cell_at(1, 1).ch, '╭');
    assert_eq!(harness.cell_at(12, 1).ch, '╮');
    assert_eq!(harness.cell_at(1, 4).ch, ' ');
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
fn prompt_bar_forwards_loading_state_to_input() {
    let (factory, theme) = wm_and_theme();
    let root = PromptBar::new()
        .placeholder("waiting for agent")
        .loading(true)
        .loading_phase(1)
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 32, 3, &theme);

    assert!(!harness.find_text("◐ waiting for agent").is_empty());
    let (col, row) = harness.find_text("◐").first().copied().unwrap();
    assert_eq!(harness.cell_at(col, row).fg, theme.warning());
}

fn row_text(harness: &WidgetHarness, row: u16, width: u16) -> String {
    (0..width).map(|col| harness.cell_at(col, row).ch).collect()
}

fn visible_screen_text(harness: &WidgetHarness) -> String {
    let mut text = String::new();
    for row in 0..harness.rows() {
        for col in 0..harness.cols() {
            let cell = harness.cell_at(col, row);
            if !cell.phantom {
                text.push(cell.ch);
            }
        }
        if row + 1 < harness.rows() {
            text.push('\n');
        }
    }
    text
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
fn fuzzy_panel_accepts_initial_query_and_selection() {
    let (factory, theme) = wm_and_theme();
    let root = FuzzyPanel::new(["alpha", "beta", "gamma"])
        .query("ga")
        .selected_index(0)
        .build(&factory, &theme);

    let harness = WidgetHarness::render(&root, 48, 8, &theme);

    assert!(!harness.find_text("ga").is_empty());
    assert!(!harness.find_text("gamma").is_empty());
    assert!(harness.find_text("alpha").is_empty());
    assert!(!harness.find_text("1/1 matches").is_empty());
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
