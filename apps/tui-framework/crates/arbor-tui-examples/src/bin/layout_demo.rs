// Layout demo — 组件树 + 布局引擎 + 渲染管线的完整验证。
// Box(Column) 里放标题 Text + 计数器 Text + 帮助 Text。
// j/k 增减计数，q 退出。

use std::io::stdout;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_core::cell::Attrs;
use arbor_tui_core::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_core::signal::ReadSignal;
use arbor_tui_core::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::{
    BoxWidget, ButtonStyle, ButtonWidget, TextWidget, TextStyle, WidgetId, WidgetNode,
};

use arbor_tui::app::{App, AppConfig};
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_core::backend::TerminalBackend;
use arbor_tui_core::input::{InputReader, Key};
use std::time::Duration;

fn main() {
    // ── 终端初始化 ──
    let mut backend = CrosstermBackend::new();
    let _ = execute!(stdout(), EnterAlternateScreen);
    backend.hide_cursor();
    backend.clear();

    let _guard = backend.enter_raw_mode();
    let input = StdinReader::new();

    // ── 状态 ──
    let theme = Theme::dark();
    let (cols, rows) = backend.size();
    let mut app = App::new(cols, rows, AppConfig::default());
    let mut counter: i32 = 0;

    // ── 主循环 ──
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
        app.render_widget_tree(&root, &theme, &mut backend);

    }

    let _ = execute!(stdout(), LeaveAlternateScreen);
}

fn build_ui(theme: &Theme, count: i32, _cols: u16, _rows: u16) -> WidgetNode {
    WidgetNode::Box(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps {
            direction: Direction::Column,
            padding: RectOffset::all(1),
            ..Default::default()
        },
        children: vec![
            // 标题
            WidgetNode::Text(TextWidget {
                id: WidgetId(1),
                props: LayoutProps {
                    padding: RectOffset{ top: 0, bottom: 1, left: 0, right: 0 },
                    ..Default::default()
                },
                text: ReadSignal::constant("Arbor TUI — Layout Demo".to_string()),
                style: ReadSignal::constant(TextStyle {
                    fg: theme.accent(),
                    bg: theme.surface(),
                    attrs: Attrs { bold: true, ..Default::default() },
                }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            // 计数器值
            WidgetNode::Text(TextWidget {
                id: WidgetId(2),
                props: LayoutProps {
                    padding: RectOffset{ top: 1, bottom: 0, left: 2, right: 0 },
                    ..Default::default()
                },
                text: ReadSignal::constant(format!("Count: {}", count)),
                style: ReadSignal::constant(TextStyle {
                    fg: theme.text(),
                    bg: theme.surface(),
                    attrs: Attrs::default(),
                }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            // 按钮（纯展示）
            WidgetNode::Button(ButtonWidget {
                id: WidgetId(3),
                props: LayoutProps {
                    padding: RectOffset{ top: 1, bottom: 1, left: 0, right: 0 },
                    width: Some(20),
                    ..Default::default()
                },
                label: ReadSignal::constant("Increment (j)".to_string()),
                style: ButtonStyle::Primary,
                on_click: None,
            }),
            // 帮助
            WidgetNode::Text(TextWidget {
                id: WidgetId(4),
                props: LayoutProps {
                    padding: RectOffset{ top: 1, left: 0, right: 0, bottom: 0 },
                    ..Default::default()
                },
                text: ReadSignal::constant("j/k: +/-  |  ^C/q: quit".to_string()),
                style: ReadSignal::constant(TextStyle {
                    fg: theme.text_dim(),
                    bg: theme.surface(),
                    attrs: Attrs::default(),
                }),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    })
}
