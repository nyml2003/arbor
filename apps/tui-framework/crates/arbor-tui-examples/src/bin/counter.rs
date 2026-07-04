// Counter demo — 组件树 + 布局引擎 + 渲染管线。
// Box(Column) 里放标题 Text + 计数器值 Text + 进度条 Box + 帮助 Text。
// j/k 增减计数，^C/q 退出。

use std::io::stdout;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_core::cell::Attrs;
use arbor_tui_core::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_core::signal::ReadSignal;
use arbor_tui_core::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::{
    BoxWidget, TextWidget, TextStyle, WidgetId, WidgetNode,
};

use arbor_tui::app::{App, AppConfig};
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_core::input::{InputReader, Key};
use std::time::Duration;

fn main() {
    if let Err(e) = run() {
        // Best-effort terminal restoration on error
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[counter] fatal error: {e:?}");
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
    let (cols, rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());
    let mut counter: i32 = 0;

    loop {
        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) {
                should_quit = true;
            }
            match &event.key {
                Key::Char('q') | Key::Escape => should_quit = true,
                Key::Char('j') => counter += 1,
                Key::Char('k') => counter -= 1,
                _ => {}
            }
        }
        if should_quit { break; }

        let root = build_ui(&theme, counter, cols, rows);
        match app.render_widget_tree(&root, &theme, &mut backend) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("[counter] render error: {e:?}");
                break;
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn build_ui(theme: &Theme, count: i32, cols: u16, rows: u16) -> WidgetNode {
    let bar_w = ((count % 40 + 40) % 40) as u16 + 1;
    let bar_text = "█".repeat(bar_w as usize);

    WidgetNode::Box(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps {
            direction: Direction::Column,
            padding: RectOffset { top: 1, bottom: 1, left: 2, right: 2 },
            width: Some(cols),
            height: Some(rows),
            ..Default::default()
        },
        children: vec![
            WidgetNode::Text(TextWidget {
                id: WidgetId(1),
                props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() },
                text: ReadSignal::constant("Arbor TUI — Counter".to_string()),
                style: ReadSignal::constant(TextStyle {
                    fg: theme.accent(), bg: theme.surface(),
                    attrs: Attrs { bold: true, ..Default::default() },
                }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            WidgetNode::Text(TextWidget {
                id: WidgetId(2),
                props: LayoutProps { padding: RectOffset { left: 2, bottom: 1, ..Default::default() }, ..Default::default() },
                text: ReadSignal::constant(format!("Count: {}", count)),
                style: ReadSignal::constant(TextStyle::default()),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            WidgetNode::Text(TextWidget {
                id: WidgetId(3),
                props: LayoutProps { padding: RectOffset { bottom: 1, ..Default::default() }, ..Default::default() },
                text: ReadSignal::constant(bar_text),
                style: ReadSignal::constant(TextStyle {
                    fg: theme.primary(), bg: theme.surface(),
                    attrs: Attrs::default(),
                }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            WidgetNode::Box(BoxWidget {
                id: WidgetId(4),
                props: LayoutProps { flex: 1.0, ..Default::default() },
                children: vec![],
            }),
            WidgetNode::Text(TextWidget {
                id: WidgetId(5),
                props: LayoutProps::default(),
                text: ReadSignal::constant("j/k: +/-  |  ^C/q: quit".to_string()),
                style: ReadSignal::constant(TextStyle { fg: theme.text_dim(), bg: theme.surface(), attrs: Attrs::default() }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    })
}
