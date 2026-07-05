// Layout demo — header / 3-column body / footer, all with rounded borders.
// Type "/theme dark" or "/theme light" in the footer input and press Enter.
// Type in the center fuzzy panel to filter files. ^C/q exits.

use std::cell::{Cell as StdCell, RefCell};
use std::rc::Rc;

use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell, Span};
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::{Theme, ThemeVariant};
use arbor_tui_domain::widget::WidgetNode;

use arbor_tui_composites::{FuzzyPanel, Panel, PromptBar, SectionDivider, StatusLine};
use arbor_tui_runtime::{run_crossterm_terminal_app, TerminalApp};
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn main() {
    if let Err(e) = run() {
        eprintln!("[layout_demo] {e:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let theme = Rc::new(RefCell::new(Theme::dark()));
    let theme_changed = Rc::new(StdCell::new(false));
    let factory = Rc::new(WidgetFactory::new());
    let initial_theme = theme.borrow().clone();

    let build_factory = Rc::clone(&factory);
    let build_theme_changed = Rc::clone(&theme_changed);
    let build_theme = Rc::clone(&theme);
    let app = TerminalApp::with_builder(initial_theme, move |cols, rows, active_theme| {
        build_ui(
            &build_factory,
            active_theme,
            cols,
            rows,
            &build_theme_changed,
            &build_theme,
        )
    });

    let update_factory = Rc::clone(&factory);
    let update_theme_changed = Rc::clone(&theme_changed);
    let update_theme = Rc::clone(&theme);
    let app = app.before_render(move |app, root, active_theme| {
        if !update_theme_changed.get() {
            return false;
        }

        update_theme_changed.set(false);
        *active_theme = update_theme.borrow().clone();
        let (cols, rows) = app.screen_size();
        *root = build_ui(
            &update_factory,
            active_theme,
            cols,
            rows,
            &update_theme_changed,
            &update_theme,
        );
        true
    });

    run_crossterm_terminal_app(app)
}

fn build_ui(
    factory: &WidgetFactory,
    t: &Theme,
    cols: u16,
    rows: u16,
    theme_changed: &Rc<StdCell<bool>>,
    theme_rc: &Rc<RefCell<Theme>>,
) -> WidgetNode {
    let left_w = cols / 5;
    let right_w = cols / 4;
    let panel_bg = demo_panel_bg(t);
    let panel_cell = demo_panel_cell(t);

    // ── Header ─────────────────────────────────────────────────
    let theme_name = match t.variant {
        ThemeVariant::Dark => "dark",
        ThemeVariant::Light => "light",
        ThemeVariant::HighContrast => "hc",
    };
    let header = Panel::new(
        StatusLine::new(format!("Theme: {theme_name}  |  ^C/q to quit"))
            .fg(t.text_dim())
            .bg(panel_bg)
            .padding(RectOffset::default())
            .build(factory, t),
    )
    .rounded()
    .fg(demo_border_fg(t, t.accent()))
    .bg(panel_bg)
    .title(" Arbor TUI ")
    .build(factory, t);

    // ── Footer with theme-switching input ──────────────────────
    let t_clone = theme_rc.clone();
    let changed = theme_changed.clone();
    let footer = PromptBar::new()
        .rounded()
        .fg(demo_border_fg(t, t.accent()))
        .bg(panel_bg)
        .title(" Commands ")
        .placeholder("type /theme dark  |  /theme light")
        .on_submit(move |cmd| {
            let cmd = cmd.trim();
            if cmd == "/theme dark" {
                *t_clone.borrow_mut() = Theme::dark();
            } else if cmd == "/theme light" {
                *t_clone.borrow_mut() = Theme::light();
            }
            changed.set(true);
        })
        .build(factory, t);

    // ── Body: 3 columns ────────────────────────────────────────
    let left_content = Col::new()
        .children([
            SectionDivider::new("Menu")
                .divider_width(6)
                .divider_fg(demo_border_fg(t, t.primary()))
                .label_fg(t.text_dim())
                .bg(panel_bg)
                .build(factory, t),
            Text::new("  Home\n  Projects\n  Settings")
                .fg(t.text())
                .bg(panel_bg)
                .build(factory, t),
            SectionDivider::new("Status")
                .divider_width(6)
                .divider_fg(demo_border_fg(t, t.success()))
                .label_fg(t.text_dim())
                .bg(panel_bg)
                .build(factory, t),
            Text::new("  CPU  12%\n  RAM  3.2G")
                .fg(t.text_dim())
                .bg(panel_bg)
                .italic()
                .build(factory, t),
        ])
        .build(factory, t);

    let left = Panel::new(left_content)
        .rounded()
        .flex(1.0)
        .fg(demo_border_fg(t, t.primary()))
        .bg(panel_bg)
        .title(" Sidebar ")
        .build(factory, t);

    let center = FuzzyPanel::new([
        "src/bin/layout_demo2.rs",
        "src/panel/builder.rs",
        "src/fuzzy_panel/widget.rs",
        "src/prompt_bar/builder.rs",
        "src/scroll_column/builder.rs",
        "tests/composites.rs",
        "Cargo.toml",
        "docs/TEPs/TEP-0005-components.md",
        "README.md",
    ])
    .rounded()
    .flex(1.0)
    .fg(demo_border_fg(t, t.accent()))
    .bg(panel_bg)
    .accent(t.primary())
    .title(" Fuzzy Files ")
    .placeholder("type to filter files")
    .empty_text("No files match")
    .build(factory, t);

    let right = Panel::new(
        RichText::new()
            .bg(panel_cell)
            .line(vec![Span::new(
                format!(" {cols}x{rows}"),
                t.text(),
                panel_bg,
                Attrs::default(),
            )])
            .line(vec![])
            .line(vec![Span::new(
                " accent",
                t.accent(),
                panel_bg,
                Default::default(),
            )])
            .line(vec![Span::new(
                " primary",
                t.primary(),
                panel_bg,
                Default::default(),
            )])
            .line(vec![Span::new(
                " success",
                t.success(),
                panel_bg,
                Default::default(),
            )])
            .line(vec![Span::new(
                " danger",
                t.danger(),
                panel_bg,
                Default::default(),
            )])
            .line(vec![Span::new(
                " warning",
                t.warning(),
                panel_bg,
                Default::default(),
            )])
            .build(factory, t),
    )
    .rounded()
    .flex(1.0)
    .fg(demo_border_fg(t, t.success()))
    .bg(panel_bg)
    .title(" Info ")
    .build(factory, t);

    let body = Row::new()
        .flex(1.0)
        .children([
            Col::new().width(left_w).children([left]).build(factory, t),
            Col::new().flex(1.0).children([center]).build(factory, t),
            Col::new()
                .width(right_w)
                .children([right])
                .build(factory, t),
        ])
        .build(factory, t);

    Col::new()
        .size(cols, rows)
        .padding(RectOffset {
            top: 0,
            bottom: 0,
            left: 1,
            right: 1,
        })
        .children([header, body, footer])
        .build(factory, t)
}

fn demo_panel_bg(t: &Theme) -> AnsiColor {
    match t.variant {
        ThemeVariant::Light => t.surface_alt(),
        ThemeVariant::Dark | ThemeVariant::HighContrast => t.surface(),
    }
}

fn demo_border_fg(t: &Theme, fallback: AnsiColor) -> AnsiColor {
    match t.variant {
        ThemeVariant::Light => t.border(),
        ThemeVariant::Dark | ThemeVariant::HighContrast => fallback,
    }
}

fn demo_panel_cell(t: &Theme) -> Cell {
    Cell {
        bg: demo_panel_bg(t),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_testing::WidgetHarness;

    #[test]
    fn light_theme_demo_uses_panel_background_for_content() {
        let cols = 80;
        let rows = 18;
        let factory = WidgetFactory::new();
        let theme = Theme::light();
        let theme_changed = Rc::new(StdCell::new(false));
        let theme_rc = Rc::new(RefCell::new(theme.clone()));
        let root = build_ui(&factory, &theme, cols, rows, &theme_changed, &theme_rc);
        let harness = WidgetHarness::render(&root, cols, rows, &theme);
        let panel_bg = demo_panel_bg(&theme);

        harness.assert_no_black_bg_on_text().unwrap();
        for text in [
            "Arbor TUI".to_string(),
            "Home".to_string(),
            "╭----╯ Menu".to_string(),
            "╭----╯ Status".to_string(),
            "Fuzzy Files".to_string(),
            format!("{cols}x{rows}"),
        ] {
            let (col, row) = harness
                .find_text(&text)
                .first()
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "expected demo screen to contain {text:?}\n{}",
                        screen_text(&harness)
                    )
                });
            assert_eq!(
                harness.cell_at(col, row).bg,
                panel_bg,
                "{text:?} should use the light panel background"
            );
        }

        let (selected_col, selected_row) = harness.find_text("src/bin/layout_demo2.rs")[0];
        assert_eq!(
            harness.cell_at(selected_col, selected_row).bg,
            theme.primary(),
            "selected fuzzy panel row should use the light primary background"
        );

        let (title_col, title_row) = harness.find_text("Arbor TUI")[0];
        assert_eq!(harness.cell_at(title_col, title_row).fg, theme.border());

        let (input_col, input_row) = harness.find_text("type /theme dark")[0];
        assert_eq!(
            harness.cell_at(input_col, input_row).bg,
            theme.surface_alt()
        );
    }

    fn screen_text(harness: &WidgetHarness) -> String {
        let mut text = String::new();
        for row in 0..harness.rows() {
            for col in 0..harness.cols() {
                text.push(harness.cell_at(col, row).ch);
            }
            if row + 1 < harness.rows() {
                text.push('\n');
            }
        }
        text
    }
}
