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
use aster_domain::{ChatMessage, ChatRole, ConversationStatus};

use crate::state::AppState;

pub fn estimate_line_count(
    messages: &[ChatMessage],
    state: &ConversationStatus,
    theme: &Theme,
) -> usize {
    if messages.is_empty() {
        return 1;
    }

    let mut total_lines = 0usize;
    for message in messages {
        total_lines += 1;
        total_lines += estimate_content_lines(message.content(), theme);
        total_lines += 1;
    }

    if matches!(state, ConversationStatus::Error { .. }) {
        total_lines += 2;
    }

    total_lines
}

pub fn build_ui(
    factory: &WidgetFactory,
    theme: &Theme,
    state: &Rc<RefCell<AppState>>,
    scroll_y: ReadSignal<u16>,
    cols: u16,
    rows: u16,
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
        .content_h(usize_to_u16_saturating(line_count.max(1)))
        .child(message_stack)
        .build(factory, theme);

    let footer = Text::new(footer_text(line_count))
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

fn footer_text(line_count: usize) -> String {
    format!("{line_count} lines | Up/Down: scroll Enter: send Esc/Ctrl+C: quit")
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

fn estimate_content_lines(content: &str, theme: &Theme) -> usize {
    if content.is_empty() {
        return 1;
    }

    aster_markdown::parse_blocks(content, theme)
        .into_iter()
        .map(|block| match block {
            aster_markdown::Block::Text(lines) => lines.len(),
            aster_markdown::Block::Code { lines, .. } => lines.len() + 4,
        })
        .sum()
}

fn usize_to_u16_saturating(value: usize) -> u16 {
    value.min(usize::from(u16::MAX)) as u16
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
    use aster_application::{ChatStreamError, ChatStreamPort, StreamReceiver};
    use aster_domain::ChatMessage;

    #[derive(Clone)]
    struct FakeClient {
        events: Vec<aster_application::StreamEvent>,
    }

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = std::sync::mpsc::channel();
            for event in self.events.clone() {
                tx.send(event).unwrap();
            }
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn welcome_screen_has_no_black_text_background_in_light_theme() {
        let factory = WidgetFactory::new();
        let theme = Theme::light();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient { events: vec![] })));
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(harness.find_text("Welcome to Aster").len() > 0);
        harness.assert_no_black_bg_on_text().unwrap();
    }

    #[test]
    fn clamped_scroll_offset_keeps_short_reply_visible() {
        let factory = WidgetFactory::new();
        let theme = Theme::dark();
        let state = Rc::new(RefCell::new(AppState::new(FakeClient {
            events: vec![
                aster_application::StreamEvent::Token("visible reply".to_string()),
                aster_application::StreamEvent::Done,
            ],
        })));
        state.borrow_mut().submit_message("hello".to_string());
        state.borrow_mut().poll_stream_and_take_changed();
        let scroll = Signal::new(0u16);

        let root = build_ui(&factory, &theme, &state, scroll.read_only(), 80, 24);
        let harness = WidgetHarness::render(&root, 80, 24, &theme);

        assert!(harness.find_text("visible reply").len() > 0);
    }
}
