// Layout demo — header / 3-column body / footer, all with rounded borders.
// Type "/theme dark" or "/theme light" in the footer input and press Enter.
// PgUp/PgDn 切换中间栏内容，^C/q 退出。

use std::cell::{Cell, RefCell};
use std::io::stdout;
use std::rc::Rc;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_primitives::cell::{Attrs, Span};
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_primitives::layout::RectOffset;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::WidgetNode;

use arbor_tui::app::{App, AppConfig};
use arbor_tui::event_loop::default_keymap;
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn main() {
    if let Err(e) = run() {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[layout_demo] {e:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut backend = CrosstermBackend::new();
    execute!(stdout(), EnterAlternateScreen)?;
    backend.hide_cursor()?;
    backend.clear()?;

    let _guard = backend.enter_raw_mode()?;
    let input = StdinReader::new();
    let theme = Rc::new(RefCell::new(Theme::dark()));
    let theme_changed = Rc::new(Cell::new(false));
    let (mut cols, mut rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());
    let wm = WidgetFactory::new();
    let mut root = build_ui(&wm, &theme.borrow(), cols, rows, &theme_changed, &theme);
    let mut needs_rebuild = true;
    let mut first = true;

    loop {
        let (new_cols, new_rows) = backend.size()?;
        if (new_cols != cols || new_rows != rows) && app.check_resize(new_cols, new_rows, 50) {
            cols = new_cols;
            rows = new_rows;
            backend.clear()?;
            root = build_ui(&wm, &theme.borrow(), cols, rows, &theme_changed, &theme);
            needs_rebuild = true;
        }

        if theme_changed.get() {
            theme_changed.set(false);
            root = build_ui(&wm, &theme.borrow(), cols, rows, &theme_changed, &theme);
            needs_rebuild = true;
        }

        if needs_rebuild || !app.dirty_tracker.is_empty() {
            if let Err(e) = app.render_widget_tree(&root, &theme.borrow(), &mut backend) {
                eprintln!("[layout_demo] render: {e:?}");
                break;
            }
            needs_rebuild = false;
            if first {
                let _ = app.focus_next();
                first = false;
            }
        }

        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) {
                should_quit = true;
            }
            match &event.key {
                Key::Char('q') | Key::Escape => should_quit = true,
                Key::Tab if event.modifiers.shift => {
                    let _ = app.focus_prev();
                }
                Key::Tab => {
                    let _ = app.focus_next();
                }
                _ => {
                    if let Some(a) = default_keymap(event) {
                        app.dispatch_action(&mut root, &a);
                    }
                }
            }
        }
        if should_quit {
            break;
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn build_ui(
    wm: &WidgetFactory,
    t: &Theme,
    cols: u16,
    rows: u16,
    theme_changed: &Rc<Cell<bool>>,
    theme_rc: &Rc<RefCell<Theme>>,
) -> WidgetNode {
    let left_w = cols / 5;
    let right_w = cols / 4;

    // ── Header ─────────────────────────────────────────────────
    let theme_name = match t.variant {
        arbor_tui_render::theme::ThemeVariant::Dark => "dark",
        arbor_tui_render::theme::ThemeVariant::Light => "light",
        arbor_tui_render::theme::ThemeVariant::HighContrast => "hc",
    };
    let header = Border::new()
        .rounded()
        .fg(t.accent())
        .title(" Arbor TUI ")
        .child(
            Text::new(format!("Theme: {theme_name}  |  ^C/q to quit"))
                .fg(t.text_dim())
                .build(wm, t),
        )
        .build(wm, t);

    // ── Footer with theme-switching input ──────────────────────
    let t_clone = theme_rc.clone();
    let changed = theme_changed.clone();
    let footer = Border::new()
        .rounded()
        .fg(t.accent())
        .title(" Commands ")
        .child(
            Input::new()
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
                .build(wm, t),
        )
        .build(wm, t);

    // ── Body: 3 columns ────────────────────────────────────────
    let left = Border::new()
        .rounded()
        .fg(t.primary())
        .title(" Nav ")
        .child(
            RichText::new()
                .line(vec![Span::new(
                    "  Home",
                    t.text(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    "  Projects",
                    t.text(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    "  Settings",
                    t.text(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![])
                .line(vec![Span::new(
                    " Status:",
                    t.text_dim(),
                    t.surface(),
                    Attrs {
                        italic: true,
                        ..Default::default()
                    },
                )])
                .line(vec![Span::new(
                    "  CPU  12%",
                    t.text_dim(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    "  RAM  3.2G",
                    t.text_dim(),
                    t.surface(),
                    Default::default(),
                )])
                .build(wm, t),
        )
        .build(wm, t);

    let center = Border::new()
        .rounded()
        .flex(1.0)
        .fg(t.accent())
        .title(" Content ")
        .child(
            RichText::new()
                .padding(RectOffset::all(1))
                .line(vec![Span::new(
                    "═══ Welcome ═══",
                    t.primary(),
                    t.surface(),
                    Attrs {
                        bold: true,
                        ..Default::default()
                    },
                )])
                .line(vec![])
                .line(vec![Span::new(
                    "3-column layout with rounded borders.",
                    t.text(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![])
                .line(vec![Span::new(
                    format!("Left: {left_w} cols  |  Right: {right_w} cols  |  Center: flex"),
                    t.text_dim(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![])
                .line(vec![
                    Span::new(
                        "Type /theme dark or /theme light",
                        t.success(),
                        t.surface(),
                        Attrs {
                            italic: true,
                            ..Default::default()
                        },
                    ),
                    Span::new(
                        " in the footer and press Enter.",
                        t.text_dim(),
                        t.surface(),
                        Attrs {
                            italic: true,
                            ..Default::default()
                        },
                    ),
                ])
                .build(wm, t),
        )
        .build(wm, t);

    let right = Border::new()
        .rounded()
        .fg(t.success())
        .title(" Info ")
        .child(
            RichText::new()
                .line(vec![Span::new(
                    format!(" {cols}x{rows}"),
                    t.text(),
                    t.surface(),
                    Attrs::default(),
                )])
                .line(vec![])
                .line(vec![Span::new(
                    " accent",
                    t.accent(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    " primary",
                    t.primary(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    " success",
                    t.success(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    " danger",
                    t.danger(),
                    t.surface(),
                    Default::default(),
                )])
                .line(vec![Span::new(
                    " warning",
                    t.warning(),
                    t.surface(),
                    Default::default(),
                )])
                .build(wm, t),
        )
        .build(wm, t);

    let body = Row::new()
        .flex(1.0)
        .children([
            left,
            Col::new().flex(1.0).children([center]).build(wm, t),
            right,
        ])
        .build(wm, t);

    Col::new()
        .size(cols, rows)
        .padding(RectOffset {
            top: 0,
            bottom: 0,
            left: 1,
            right: 1,
        })
        .children([header, body, footer])
        .build(wm, t)
}
