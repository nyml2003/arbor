// Markdown → Vec<Vec<Span>> via pulldown-cmark + syntect.
//
// Single-pass: pulldown-cmark event iterator drives everything.
// Code block text is buffered inline, highlighted at End(CodeBlock),
// everything else gets inline styling immediately.

use std::sync::OnceLock;

use arbor_tui_primitives::cell::{AnsiColor, Attrs, PaletteColor, Span};
use arbor_tui_render::theme::Theme;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use syntect::highlighting::{Highlighter, ThemeSet};
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

// ── Code block colors ───────────────────────────────────────────────

fn code_bg() -> AnsiColor { AnsiColor { palette: PaletteColor(236), true_color: None } }
fn code_fg() -> AnsiColor { AnsiColor { palette: PaletteColor(250), true_color: None } }

// ── Lazy init ───────────────────────────────────────────────────────

fn syntax_set() -> &'static SyntaxSet {
    static S: OnceLock<SyntaxSet> = OnceLock::new();
    S.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn hl_theme() -> &'static syntect::highlighting::Theme {
    static T: OnceLock<syntect::highlighting::Theme> = OnceLock::new();
    T.get_or_init(|| ThemeSet::load_defaults().themes["base16-ocean.dark"].clone())
}

// ── Syntax highlight one block ──────────────────────────────────────

fn highlight_block(lang: &str, source: &str) -> Vec<Vec<Span>> {
    let syn = syntax_set()
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| syntax_set().find_syntax_plain_text());
    let highlighter = Highlighter::new(hl_theme());
    let mut state = ParseState::new(syn);
    let mut scope = ScopeStack::new();
    let mut out: Vec<Vec<Span>> = Vec::new();

    for line in source.lines() {
        let ops = match state.parse_line(line, syntax_set()) {
            Ok(o) => o,
            Err(_) => {
                out.push(vec![Span::new(
                    line.to_string(), code_fg(), code_bg(), Attrs::default(),
                )]);
                state = ParseState::new(syn);
                scope = ScopeStack::new();
                continue;
            }
        };

        let mut spans: Vec<Span> = Vec::new();
        let mut pos: usize = 0;
        let mut scopes_ok = true;

        for (next, op) in &ops {
            let end = *next;
            if end > pos {
                let style = highlighter.style_for_stack(scope.as_slice());
                let fg = mk_fg(style);
                spans.push(Span::new(line[pos..end].to_string(), fg, code_bg(), Attrs::default()));
            }
            if scope.apply(op).is_err() {
                scopes_ok = false;
                break;
            }
            pos = end;
        }

        if !scopes_ok {
            // Scope stack corrupted — render rest of line plain, reset
            if pos < line.len() {
                spans.push(Span::new(line[pos..].to_string(), code_fg(), code_bg(), Attrs::default()));
            }
            state = ParseState::new(syn);
            scope = ScopeStack::new();
        } else if pos < line.len() {
            let style = highlighter.style_for_stack(scope.as_slice());
            let fg = mk_fg(style);
            spans.push(Span::new(line[pos..].to_string(), fg, code_bg(), Attrs::default()));
        }

        out.push(spans);
    }
    out
}

fn mk_fg(style: syntect::highlighting::Style) -> AnsiColor {
    if style.foreground.a == 0 {
        code_fg()
    } else {
        AnsiColor::from_rgb(style.foreground.r, style.foreground.g, style.foreground.b)
    }
}

// ── Public renderer ─────────────────────────────────────────────────

pub fn render_message(text: &str, theme: &Theme) -> Vec<Vec<Span>> {
    let mut out: Vec<Vec<Span>> = Vec::new();
    let mut cur: Vec<Span> = Vec::new();
    let empty_line = || out.push(Vec::new());

    // Inline style stacks
    let mut attrs_stack: Vec<Attrs> = vec![Attrs::default()];
    let mut fg_stack: Vec<Option<AnsiColor>> = vec![None];

    let bg = theme.surface();

    // Code block buffering
    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    let parser = Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH);

    for event in parser {
        if in_code {
            match event {
                Event::Text(t) => { code_buf.push_str(&t); continue; }
                Event::End(TagEnd::CodeBlock) => {
                    in_code = false;
                    // Fence label
                    let label = if code_lang.is_empty() {
                        "```".to_string()
                    } else {
                        format!("```{}", code_lang)
                    };
                    out.push(vec![Span::new(label, theme.text_dim(), bg, Attrs::default())]);
                    // Highlighted code
                    let source = std::mem::take(&mut code_buf);
                    let trimmed = source.trim_end_matches('\n');
                    if !trimmed.is_empty() {
                        if code_lang.is_empty() {
                            for line in trimmed.lines() {
                                out.push(vec![Span::new(line.to_string(), code_fg(), code_bg(), Attrs::default())]);
                            }
                        } else {
                            out.extend(highlight_block(&code_lang, trimmed));
                        }
                    }
                    code_lang.clear();
                    continue;
                }
                _ => continue, // ignore non-text events inside code blocks
            }
        }

        match event {
            // ── Code block start ──
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code = true;
                code_lang = match &kind {
                    CodeBlockKind::Fenced(l) if !l.is_empty() => l.to_string(),
                    _ => String::new(),
                };
                code_buf.clear();
            }

            // ── Inline text ──
            Event::Text(t) => {
                let fg = fg_stack.last().unwrap().unwrap_or(theme.text());
                let attrs = *attrs_stack.last().unwrap();
                for (i, line) in t.lines().enumerate() {
                    if i > 0 { out.push(std::mem::take(&mut cur)); }
                    if !line.is_empty() {
                        cur.push(Span::new(line.to_string(), fg, bg, attrs));
                    }
                }
            }
            Event::Code(t) => {
                for (i, line) in t.lines().enumerate() {
                    if i > 0 { out.push(std::mem::take(&mut cur)); }
                    if !line.is_empty() {
                        cur.push(Span::new(line.to_string(), code_fg(), code_bg(), Attrs::default()));
                    }
                }
            }

            // ── Headings ──
            Event::Start(Tag::Heading { .. }) => {
                out.push(std::mem::take(&mut cur));
                fg_stack.push(Some(theme.accent()));
                let mut top = *attrs_stack.last().unwrap();
                top.bold = true;
                attrs_stack.push(top);
            }
            Event::End(TagEnd::Heading(_)) => {
                attrs_stack.pop();
                fg_stack.pop();
                out.push(std::mem::take(&mut cur));
            }

            // ── Lists ──
            Event::Start(Tag::Item) => {
                out.push(std::mem::take(&mut cur));
                cur.push(Span::new("  • ".to_string(), theme.accent(), bg, Attrs::default()));
            }
            Event::End(TagEnd::Item) => { out.push(std::mem::take(&mut cur)); }

            // ── Inline formatting ──
            Event::Start(Tag::Strong) => {
                let mut top = *attrs_stack.last().unwrap();
                top.bold = true;
                attrs_stack.push(top);
            }
            Event::Start(Tag::Emphasis) => {
                let mut top = *attrs_stack.last().unwrap();
                top.italic = true;
                attrs_stack.push(top);
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis) => {
                if attrs_stack.len() > 1 { attrs_stack.pop(); }
            }
            Event::Start(Tag::Strikethrough) => { fg_stack.push(Some(theme.text_dim())); }
            Event::End(TagEnd::Strikethrough) => { if fg_stack.len() > 1 { fg_stack.pop(); } }
            Event::Start(Tag::Link { .. }) => { fg_stack.push(Some(theme.primary())); }
            Event::End(TagEnd::Link) => { if fg_stack.len() > 1 { fg_stack.pop(); } }

            // ── Breaks ──
            Event::SoftBreak | Event::HardBreak => { out.push(std::mem::take(&mut cur)); }
            Event::Rule => {
                out.push(std::mem::take(&mut cur));
                cur.push(Span::new("─".repeat(40), theme.border(), bg, Attrs::default()));
                out.push(std::mem::take(&mut cur));
            }

            // ── HTML ──
            Event::Html(t) | Event::InlineHtml(t) => {
                cur.push(Span::new(t.to_string(), theme.text_dim(), bg, Attrs::default()));
            }

            // ── Ignored ──
            Event::Start(Tag::List(_) | Tag::Paragraph | Tag::BlockQuote(_)) => {}
            Event::End(TagEnd::List(_) | TagEnd::Paragraph | TagEnd::BlockQuote(_)) => {}
            Event::FootnoteReference(_) | Event::TaskListMarker(_)
            | Event::InlineMath(_) | Event::DisplayMath(_) => {}
            _ => {}
        }
    }

    if !cur.is_empty() { out.push(cur); }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Theme { Theme::dark() }

    fn flatten(r: &[Vec<Span>]) -> String {
        r.iter().flat_map(|l| l.iter().map(|s| s.text.as_str())).collect()
    }

    #[test]
    fn plain() { assert_eq!(flatten(&render_message("hi", &t())), "hi"); }

    #[test]
    fn bold() {
        let r = render_message("**x**", &t());
        assert!(r.iter().flat_map(|l| l.iter()).any(|s| s.text == "x" && s.attrs.bold));
    }

    #[test]
    fn inline_code() {
        let r = render_message("`cargo`", &t());
        assert!(flatten(&r).contains("cargo"));
        assert!(!flatten(&r).contains('`'));
    }

    #[test]
    fn heading() {
        let r = render_message("## T", &t());
        let s = r.iter().flat_map(|l| l.iter()).find(|s| s.text == "T").unwrap();
        assert!(s.attrs.bold);
    }

    #[test]
    fn list() {
        let r = render_message("- a\n- b", &t());
        assert!(flatten(&r).contains("•"));
        assert!(flatten(&r).contains("a"));
    }

    #[test]
    fn code_block_plain() {
        let r = render_message("```\nline1\nline2\n```", &t());
        assert!(flatten(&r).contains("line1"));
        assert!(flatten(&r).contains("line2"));
    }

    #[test]
    fn code_block_python() {
        let src = "```python\ndef foo():\n    pass\n```";
        let r = render_message(src, &t());
        let all = flatten(&r);
        assert!(all.contains("def"));
        assert!(all.contains("pass"));
        // fence label is preserved (dimmed) — code content is highlighted
    }

    #[test]
    fn code_block_mid_document() {
        let src = "text before\n```rust\nlet x = 1;\n```\ntext after";
        let r = render_message(src, &t());
        let all = flatten(&r);
        assert!(all.contains("text before"));
        assert!(all.contains("let x = 1"));
        assert!(all.contains("text after"));
    }
}
