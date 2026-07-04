// File viewer demo — 只读文件浏览器，使用组件树+布局引擎。
// 用法: cargo run --example viewer -p arbor-tui -- <文件路径>
// ↑↓/jk:滚动  PgUp/PgDn:翻页  Home/End:首尾  ^C/q:退出

use std::env;
use std::fs;
use std::io::stdout;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_core::cell::Attrs;
use arbor_tui_core::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_core::signal::ReadSignal;
use arbor_tui_core::text::{self, TruncateStrategy, WrapStrategy};
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::{
    BoxWidget, TextWidget, TextStyle, WidgetId, WidgetNode,
};

use arbor_tui::app::{App, AppConfig};
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_core::backend::TerminalBackend;
use arbor_tui_core::input::{InputReader, Key};
use std::time::Duration;

fn main() {
    // ── 读取文件 ──
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: cargo run --example viewer -p arbor-tui -- <文件路径>");
        std::process::exit(1);
    }
    let path = &args[1];
    let raw_content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("无法读取文件 {}: {}", path, e);
            std::process::exit(1);
        }
    };
    let content = text::expand_tabs(&raw_content);

    // ── 终端初始化 ──
    let mut backend = CrosstermBackend::new();
    let _ = execute!(stdout(), EnterAlternateScreen);
    backend.hide_cursor();
    backend.clear();

    let _guard = backend.enter_raw_mode();
    let input = StdinReader::new();
    let theme = Theme::dark();
    let (mut cols, mut rows) = backend.size();
    let mut app = App::new(cols, rows, AppConfig::default());

    // ── 状态 ──
    let raw_content = content; // keep original for re-wrap on resize
    let mut content_w = cols.saturating_sub(6);
    let mut all_lines = wrap_content(&raw_content, content_w);
    let mut max_scroll = all_lines.len(); // 允许最后一行滚到顶部
    let mut scroll: usize = 0;

    // ── 主循环 ──
    loop {
        // 检测终端尺寸变化
        let (new_cols, new_rows) = backend.size();
        if new_cols != cols || new_rows != rows {
            cols = new_cols;
            rows = new_rows;
            app.resize(cols, rows);
            content_w = cols.saturating_sub(6);
            all_lines = wrap_content(&raw_content, content_w);
            max_scroll = all_lines.len();
            scroll = scroll.min(max_scroll);
        }
        let body_rows = rows.saturating_sub(2);

        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) {
                should_quit = true;
            }
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

        // ── 构建组件树 ──
        let root = build_ui(&theme, path, &all_lines, scroll, body_rows, cols, rows);

        // ── 管线 ──
        app.render_widget_tree(&root, &theme, &mut backend);

    }

    let _ = execute!(stdout(), LeaveAlternateScreen);
}

fn wrap_content(raw: &str, width: u16) -> Vec<String> {
    raw.lines()
        .flat_map(|line| {
            if line.is_empty() {
                vec![String::new()]
            } else {
                text::wrap_lines(line, width, text::WrapStrategy::Char)
            }
        })
        .collect()
}

fn build_ui(
    theme: &Theme,
    path: &str,
    all_lines: &[String],
    scroll: usize,
    body_rows: u16,
    cols: u16,
    rows: u16,
) -> WidgetNode {
    let title = format!(" {} ({} 行, {} 列) ", path, all_lines.len(), cols);
    let header_style = TextStyle {
        fg: theme.surface(),
        bg: theme.accent(),
        attrs: Attrs { bold: true, ..Default::default() },
    };

    // 可见行：拼接行号 + 内容
    let end = (scroll + body_rows as usize).min(all_lines.len());
    let body_text: String = (scroll..end)
        .map(|i| {
            let num = format!("{:>5} ", i + 1);
            let line = &all_lines[i];
            format!("{}{}", num, line)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let visible_first = (scroll + 1).min(all_lines.len());
    let pct = if all_lines.is_empty() { 0 } else { visible_first * 100 / all_lines.len() };
    let status = format!(
        " {}%  L{}/{}  ↑↓/jk  PgUp/PgDn  Home/End  ^C/q ",
        pct, visible_first, all_lines.len()
    );
    let footer_style = TextStyle {
        fg: theme.accent(),
        bg: theme.surface(),
        attrs: Attrs { bold: true, ..Default::default() },
    };

    WidgetNode::Box(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps {
            direction: Direction::Column,
            width: Some(cols),
            height: Some(rows),
            ..Default::default()
        },
        children: vec![
            // 标题栏
            WidgetNode::Text(TextWidget {
                id: WidgetId(1),
                props: LayoutProps::default(),
                text: ReadSignal::constant(title),
                style: ReadSignal::constant(header_style),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            // 文件内容（弹性填充）
            WidgetNode::Text(TextWidget {
                id: WidgetId(2),
                props: LayoutProps {
                    flex: 1.0,
                    padding: RectOffset { left: 1, right: 1, top: 0, bottom: 0 },
                    ..Default::default()
                },
                text: ReadSignal::constant(body_text),
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None, // 行已预先 wrap 好
                truncate: TruncateStrategy::End,
            }),
            // 状态栏
            WidgetNode::Text(TextWidget {
                id: WidgetId(3),
                props: LayoutProps::default(),
                text: ReadSignal::constant(status),
                style: ReadSignal::constant(footer_style),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    })
}
