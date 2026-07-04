// Arbor TUI Showcase — Builder DSL, zero manual WidgetId.
// PgUp/PgDn:tab  Tab:focus  ^C/q:quit

use std::io::stdout;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_primitives::cell::{AnsiColor, Attrs, Span};
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_primitives::layout::RectOffset;
use arbor_tui_primitives::text::{self, WrapStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_widget::widget::WidgetNode;

use arbor_tui::app::{App, AppConfig};
use arbor_tui::event_loop::default_keymap;
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_widgets::container::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::button::Button;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::tabs::Tabs;
use arbor_tui_widgets::list::List;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::widget_manager::WidgetManager;
use arbor_tui_widgets::{ButtonStyle, TabDef};

fn main() {
    if let Err(e) = run() {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[showcase] fatal error: {e:?}");
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
    let theme = Theme::dark();
    let (mut cols, mut rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());
    let mut active_tab: usize = 0;
    let wm = WidgetManager::new();
    let mut root = build_ui(&wm, &theme, active_tab, cols, rows);
    let mut needs_rebuild = true;

    loop {
        let (new_cols, new_rows) = backend.size()?;
        if new_cols != cols || new_rows != rows {
            if app.check_resize(new_cols, new_rows, 50) {
                cols = new_cols; rows = new_rows;
                backend.clear()?;
                root = build_ui(&wm, &theme, active_tab, cols, rows);
                needs_rebuild = true;
            }
        }

        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) { should_quit = true; }
            match &event.key {
                Key::Char('q') | Key::Escape => should_quit = true,
                Key::PageDown => { active_tab = (active_tab + 1) % 6; root = build_ui(&wm, &theme, active_tab, cols, rows); app.focus_manager.rebuild(&root); let _ = app.focus_next(); let _ = app.focus_next(); needs_rebuild = true; }
                Key::PageUp => { active_tab = if active_tab == 0 { 5 } else { active_tab - 1 }; root = build_ui(&wm, &theme, active_tab, cols, rows); app.focus_manager.rebuild(&root); let _ = app.focus_next(); let _ = app.focus_next(); needs_rebuild = true; }
                Key::Tab if event.modifiers.shift => { let _ = app.focus_prev(); }
                Key::Tab => { let _ = app.focus_next(); }
                _ => { if let Some(a) = default_keymap(event) { app.dispatch_action(&mut root, &a); } }
            }
        }
        if should_quit { break; }

        if needs_rebuild || !app.dirty_tracker.is_empty() {
            if let Err(e) = app.render_widget_tree(&root, &theme, &mut backend) {
                eprintln!("[showcase] render: {e:?}"); break;
            }
            needs_rebuild = false;
        }
    }
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

// ── Build UI ───────────────────────────────────────────────────────

fn build_ui(wm: &WidgetManager, t: &Theme, active_tab: usize, cols: u16, rows: u16) -> WidgetNode {
    let bold = Attrs { bold: true, ..Default::default() };

    Col::new()
        .size(cols, rows)
        .children([
            RichText::new()
                .line(vec![
                    Span::new(" Arbor TUI ", t.surface(), t.accent(), bold),
                    Span::new(" Showcase ", t.surface(), t.primary(), bold),
                    Span::new(" PgUp/PgDn:tab  q:quit ", t.text_dim(), t.surface(), Attrs::default()),
                ])
                .build(wm, t),

            Tabs::new(active_tab)
                .flex(1.0)
                .tabs(vec![
                    TabDef { label: "RichText".into(), content: rich_text_tab(wm, t) },
                    TabDef { label: "Columns".into(), content: columns_tab(wm, t, cols.into(), rows.into()) },
                    TabDef { label: "Layout".into(), content: layout_tab(wm, t) },
                    TabDef { label: "Input".into(), content: input_tab(wm, t) },
                    TabDef { label: "List".into(), content: list_tab(wm, t) },
                    TabDef { label: "Scroll".into(), content: scroll_tab(wm, t, cols.into(), rows.into()) },
                ])
                .build(wm, t),

            Text::new(format!(" PgUp/PgDn:tab  Tab:focus  ^C/q:quit  {cols}x{rows} "))
                .fg(t.accent()).bold()
                .build(wm, t),
        ])
        .build(wm, t)
}

// ── Tab: Rich Text ─────────────────────────────────────────────────

fn rich_text_tab(wm: &WidgetManager, t: &Theme) -> WidgetNode {
    let c = |i| AnsiColor::from_palette(i);
    RichText::new().padding(RectOffset::all(1))
        .line(vec![Span::new("═══ Rich Text Demo ═══", t.accent(), t.surface(), Attrs { bold: true, ..Default::default() })])
        .line(vec![])
        .line(vec![Span::new("256-color: ", t.text(), t.surface(), Default::default()), Span::new(" RED ", t.surface(), c(167), Default::default()), Span::new(" GREEN ", t.surface(), c(71), Default::default()), Span::new(" BLUE ", t.surface(), c(68), Default::default()), Span::new(" GOLD ", t.surface(), c(179), Default::default())])
        .line(vec![])
        .line(vec![Span::new("TrueColor: ", t.text(), t.surface(), Default::default()), Span::new(" orange ", AnsiColor::from_rgb(255,128,0), c(0), Default::default()), Span::new(" pink ", AnsiColor::from_rgb(255,105,180), c(0), Default::default()), Span::new(" teal ", AnsiColor::from_rgb(0,200,180), c(0), Default::default())])
        .line(vec![])
        .line(vec![Span::new("Attr: ", t.text(), t.surface(), Default::default()), Span::new("bold", t.text(), t.surface(), Attrs { bold: true, ..Default::default() }), Span::new("  italic", t.text(), t.surface(), Attrs { italic: true, ..Default::default() }), Span::new("  underline", t.text(), t.surface(), Attrs { underline: true, ..Default::default() }), Span::new("  dim", t.text_dim(), t.surface(), Attrs { dim: true, ..Default::default() })])
        .line(vec![])
        .line(vec![Span::new("Theme: ", t.text(), t.surface(), Default::default()), Span::new(" accent ", t.surface(), t.accent(), Default::default()), Span::new(" primary ", t.surface(), t.primary(), Default::default()), Span::new(" danger ", t.surface(), t.danger(), Default::default()), Span::new(" success ", t.surface(), t.success(), Default::default()), Span::new(" warning ", t.surface(), t.warning(), Default::default())])
        .line(vec![])
        .line(vec![Span::new("Mix: ", t.text(), t.surface(), Default::default()), Span::new("BOLD RED", t.danger(), t.surface(), Attrs { bold: true, ..Default::default() }), Span::new("  italic blue", t.primary(), t.surface(), Attrs { italic: true, ..Default::default() }), Span::new("  underline green", t.success(), t.surface(), Attrs { underline: true, ..Default::default() })])
        .build(wm, t)
}

// ── Tab: Three-Column ──────────────────────────────────────────────

fn columns_tab(wm: &WidgetManager, t: &Theme, cols: u16, _rows: u16) -> WidgetNode {
    let left_w = (cols / 5).max(14);
    let right_w = (cols / 4).max(16);

    fn sidebar(wm: &WidgetManager, title: &str, title_fg: AnsiColor, items: &[&str], width: u16, t: &Theme) -> WidgetNode {
        let mut r = RichText::new();
        r = r.line(vec![Span::new(title, title_fg, t.surface(), Attrs { bold: true, ..Default::default() })]);
        r = r.line(vec![]);
        for item in items { r = r.line(vec![Span::new(format!("  {item}"), t.text(), t.surface(), Default::default())]); }
        Col::new().width(width).padding(RectOffset { top: 1, left: 2, right: 0, bottom: 1 }).children([r.build(wm, t)]).build(wm, t)
    }

    Row::new().children([
        sidebar(wm, "\u{25B6} NAV", t.accent(), &["Home", "Projects", "About"], left_w, t),
        Col::new().flex(1.0).children([
            RichText::new().line(vec![
                Span::new("Row + flex. ", t.primary(), t.surface(), Attrs { bold: true, ..Default::default() }),
                Span::new("Left ", t.accent(), t.surface(), Default::default()), Span::new(format!("{left_w} cols  "), t.text_dim(), t.surface(), Default::default()),
                Span::new("Center flex  ", t.text(), t.surface(), Default::default()),
                Span::new("Right ", t.accent(), t.surface(), Default::default()), Span::new(format!("{right_w} cols"), t.text_dim(), t.surface(), Default::default()),
            ]).build(wm, t),
            RichText::new().padding(RectOffset { top: 1, left: 1, ..Default::default() }).flex(1.0)
                .line(vec![Span::new("Patterns:", t.text_dim(), t.surface(), Attrs { italic: true, ..Default::default() })])
                .line(vec![Span::new("  Explorer", t.text(), t.surface(), Default::default()), Span::new("    tree | editor | preview", t.text_dim(), t.surface(), Default::default())])
                .line(vec![Span::new("  Email", t.text(), t.surface(), Default::default()), Span::new("       folders | list | msg", t.text_dim(), t.surface(), Default::default())])
                .line(vec![Span::new("  Dashboard", t.text(), t.surface(), Default::default()), Span::new("   nav | stats | alerts", t.text_dim(), t.surface(), Default::default())])
                .build(wm, t),
        ]).build(wm, t),
        sidebar(wm, "\u{25C9} STATUS", t.success(), &["CPU  12%", "RAM  3.2G", "Net  \u{2191}2M"], right_w, t),
    ]).build(wm, t)
}

// ── Tab: Layout ────────────────────────────────────────────────────

fn layout_tab(wm: &WidgetManager, t: &Theme) -> WidgetNode {
    Border::new().rounded().fg(t.accent()).title(" Layout Demo ").padding(RectOffset::all(1)).child(
        Col::new().children([
            Row::new().children([
                Button::new("Primary").primary().build(wm, t), Col::new().build(wm, t),
                Button::new("Secondary").build(wm, t), Col::new().build(wm, t),
                Button::new("Danger").danger().build(wm, t),
            ]).build(wm, t),
            Text::new("Column: stacks vertically").build(wm, t),
            Text::new("Row: arranges horizontally").build(wm, t),
            Text::new("Flex: fills remaining space").build(wm, t),
            Text::new("Padding/Margin: spacing control").dim().build(wm, t),
            Col::new().flex(1.0).build(wm, t),
        ]).build(wm, t)
    ).build(wm, t)
}

// ── Tab: Input ─────────────────────────────────────────────────────

fn input_tab(wm: &WidgetManager, t: &Theme) -> WidgetNode {
    Border::new().rounded().fg(t.accent()).title(" Input & Focus ").padding(RectOffset::all(1)).child(
        Col::new().children([
            Text::new("Name:").build(wm, t),
            Input::new().placeholder("Enter your name...").build(wm, t),
            Text::new("Password:").padding(RectOffset { top: 1, ..Default::default() }).build(wm, t),
            Input::new().placeholder("secret").password().build(wm, t),
            Col::new().flex(1.0).build(wm, t),
            Text::new("Tab/Shift+Tab  navigate focus\nType to fill input fields\nEnter to submit  Esc to cancel\nArrow keys move cursor\nHome/End jump to start/end\nBackspace/Delete remove chars")
                .padding(RectOffset::all(1)).dim().build(wm, t),
        ]).build(wm, t)
    ).build(wm, t)
}

// ── Tab: List ──────────────────────────────────────────────────────

fn list_tab(wm: &WidgetManager, t: &Theme) -> WidgetNode {
    Col::new().padding(RectOffset::all(1)).children([
        Text::new("\u{25B6} Fruit Browser").fg(t.accent()).bold().build(wm, t),
        List::new().items(vec!["  Fruit".into(),"  ├─ Apple".into(),"  ├─ Banana".into(),"  ├─ Cherry".into(),"  ├─ Dragonfruit".into(),"  ├─ Elderberry".into(),"  ├─ Fig".into(),"  ├─ Grape".into(),"  ├─ Honeydew".into(),"  ├─ Imbe".into(),"  ├─ Jackfruit".into(),"  ├─ Kiwi".into(),"  ├─ Lemon".into(),"  ├─ Mango".into(),"  └─ Nectarine".into()]).build(wm, t),
    ]).build(wm, t)
}

// ── Tab: Scroll ────────────────────────────────────────────────────

fn scroll_tab(wm: &WidgetManager, t: &Theme, cols: u16, _rows: u16) -> WidgetNode {
    let w = cols.saturating_sub(6);
    let div = "\u{2500}".repeat(w as usize);
    let items: Vec<(&str, AnsiColor, Attrs)> = vec![
        ("═══ ScrollView Demo ═══", t.accent(), Attrs { bold: true, ..Default::default() }),
        ("", t.text(), Default::default()),
        ("ScrollView wraps a child widget and provides a", t.text(), Default::default()),
        ("scrollable viewport. Child renders at natural size;", t.text(), Default::default()),
        ("scroll offsets clip the visible portion.", t.text(), Default::default()),
        ("", t.text(), Default::default()),
        ("Child can be any widget: Text, RichText, Box, List...", t.text(), Default::default()),
        ("Scroll offsets are ReadSignal<u16>.", t.text(), Default::default()),
        ("", t.text(), Default::default()),
        (div.as_str(), t.text_dim(), Default::default()), ("", t.text(), Default::default()),
        ("Lorem ipsum dolor sit amet, consectetur", t.text_dim(), Default::default()),
        ("adipiscing elit. Sed do eiusmod tempor", t.text_dim(), Default::default()),
        ("incididunt ut labore et dolore magna aliqua.", t.text_dim(), Default::default()),
        ("Ut enim ad minim veniam, quis nostrud", t.text_dim(), Default::default()),
        ("exercitation ullamco laboris nisi ut aliquip", t.text_dim(), Default::default()),
        ("ex ea commodo consequat.", t.text_dim(), Default::default()),
        ("", t.text(), Default::default()),
        (div.as_str(), t.text_dim(), Default::default()), ("", t.text(), Default::default()),
        ("End of scrollable content.", t.success(), Attrs { italic: true, ..Default::default() }),
    ];
    let mut r = RichText::new();
    for (txt, fg, attrs) in &items {
        let lines = text::wrap_lines(txt, w, text::WrapStrategy::Word);
        for line in &lines { r = r.line(vec![Span::new(line.as_str(), *fg, t.surface(), *attrs)]); }
    }
    Scroll::new().flex(1.0).padding(RectOffset::all(1)).child(r.build(wm, t)).build(wm, t)
}
