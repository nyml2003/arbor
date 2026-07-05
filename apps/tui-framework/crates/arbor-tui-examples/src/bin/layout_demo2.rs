// Layout demo — header / 3-column body / footer, all with rounded borders.
// Type "/theme dark" or "/theme light" in the footer input and press Enter.
// PgUp/PgDn 切换中间栏内容，^C/q 退出。

use std::cell::{Cell, RefCell};
use std::io::stdout;
use std::rc::Rc;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_domain::cell::{Attrs, Span};
use arbor_tui_domain::input::InputReader;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;

use arbor_tui_adapters::crossterm_backend::CrosstermBackend;
use arbor_tui_adapters::stdin_reader::StdinReader;
use arbor_tui_application::app::App;
use arbor_tui_application::runtime::{runtime_step, RuntimeInput};
use arbor_tui_application::TerminalBackend;
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
    let mut app = App::new(cols, rows);
    let factory = WidgetFactory::new();
    let mut root = build_ui(
        &factory,
        &theme.borrow(),
        cols,
        rows,
        &theme_changed,
        &theme,
    );
    let mut needs_rebuild = true;
    let mut first = true;
    app.run();

    while app.is_running() {
        let events = input.poll_timeout(Duration::from_millis(100));
        let runtime_input = if first {
            RuntimeInput::first_frame_with_events(events)
        } else {
            RuntimeInput::new(events)
        };
        let step = runtime_step(&mut app, &mut root, &backend, runtime_input)?;

        if step.resized {
            (cols, rows) = app.screen_size();
            root = build_ui(
                &factory,
                &theme.borrow(),
                cols,
                rows,
                &theme_changed,
                &theme,
            );
            needs_rebuild = true;
        }

        if step.should_clear {
            backend.clear()?;
        }

        if theme_changed.get() {
            theme_changed.set(false);
            root = build_ui(
                &factory,
                &theme.borrow(),
                cols,
                rows,
                &theme_changed,
                &theme,
            );
            needs_rebuild = true;
        }

        if needs_rebuild || step.should_render {
            if let Err(e) = app.render_widget_tree(&root, &theme.borrow(), &mut backend) {
                eprintln!("[layout_demo] render: {e:?}");
                break;
            }
            needs_rebuild = false;
            if first {
                let _ = app.focus_next();
            }
        }

        first = false;
        if step.should_quit {
            break;
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn build_ui(
    factory: &WidgetFactory,
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
        arbor_tui_domain::theme::ThemeVariant::Dark => "dark",
        arbor_tui_domain::theme::ThemeVariant::Light => "light",
        arbor_tui_domain::theme::ThemeVariant::HighContrast => "hc",
    };
    let header = Border::new()
        .rounded()
        .fg(t.accent())
        .title(" Arbor TUI ")
        .child(
            Text::new(format!("Theme: {theme_name}  |  ^C/q to quit"))
                .fg(t.text_dim())
                .build(factory, t),
        )
        .build(factory, t);

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
                .build(factory, t),
        )
        .build(factory, t);

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
                .build(factory, t),
        )
        .build(factory, t);

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
                .build(factory, t),
        )
        .build(factory, t);

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
                .build(factory, t),
        )
        .build(factory, t);

    let body = Row::new()
        .flex(1.0)
        .children([
            left,
            Col::new().flex(1.0).children([center]).build(factory, t),
            right,
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
