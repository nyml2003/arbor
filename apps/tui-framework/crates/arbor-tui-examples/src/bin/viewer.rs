// File viewer demo — 只读文件浏览器。
// ↑↓/jk:滚动  PgUp/PgDn:翻页  Home/End:首尾  ^C/q:退出

use std::env;
use std::fs;
use std::io::stdout;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_primitives::cell::Attrs;
use arbor_tui_primitives::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_primitives::text::{self, TruncateStrategy, WrapStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::WidgetNode;

use arbor_tui::app::{App, AppConfig};
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_widgets::box_widget::BoxWidget;
use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[viewer] fatal error: {e:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        anyhow::bail!("用法: cargo run --example viewer -p arbor-tui -- <文件路径>");
    }
    let path = &args[1];
    let raw_content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("无法读取文件 {path}: {e}"))?;
    let content = text::expand_tabs(&raw_content);

    let mut backend = CrosstermBackend::new();
    execute!(stdout(), EnterAlternateScreen)?;
    backend.hide_cursor()?;
    backend.clear()?;

    let _guard = backend.enter_raw_mode()?;
    let input = StdinReader::new();
    let theme = Theme::dark();
    let (mut cols, mut rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());

    let raw_content = content;
    let mut content_w = cols.saturating_sub(6);
    let mut all_lines = wrap_content(&raw_content, content_w);
    let mut max_scroll = all_lines.len();
    let mut scroll: usize = 0;

    loop {
        let mut body_rows = rows.saturating_sub(2);

        // Debounced resize: only apply after terminal size has been stable for 50ms.
        // When resize is applied, immediately render (skip input poll) to fill the
        // blank canvas — avoids showing stale content between resize and next poll.
        let (new_cols, new_rows) = backend.size()?;
        if new_cols != cols || new_rows != rows {
            if app.check_resize(new_cols, new_rows, 50) {
                cols = new_cols; rows = new_rows;
                content_w = cols.saturating_sub(6);
                all_lines = wrap_content(&raw_content, content_w);
                max_scroll = all_lines.len();
                scroll = scroll.min(max_scroll);
                body_rows = rows.saturating_sub(2);
                // Clear terminal first — app.apply_resize sets blank VirtualScreen
                // but the terminal still has old content. Without clearing, diff
                // only emits text cells and stale background cells remain visible.
                backend.clear()?;
                let root = build_ui(&theme, path, &all_lines, scroll, body_rows, cols, rows);
                app.render_widget_tree(&root, &theme, &mut backend)?;
                continue;
            }
        }

        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) { should_quit = true; }
            match &event.key {
                Key::Char('q') | Key::Escape => should_quit = true,
                Key::Char('j') | Key::ArrowDown => scroll = (scroll + 1).min(max_scroll),
                Key::Char('k') | Key::ArrowUp => scroll = scroll.saturating_sub(1),
                Key::PageDown => scroll = (scroll + body_rows as usize).min(max_scroll),
                Key::PageUp => scroll = scroll.saturating_sub(body_rows as usize),
                Key::Char('g') | Key::Home => scroll = 0,
                Key::Char('G') | Key::End => scroll = max_scroll,
                _ => {}
            }
        }
        if should_quit { break; }

        let root = build_ui(&theme, path, &all_lines, scroll, body_rows, cols, rows);
        match app.render_widget_tree(&root, &theme, &mut backend) {
            Ok(_) => {}
            Err(e) => { eprintln!("[viewer] render error: {e:?}"); break; }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn wrap_content(raw: &str, width: u16) -> Vec<String> {
    raw.lines()
        .flat_map(|line| if line.is_empty() { vec![String::new()] } else { text::wrap_lines(line, width, WrapStrategy::Char) })
        .collect()
}

fn build_ui(theme: &Theme, path: &str, all_lines: &[String], scroll: usize, body_rows: u16, cols: u16, rows: u16) -> WidgetNode {
    use arbor_tui_widget::widget::WidgetId;

    let title = format!(" {} ({} 行, {} 列) ", path, all_lines.len(), cols);
    let header_style = TextStyle { fg: theme.surface(), bg: theme.accent(), attrs: Attrs { bold: true, ..Default::default() } };

    let end = (scroll + body_rows as usize).min(all_lines.len());
    let body_text: String = (scroll..end)
        .map(|i| format!("{:>5} {}", i + 1, all_lines[i]))
        .collect::<Vec<_>>().join("\n");

    let visible_first = (scroll + 1).min(all_lines.len());
    let pct = if all_lines.is_empty() { 0 } else { visible_first * 100 / all_lines.len() };
    let status = format!(" {}%  L{}/{}  ↑↓/jk  PgUp/PgDn  Home/End  ^C/q ", pct, visible_first, all_lines.len());
    let footer_style = TextStyle { fg: theme.accent(), bg: theme.surface(), attrs: Attrs { bold: true, ..Default::default() } };

    WidgetNode::new(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps { direction: Direction::Column, width: Some(cols), height: Some(rows), ..Default::default() },
        children: vec![
            WidgetNode::new(TextWidget { id: WidgetId(1), props: LayoutProps::default(), text: ReadSignal::constant(title), style: ReadSignal::constant(header_style), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(2), props: LayoutProps { flex: 1.0, padding: RectOffset { left: 1, right: 1, top: 0, bottom: 0 }, ..Default::default() }, text: ReadSignal::constant(body_text), style: ReadSignal::constant(TextStyle::default()), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
            WidgetNode::new(TextWidget { id: WidgetId(3), props: LayoutProps::default(), text: ReadSignal::constant(status), style: ReadSignal::constant(footer_style), wrap: WrapStrategy::None, truncate: TruncateStrategy::End }),
        ],
    })
}
