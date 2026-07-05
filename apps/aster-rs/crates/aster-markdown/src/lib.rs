// aster-markdown - Markdown to arbor-tui spans.

use std::sync::OnceLock;

use arbor_tui_domain::cell::{AnsiColor, Attrs, PaletteColor, Span};
use arbor_tui_domain::theme::Theme;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::highlighting::{Highlighter, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack};
use syntect::parsing::{ScopeStackOp, SyntaxSet};

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Text(Vec<Vec<Span>>),
    Code { lang: String, lines: Vec<Vec<Span>> },
}

pub fn parse_blocks(text: &str, theme: &Theme) -> Vec<Block> {
    let parser = MarkdownBlockParser::new(theme);
    parser.parse(text)
}

pub fn render_message(text: &str, theme: &Theme) -> Vec<Vec<Span>> {
    parse_blocks(text, theme)
        .into_iter()
        .flat_map(|block| match block {
            Block::Text(lines) | Block::Code { lines, .. } => lines,
        })
        .collect()
}

struct MarkdownBlockParser<'a> {
    theme: &'a Theme,
    blocks: Vec<Block>,
    current_block: Vec<Vec<Span>>,
    current_line: Vec<Span>,
    attrs_stack: Vec<Attrs>,
    fg_stack: Vec<Option<AnsiColor>>,
    in_code: bool,
    code_lang: String,
    code_buf: String,
}

impl<'a> MarkdownBlockParser<'a> {
    fn new(theme: &'a Theme) -> Self {
        Self {
            theme,
            blocks: Vec::new(),
            current_block: Vec::new(),
            current_line: Vec::new(),
            attrs_stack: vec![Attrs::default()],
            fg_stack: vec![None],
            in_code: false,
            code_lang: String::new(),
            code_buf: String::new(),
        }
    }

    fn parse(mut self, text: &str) -> Vec<Block> {
        let parser = Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH);
        for event in parser {
            if self.in_code {
                self.handle_code_event(event);
            } else {
                self.handle_text_event(event);
            }
        }

        if !self.current_line.is_empty() {
            self.current_block
                .push(std::mem::take(&mut self.current_line));
        }
        self.flush_text_block();
        self.blocks
    }

    fn handle_code_event(&mut self, event: Event<'_>) {
        match event {
            Event::Text(text) => self.code_buf.push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                self.in_code = false;
                self.flush_text_block();
                let source = std::mem::take(&mut self.code_buf);
                let trimmed = source.trim_end_matches('\n');
                let lines = if trimmed.is_empty() {
                    Vec::new()
                } else if self.code_lang.is_empty() {
                    trimmed
                        .lines()
                        .map(|line| {
                            vec![Span::new(
                                line.to_string(),
                                code_fg(),
                                code_bg(),
                                Attrs::default(),
                            )]
                        })
                        .collect()
                } else {
                    highlight_block(&self.code_lang, trimmed)
                };

                self.blocks.push(Block::Code {
                    lang: std::mem::take(&mut self.code_lang),
                    lines,
                });
            }
            _ => {}
        }
    }

    fn handle_text_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                if !self.current_line.is_empty() {
                    self.finish_line();
                }
                self.flush_text_block();
                self.in_code = true;
                self.code_lang = match &kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_string(),
                    _ => String::new(),
                };
                self.code_buf.clear();
            }
            Event::Text(text) => self.push_text(&text, self.current_fg(), self.current_attrs()),
            Event::Code(text) => self.push_text(&text, code_fg(), Attrs::default()),
            Event::Start(Tag::Heading { .. }) => {
                self.finish_line();
                self.fg_stack.push(Some(self.theme.accent()));
                let mut attrs = self.current_attrs();
                attrs.bold = true;
                self.attrs_stack.push(attrs);
            }
            Event::End(TagEnd::Heading(_)) => {
                self.attrs_stack.pop();
                self.fg_stack.pop();
                self.finish_line();
            }
            Event::Start(Tag::Item) => {
                self.finish_line();
                self.current_line.push(Span::new(
                    "  * ".to_string(),
                    self.theme.accent(),
                    self.theme.surface(),
                    Attrs::default(),
                ));
            }
            Event::End(TagEnd::Item) => self.finish_line(),
            Event::Start(Tag::Strong) => {
                let mut attrs = self.current_attrs();
                attrs.bold = true;
                self.attrs_stack.push(attrs);
            }
            Event::Start(Tag::Emphasis) => {
                let mut attrs = self.current_attrs();
                attrs.italic = true;
                self.attrs_stack.push(attrs);
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis) => {
                if self.attrs_stack.len() > 1 {
                    self.attrs_stack.pop();
                }
            }
            Event::Start(Tag::Strikethrough) => self.fg_stack.push(Some(self.theme.text_dim())),
            Event::End(TagEnd::Strikethrough) => {
                if self.fg_stack.len() > 1 {
                    self.fg_stack.pop();
                }
            }
            Event::Start(Tag::Link { .. }) => self.fg_stack.push(Some(self.theme.primary())),
            Event::End(TagEnd::Link) => {
                if self.fg_stack.len() > 1 {
                    self.fg_stack.pop();
                }
            }
            Event::SoftBreak | Event::HardBreak => self.finish_line(),
            Event::Rule => {
                self.finish_line();
                self.current_line.push(Span::new(
                    "-".repeat(40),
                    self.theme.border(),
                    self.theme.surface(),
                    Attrs::default(),
                ));
                self.finish_line();
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                self.push_text(&text, self.theme.text_dim(), Attrs::default());
            }
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str, fg: AnsiColor, attrs: Attrs) {
        for (index, line) in text.lines().enumerate() {
            if index > 0 {
                self.finish_line();
            }
            if !line.is_empty() {
                let bg = if fg == code_fg() {
                    code_bg()
                } else {
                    self.theme.surface()
                };
                self.current_line
                    .push(Span::new(line.to_string(), fg, bg, attrs));
            }
        }
    }

    fn finish_line(&mut self) {
        self.current_block
            .push(std::mem::take(&mut self.current_line));
    }

    fn flush_text_block(&mut self) {
        if !self.current_block.is_empty() {
            self.blocks
                .push(Block::Text(std::mem::take(&mut self.current_block)));
        }
    }

    fn current_attrs(&self) -> Attrs {
        *self.attrs_stack.last().expect("attrs stack is never empty")
    }

    fn current_fg(&self) -> AnsiColor {
        self.fg_stack
            .last()
            .expect("fg stack is never empty")
            .unwrap_or_else(|| self.theme.text())
    }
}

fn code_bg() -> AnsiColor {
    AnsiColor {
        palette: PaletteColor(236),
        true_color: None,
    }
}

fn code_fg() -> AnsiColor {
    AnsiColor {
        palette: PaletteColor(250),
        true_color: None,
    }
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn highlight_theme() -> &'static syntect::highlighting::Theme {
    static THEME: OnceLock<syntect::highlighting::Theme> = OnceLock::new();
    THEME.get_or_init(|| ThemeSet::load_defaults().themes["base16-ocean.dark"].clone())
}

fn highlight_block(lang: &str, source: &str) -> Vec<Vec<Span>> {
    let syn = syntax_set()
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| syntax_set().find_syntax_plain_text());
    let highlighter = Highlighter::new(highlight_theme());
    let mut state = ParseState::new(syn);
    let mut scope = ScopeStack::new();
    let mut out = Vec::new();

    for line in source.lines() {
        let ops = match state.parse_line(line, syntax_set()) {
            Ok(ops) => ops,
            Err(_) => {
                out.push(vec![Span::new(
                    line.to_string(),
                    code_fg(),
                    code_bg(),
                    Attrs::default(),
                )]);
                state = ParseState::new(syn);
                scope = ScopeStack::new();
                continue;
            }
        };

        let mut spans = Vec::new();
        let mut pos = 0;
        let mut scopes_ok = true;
        for (next, op) in &ops {
            let end = *next;
            if end > pos {
                let style = highlighter.style_for_stack(scope.as_slice());
                spans.push(Span::new(
                    line[pos..end].to_string(),
                    style_to_ansi(style),
                    code_bg(),
                    Attrs::default(),
                ));
            }
            if scope.apply(op).is_err() {
                scopes_ok = false;
                break;
            }
            pos = end;
        }

        if !scopes_ok {
            if pos < line.len() {
                spans.push(Span::new(
                    line[pos..].to_string(),
                    code_fg(),
                    code_bg(),
                    Attrs::default(),
                ));
            }
            state = ParseState::new(syn);
            scope = ScopeStack::new();
        } else if pos < line.len() {
            let style = highlighter.style_for_stack(scope.as_slice());
            spans.push(Span::new(
                line[pos..].to_string(),
                style_to_ansi(style),
                code_bg(),
                Attrs::default(),
            ));
        }

        while let Some(top) = scope.as_slice().last() {
            if top.to_string().contains("comment.line") {
                let _ = scope.apply(&ScopeStackOp::Pop(1));
            } else {
                break;
            }
        }

        out.push(spans);
    }

    out
}

fn style_to_ansi(style: syntect::highlighting::Style) -> AnsiColor {
    if style.foreground.a == 0 {
        code_fg()
    } else {
        rgb_to_ansi256(style.foreground.r, style.foreground.g, style.foreground.b)
    }
}

fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> AnsiColor {
    let standard: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];

    let mut best_idx = 7u8;
    let mut best_dist = u32::MAX;
    for (idx, &(sr, sg, sb)) in standard.iter().enumerate() {
        let dist = color_distance(r, g, b, sr, sg, sb);
        if dist < best_dist {
            best_dist = dist;
            best_idx = idx as u8;
        }
    }

    let ri = (r as f32 * 5.0 / 255.0).round() as u8;
    let gi = (g as f32 * 5.0 / 255.0).round() as u8;
    let bi = (b as f32 * 5.0 / 255.0).round() as u8;
    let cube_r = ri * 51;
    let cube_g = gi * 51;
    let cube_b = bi * 51;
    let cube_idx = 16 + 36 * ri + 6 * gi + bi;
    let cube_dist = color_distance(r, g, b, cube_r, cube_g, cube_b);
    if cube_dist < best_dist {
        best_idx = cube_idx;
        best_dist = cube_dist;
    }

    let gray = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;
    let gray_level = ((gray as f32 * 23.0 / 255.0).round() as u8).min(23);
    let gray_value = gray_level * 10 + 8;
    let gray_dist = color_distance(r, g, b, gray_value, gray_value, gray_value);
    let gray_idx = 232 + gray_level;
    if gray_dist < best_dist {
        best_idx = gray_idx;
    }

    AnsiColor {
        palette: PaletteColor(best_idx),
        true_color: None,
    }
}

fn color_distance(r: u8, g: u8, b: u8, cr: u8, cg: u8, cb: u8) -> u32 {
    let dr = r as i32 - cr as i32;
    let dg = g as i32 - cg as i32;
    let db = b as i32 - cb as i32;
    (dr * dr + dg * dg + db * db) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> Theme {
        Theme::dark()
    }

    fn flatten(lines: &[Vec<Span>]) -> String {
        lines
            .iter()
            .flat_map(|line| line.iter().map(|span| span.text.as_str()))
            .collect()
    }

    #[test]
    fn plain_text_renders_without_markup() {
        assert_eq!(flatten(&render_message("hi", &theme())), "hi");
    }

    #[test]
    fn bold_marks_span_attrs() {
        let rendered = render_message("**x**", &theme());

        assert!(rendered
            .iter()
            .flat_map(|line| line.iter())
            .any(|span| span.text == "x" && span.attrs.bold));
    }

    #[test]
    fn inline_code_drops_backticks() {
        let rendered = render_message("`cargo`", &theme());

        assert!(flatten(&rendered).contains("cargo"));
        assert!(!flatten(&rendered).contains('`'));
    }

    #[test]
    fn code_block_is_split_into_code_block() {
        let blocks = parse_blocks("before\n```rust\nlet x = 1;\n```\nafter", &theme());

        assert!(matches!(blocks[0], Block::Text(_)));
        assert!(matches!(blocks[1], Block::Code { .. }));
        assert!(matches!(blocks[2], Block::Text(_)));
    }

    #[test]
    fn c_line_comment_does_not_leak_to_next_line() {
        let rendered = render_message("```c\nint x = 1; // comment\nint y = 2;\n```", &theme());
        let all = flatten(&rendered);

        assert!(all.contains("int x = 1; // comment"));
        assert!(all.contains("int y = 2;"));
        assert!(rendered
            .iter()
            .flatten()
            .filter(|span| span.text.contains("int"))
            .all(|span| span.bg.palette.0 == 236));
    }
}
