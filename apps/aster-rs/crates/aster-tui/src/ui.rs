use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_domain::cell::{Attrs, Cell, Span};
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;
use aster_application::ChatStreamPort;
use aster_domain::{ChatMessage, ChatRole, ConversationStatus};

use crate::state::AppState;

pub struct UiMetrics {
    pub fps: f64,
    pub last_frame_us: u64,
}

pub fn build_ui<C: ChatStreamPort + 'static>(
    factory: &WidgetFactory,
    theme: &Theme,
    state: &Rc<RefCell<AppState<C>>>,
    scroll_y: ReadSignal<u16>,
    cols: u16,
    rows: u16,
    metrics: UiMetrics,
) -> WidgetNode {
    let borrowed = state.borrow();
    let chat = borrowed.chat();
    let panel_bg = theme.surface();
    let title = title_for(chat.state());
    let msg_width = cols.saturating_sub(6) as usize;
    let (messages, line_count) =
        build_message_blocks(chat.messages(), chat.state(), theme, factory, msg_width);

    let state_for_submit = Rc::clone(state);
    let input = Border::new()
        .fg(theme.border())
        .bg(panel_bg)
        .child(
            Input::new()
                .placeholder("Type a message")
                .on_submit(move |message| {
                    state_for_submit.borrow_mut().submit_message(message);
                })
                .build(factory, theme),
        )
        .build(factory, theme);

    let message_stack = Col::new()
        .flex(1.0)
        .children(messages)
        .build(factory, theme);
    let message_scroll = Scroll::new()
        .flex(1.0)
        .scroll_y(scroll_y)
        .content_h(line_count.max(1) as u16)
        .child(message_stack)
        .build(factory, theme);

    let footer = Text::new(footer_text(line_count, metrics))
        .fg(theme.text_dim())
        .bg(panel_bg)
        .padding(RectOffset {
            left: 1,
            ..Default::default()
        })
        .build(factory, theme);

    let inner = Col::new()
        .flex(1.0)
        .children([message_scroll, input, footer])
        .build(factory, theme);

    let page = Border::new()
        .title(title)
        .rounded()
        .flex(1.0)
        .padding(RectOffset {
            top: 1,
            bottom: 1,
            left: 1,
            right: 1,
        })
        .fg(title_color(chat.state(), theme))
        .bg(panel_bg)
        .child(inner)
        .build(factory, theme);

    Col::new()
        .size(cols, rows)
        .children([page])
        .build(factory, theme)
}

fn title_for(state: &ConversationStatus) -> String {
    match state {
        ConversationStatus::Idle => " Aster - Chat ".to_string(),
        ConversationStatus::Streaming { token_count } => format!(" Aster - {token_count} tokens "),
        ConversationStatus::Error { .. } => " Aster - Error ".to_string(),
    }
}

fn title_color(state: &ConversationStatus, theme: &Theme) -> arbor_tui_domain::cell::AnsiColor {
    match state {
        ConversationStatus::Error { .. } => theme.danger(),
        ConversationStatus::Streaming { .. } => theme.primary(),
        ConversationStatus::Idle => theme.accent(),
    }
}

fn footer_text(line_count: usize, metrics: UiMetrics) -> String {
    let frame = if metrics.last_frame_us < 1000 {
        format!("{} us", metrics.last_frame_us)
    } else {
        format!("{:.1} ms", metrics.last_frame_us as f64 / 1000.0)
    };

    format!(
        "{line_count} lines | FPS: {fps:.0} frame: {frame} | Up/Down: scroll Enter: send Esc/Ctrl+C: quit",
        fps = metrics.fps,
    )
}

fn build_message_blocks(
    messages: &[ChatMessage],
    state: &ConversationStatus,
    theme: &Theme,
    factory: &WidgetFactory,
    width: usize,
) -> (Vec<WidgetNode>, usize) {
    let mut widgets = Vec::new();
    let mut total_lines = 0usize;

    if messages.is_empty() {
        widgets.push(
            RichText::new()
                .bg(surface_cell(theme))
                .lines(vec![vec![surface_span(
                    "  Welcome to Aster. Type a message and press Enter.",
                    theme.text(),
                    theme,
                    Attrs::default(),
                )]])
                .build(factory, theme),
        );
        return (widgets, 1);
    }

    for message in messages {
        push_message_label(
            &mut widgets,
            &mut total_lines,
            message.role(),
            theme,
            factory,
        );
        push_message_content(
            &mut widgets,
            &mut total_lines,
            message.content(),
            theme,
            factory,
            width,
        );

        widgets.push(
            RichText::new()
                .bg(surface_cell(theme))
                .lines(vec![vec![surface_span(
                    "",
                    theme.text(),
                    theme,
                    Attrs::default(),
                )]])
                .build(factory, theme),
        );
        total_lines += 1;
    }

    if let ConversationStatus::Error { message } = state {
        widgets.push(
            RichText::new()
                .bg(surface_cell(theme))
                .lines(vec![
                    vec![surface_span(
                        format!("  Error: {message}"),
                        theme.danger(),
                        theme,
                        Attrs::default(),
                    )],
                    vec![surface_span(
                        "  Press Esc after the stream stops, or submit another message.",
                        theme.text_dim(),
                        theme,
                        Attrs::default(),
                    )],
                ])
                .build(factory, theme),
        );
        total_lines += 2;
    }

    (widgets, total_lines)
}

fn push_message_label(
    widgets: &mut Vec<WidgetNode>,
    total_lines: &mut usize,
    role: &ChatRole,
    theme: &Theme,
    factory: &WidgetFactory,
) {
    let (label, color) = match role {
        ChatRole::User => ("You", theme.accent()),
        ChatRole::Assistant => ("Aster", theme.primary()),
        ChatRole::System => ("System", theme.warning()),
        ChatRole::Other(name) => (name.as_str(), theme.text()),
    };

    widgets.push(
        RichText::new()
            .bg(surface_cell(theme))
            .lines(vec![vec![
                surface_span("  ", theme.text(), theme, Attrs::default()),
                surface_span(
                    format!("{label}: "),
                    color,
                    theme,
                    Attrs {
                        bold: true,
                        ..Default::default()
                    },
                ),
            ]])
            .build(factory, theme),
    );
    *total_lines += 1;
}

fn push_message_content(
    widgets: &mut Vec<WidgetNode>,
    total_lines: &mut usize,
    content: &str,
    theme: &Theme,
    factory: &WidgetFactory,
    width: usize,
) {
    if content.is_empty() {
        widgets.push(
            RichText::new()
                .bg(surface_cell(theme))
                .lines(vec![vec![surface_span(
                    "",
                    theme.text(),
                    theme,
                    Attrs::default(),
                )]])
                .build(factory, theme),
        );
        *total_lines += 1;
        return;
    }

    for block in aster_markdown::parse_blocks(content, theme) {
        match block {
            aster_markdown::Block::Text(lines) => {
                let text_lines = indent_lines(lines, theme);
                *total_lines += text_lines.len();
                widgets.push(
                    RichText::new()
                        .bg(surface_cell(theme))
                        .lines(text_lines)
                        .build(factory, theme),
                );
            }
            aster_markdown::Block::Code { lang, lines } => {
                let code_height = lines.len();
                let title = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" {lang} ")
                };
                let code = RichText::new()
                    .bg(Cell {
                        bg: arbor_tui_domain::cell::AnsiColor::from_palette(236),
                        ..Default::default()
                    })
                    .lines(lines)
                    .padding(RectOffset {
                        left: 1,
                        right: 1,
                        top: 1,
                        bottom: 1,
                    })
                    .build(factory, theme);

                widgets.push(
                    Border::new()
                        .title(title)
                        .fg(theme.border())
                        .bg(theme.surface())
                        .child(code)
                        .build(factory, theme),
                );
                *total_lines += code_height + 4;
            }
        }
    }

    let _ = width;
}

fn indent_lines(lines: Vec<Vec<Span>>, theme: &Theme) -> Vec<Vec<Span>> {
    lines
        .into_iter()
        .map(|mut line| {
            let bg = line
                .first()
                .map(|span| span.bg)
                .unwrap_or_else(|| theme.surface());
            line.insert(
                0,
                Span::new("    ".to_string(), theme.text(), bg, Attrs::default()),
            );
            line
        })
        .collect()
}

fn surface_cell(theme: &Theme) -> Cell {
    Cell {
        bg: theme.surface(),
        ..Default::default()
    }
}

fn surface_span(
    text: impl Into<String>,
    fg: arbor_tui_domain::cell::AnsiColor,
    theme: &Theme,
    attrs: Attrs,
) -> Span {
    Span::new(text, fg, theme.surface(), attrs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::signal::Signal;
    use arbor_tui_testing::WidgetHarness;
    use aster_application::{ChatStreamError, StreamReceiver};
    use aster_domain::ChatMessage;

    #[derive(Clone)]
    struct FakeClient;

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
        ) -> Result<StreamReceiver, ChatStreamError> {
            unimplemented!("UI tests do not start streams")
        }
    }

    #[test]
    fn welcome_screen_has_no_black_text_background_in_light_theme() {
        let factory = WidgetFactory::new();
        let theme = Theme::light();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient)));
        let scroll = Signal::new(0u16);

        let root = build_ui(
            &factory,
            &theme,
            &state,
            scroll.read_only(),
            80,
            24,
            UiMetrics {
                fps: 0.0,
                last_frame_us: 0,
            },
        );
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(harness.find_text("Welcome to Aster").len() > 0);
        harness.assert_no_black_bg_on_text().unwrap();
    }
}
