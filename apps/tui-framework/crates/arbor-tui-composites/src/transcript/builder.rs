use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell, Span};
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_markdown::{parse_blocks, MarkdownBlock};
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::usize_to_u16_saturating;

#[derive(Clone)]
pub struct TranscriptMessage {
    label: String,
    label_fg: AnsiColor,
    body: String,
}

impl TranscriptMessage {
    pub fn new(label: impl Into<String>, label_fg: AnsiColor, body: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            label_fg,
            body: body.into(),
        }
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Clone)]
pub struct TranscriptNotice {
    title: String,
    detail: String,
    fg: AnsiColor,
}

impl TranscriptNotice {
    pub fn new(title: impl Into<String>, detail: impl Into<String>, fg: AnsiColor) -> Self {
        Self {
            title: title.into(),
            detail: detail.into(),
            fg,
        }
    }
}

pub struct Transcript {
    messages: Vec<TranscriptMessage>,
    empty_text: String,
    notice: Option<TranscriptNotice>,
    scroll_y: Option<ReadSignal<u16>>,
    bg: Option<AnsiColor>,
    flex: f32,
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

impl Transcript {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            empty_text: String::new(),
            notice: None,
            scroll_y: None,
            bg: None,
            flex: 0.0,
        }
    }

    pub fn messages(mut self, messages: impl IntoIterator<Item = TranscriptMessage>) -> Self {
        self.messages = messages.into_iter().collect();
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<String>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn notice(mut self, notice: Option<TranscriptNotice>) -> Self {
        self.notice = notice;
        self
    }

    pub fn scroll_y(mut self, signal: ReadSignal<u16>) -> Self {
        self.scroll_y = Some(signal);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    pub fn line_count(&self, theme: &Theme) -> usize {
        estimate_line_count(&self.messages, self.notice.as_ref(), theme)
    }

    pub fn build(self, factory: &WidgetFactory, theme: &Theme) -> WidgetNode {
        let bg = self.bg.unwrap_or_else(|| theme.surface());
        let notice_lines = self.notice.as_ref().map_or(0, |_| 2);
        let mut line_count = notice_lines;
        let mut widgets = Vec::new();

        if self.messages.is_empty() {
            line_count += 1;
            widgets.push(
                RichText::new()
                    .bg(bg_cell(bg))
                    .lines(vec![vec![span(
                        format!("  {}", self.empty_text),
                        theme.text(),
                        bg,
                        Attrs::default(),
                    )]])
                    .build(factory, theme),
            );
        } else {
            for message in self.messages {
                push_message_label(&mut widgets, &message, bg, theme, factory);
                line_count += 1;

                let blocks = parse_markdown_blocks(message.body(), theme);
                line_count += count_markdown_block_lines(&blocks);
                push_message_content_blocks(&mut widgets, blocks, bg, theme, factory);

                widgets.push(blank_line(bg, theme, factory));
                line_count += 1;
            }
        }

        if let Some(notice) = &self.notice {
            widgets.push(
                RichText::new()
                    .bg(bg_cell(bg))
                    .lines(vec![
                        vec![span(
                            format!("  {}", notice.title),
                            notice.fg,
                            bg,
                            Attrs::default(),
                        )],
                        vec![span(
                            format!("  {}", notice.detail),
                            theme.text_dim(),
                            bg,
                            Attrs::default(),
                        )],
                    ])
                    .build(factory, theme),
            );
        }

        let stack = Col::new().flex(1.0).children(widgets).build(factory, theme);
        let mut scroll = Scroll::new()
            .flex(self.flex)
            .content_h(usize_to_u16_saturating(line_count.max(1)))
            .child(stack);
        if let Some(scroll_y) = self.scroll_y {
            scroll = scroll.scroll_y(scroll_y);
        }
        scroll.build(factory, theme)
    }
}

fn estimate_line_count(
    messages: &[TranscriptMessage],
    notice: Option<&TranscriptNotice>,
    theme: &Theme,
) -> usize {
    if messages.is_empty() {
        return 1 + notice.map_or(0, |_| 2);
    }

    let mut total = 0usize;
    for message in messages {
        total += 1;
        total += estimate_markdown_lines(message.body(), theme);
        total += 1;
    }
    total + notice.map_or(0, |_| 2)
}

fn estimate_markdown_lines(content: &str, theme: &Theme) -> usize {
    count_markdown_block_lines(&parse_markdown_blocks(content, theme))
}

fn parse_markdown_blocks(content: &str, theme: &Theme) -> Vec<MarkdownBlock> {
    if content.is_empty() {
        Vec::new()
    } else {
        parse_blocks(content, theme)
    }
}

fn count_markdown_block_lines(blocks: &[MarkdownBlock]) -> usize {
    if blocks.is_empty() {
        return 1;
    }

    blocks
        .iter()
        .map(|block| match block {
            MarkdownBlock::Text(lines) => lines.len(),
            MarkdownBlock::Code { lines, .. } => lines.len() + 4,
        })
        .sum()
}

fn push_message_label(
    widgets: &mut Vec<WidgetNode>,
    message: &TranscriptMessage,
    bg: AnsiColor,
    theme: &Theme,
    factory: &WidgetFactory,
) {
    widgets.push(
        RichText::new()
            .bg(bg_cell(bg))
            .lines(vec![vec![
                span("  ", theme.text(), bg, Attrs::default()),
                span(
                    format!("{}: ", message.label),
                    message.label_fg,
                    bg,
                    Attrs {
                        bold: true,
                        ..Default::default()
                    },
                ),
            ]])
            .build(factory, theme),
    );
}

fn push_message_content_blocks(
    widgets: &mut Vec<WidgetNode>,
    blocks: Vec<MarkdownBlock>,
    bg: AnsiColor,
    theme: &Theme,
    factory: &WidgetFactory,
) {
    if blocks.is_empty() {
        widgets.push(blank_line(bg, theme, factory));
        return;
    }

    for block in blocks {
        match block {
            MarkdownBlock::Text(lines) => {
                widgets.push(
                    RichText::new()
                        .bg(bg_cell(bg))
                        .lines(indent_lines(lines, bg, theme))
                        .build(factory, theme),
                );
            }
            MarkdownBlock::Code { lang, lines } => {
                let title = if lang.is_empty() {
                    String::new()
                } else {
                    format!(" {lang} ")
                };
                let code = RichText::new()
                    .bg(bg_cell(code_bg()))
                    .lines(pad_code_lines(lines, theme))
                    .build(factory, theme);

                widgets.push(
                    Border::new()
                        .title(title)
                        .fg(theme.border())
                        .bg(bg)
                        .child(code)
                        .build(factory, theme),
                );
            }
        }
    }
}

fn indent_lines(lines: Vec<Vec<Span>>, bg: AnsiColor, theme: &Theme) -> Vec<Vec<Span>> {
    lines
        .into_iter()
        .map(|mut line| {
            line.insert(0, span("    ", theme.text(), bg, Attrs::default()));
            line
        })
        .collect()
}

fn pad_code_lines(lines: Vec<Vec<Span>>, theme: &Theme) -> Vec<Vec<Span>> {
    let mut padded = Vec::with_capacity(lines.len() + 2);
    padded.push(vec![span(
        "",
        theme.text_dim(),
        code_bg(),
        Attrs::default(),
    )]);
    for mut line in lines {
        line.insert(0, span(" ", theme.text_dim(), code_bg(), Attrs::default()));
        padded.push(line);
    }
    padded.push(vec![span(
        "",
        theme.text_dim(),
        code_bg(),
        Attrs::default(),
    )]);
    padded
}

fn blank_line(bg: AnsiColor, theme: &Theme, factory: &WidgetFactory) -> WidgetNode {
    RichText::new()
        .bg(bg_cell(bg))
        .lines(vec![vec![span("", theme.text(), bg, Attrs::default())]])
        .build(factory, theme)
}

fn bg_cell(bg: AnsiColor) -> Cell {
    Cell {
        bg,
        ..Default::default()
    }
}

fn span(text: impl Into<String>, fg: AnsiColor, bg: AnsiColor, attrs: Attrs) -> Span {
    Span::new(text, fg, bg, attrs)
}

fn code_bg() -> AnsiColor {
    AnsiColor::from_palette(236)
}
