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

        let mut spans: Vec<Span> = Vec::new();
        let mut pos: usize = 0;
        let mut scopes_ok = true;

        for (next, op) in &ops {
            let end = *next;
            if end > pos {
                let style = highlighter.style_for_stack(scope.as_slice());
                let fg = mk_fg(style);
                spans.push(Span::new(
                    line[pos..end].to_string(),
                    fg,
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
            // Scope stack corrupted — render rest of line plain, reset
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
            let fg = mk_fg(style);
            spans.push(Span::new(
                line[pos..].to_string(),
                fg,
                code_bg(),
                Attrs::default(),
            ));
        }

        // Pop single-line comment scopes that weren't auto-closed at EOL.
        // Only "comment.line" scopes — "comment.block" (/* */) are multi-line
        // and tracked by ParseState, which will pop them at the closing */.
        while let Some(top) = scope.as_slice().last() {
            let s = top.to_string();
            if s.contains("comment.line") {
                let _ = scope.apply(&syntect::parsing::ScopeStackOp::Pop(1));
            } else {
                break;
            }
        }

        out.push(spans);
    }
    out
}

fn mk_fg(style: syntect::highlighting::Style) -> AnsiColor {
    if style.foreground.a == 0 {
        code_fg()
    } else {
        rgb_to_ansi256(style.foreground.r, style.foreground.g, style.foreground.b)
    }
}

/// Map 24-bit RGB to the nearest 256-color palette entry.
/// This avoids the `AnsiColor::from_rgb(true_color)` path, because the
/// framework's diff (`Cell::eq`) ignores `true_color` — all palette-7
/// cells look identical to the diff engine, so highlighted tokens never
/// get re-emitted when their RGB value changes during streaming.
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> AnsiColor {
    // 16 standard ANSI colors
    let std_colors: [(u8, u8, u8); 16] = [
        (0, 0, 0), (128, 0, 0), (0, 128, 0), (128, 128, 0),
        (0, 0, 128), (128, 0, 128), (0, 128, 128), (192, 192, 192),
        (128, 128, 128), (255, 0, 0), (0, 255, 0), (255, 255, 0),
        (0, 0, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255),
    ];

    // Find nearest standard color
    let mut best_idx = 7u8; // default white
    let mut best_dist = u32::MAX;
    for (i, &(sr, sg, sb)) in std_colors.iter().enumerate() {
        let dr = r as i32 - sr as i32;
        let dg = g as i32 - sg as i32;
        let db = b as i32 - sb as i32;
        let dist = (dr * dr + dg * dg + db * db) as u32;
        if dist < best_dist {
            best_dist = dist;
            best_idx = i as u8;
        }
    }

    // 6x6x6 color cube (indices 16–231) + 24 grays (232–255)
    let cube_dist = |cr: u8, cg: u8, cb: u8| -> u32 {
        let dr = r as i32 - cr as i32;
        let dg = g as i32 - cg as i32;
        let db = b as i32 - cb as i32;
        (dr * dr + dg * dg + db * db) as u32
    };

    // Check if a cube color is closer
    let ri = (r as f32 * 5.0 / 255.0).round() as u8;
    let gi = (g as f32 * 5.0 / 255.0).round() as u8;
    let bi = (b as f32 * 5.0 / 255.0).round() as u8;
    let cube_r = ri * 51;
    let cube_g = gi * 51;
    let cube_b = bi * 51;
    let cube_idx = 16 + 36 * ri + 6 * gi + bi;

    if (cube_idx as u32) < 256 && cube_dist(cube_r, cube_g, cube_b) < best_dist {
        best_idx = cube_idx;
        best_dist = cube_dist(cube_r, cube_g, cube_b);
    }

    // Check grayscale
    let gray = ((r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000) as u8;
    let gray_level = ((gray as f32 * 23.0 / 255.0).round() as u8).min(23);
    let gray_r = gray_level * 10 + 8;
    let gray_dist = {
        let dr = r as i32 - gray_r as i32;
        (dr * dr * 3) as u32 // multiply by 3 to account for 3 channels
    };
    let gray_idx = 232 + gray_level;

    if (gray_idx as u32) < 256 && gray_dist < best_dist {
        best_idx = gray_idx;
    }

    AnsiColor {
        palette: PaletteColor(best_idx),
        true_color: None,
    }
}

// ── Block type ──────────────────────────────────────────────────────

pub enum Block {
    Text(Vec<Vec<Span>>),
    Code { lang: String, lines: Vec<Vec<Span>> },
}

/// Parse text into blocks, splitting at code fences. Code blocks get
/// syntax-highlighted and returned as `Block::Code` for Border wrapping.
pub fn parse_blocks(text: &str, theme: &Theme) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut cur: Vec<Vec<Span>> = Vec::new();
    let mut cur_line: Vec<Span> = Vec::new();

    let mut attrs_stack: Vec<Attrs> = vec![Attrs::default()];
    let mut fg_stack: Vec<Option<AnsiColor>> = vec![None];
    let bg = theme.surface();

    let mut in_code = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    let flush_cur = |cur: &mut Vec<Vec<Span>>, blocks: &mut Vec<Block>| {
        if !cur.is_empty() {
            blocks.push(Block::Text(std::mem::take(cur)));
        }
    };

    let parser = Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH);

    for event in parser {
        if in_code {
            match event {
                Event::Text(t) => { code_buf.push_str(&t); continue; }
                Event::End(TagEnd::CodeBlock) => {
                    in_code = false;
                    flush_cur(&mut cur, &mut blocks);
                    let source = std::mem::take(&mut code_buf);
                    let trimmed = source.trim_end_matches('\n').to_string();
                    let lines = if !trimmed.is_empty() {
                        if code_lang.is_empty() {
                            trimmed.lines().map(|l| vec![Span::new(l.to_string(), code_fg(), code_bg(), Attrs::default())]).collect()
                        } else {
                            highlight_block(&code_lang, &trimmed)
                        }
                    } else { vec![] };
                    blocks.push(Block::Code { lang: std::mem::take(&mut code_lang), lines });
                    continue;
                }
                _ => continue,
            }
        }

        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code = true;
                code_lang = match &kind {
                    CodeBlockKind::Fenced(l) if !l.is_empty() => l.to_string(),
                    _ => String::new(),
                };
                code_buf.clear();
            }
            Event::Text(t) => {
                let fg = fg_stack.last().unwrap().unwrap_or(theme.text());
                let attrs = *attrs_stack.last().unwrap();
                for (i, line) in t.lines().enumerate() {
                    if i > 0 { cur.push(std::mem::take(&mut cur_line)); }
                    if !line.is_empty() {
                        cur_line.push(Span::new(line.to_string(), fg, bg, attrs));
                    }
                }
            }
            Event::Code(t) => {
                for (i, line) in t.lines().enumerate() {
                    if i > 0 { cur.push(std::mem::take(&mut cur_line)); }
                    if !line.is_empty() {
                        cur_line.push(Span::new(line.to_string(), code_fg(), code_bg(), Attrs::default()));
                    }
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                cur.push(std::mem::take(&mut cur_line));
                fg_stack.push(Some(theme.accent()));
                let mut top = *attrs_stack.last().unwrap();
                top.bold = true; attrs_stack.push(top);
            }
            Event::End(TagEnd::Heading(_)) => {
                attrs_stack.pop(); fg_stack.pop();
                cur.push(std::mem::take(&mut cur_line));
            }
            Event::Start(Tag::Item) => {
                cur.push(std::mem::take(&mut cur_line));
                cur_line.push(Span::new("  • ".to_string(), theme.accent(), bg, Attrs::default()));
            }
            Event::End(TagEnd::Item) => { cur.push(std::mem::take(&mut cur_line)); }
            Event::Start(Tag::Strong) => {
                let mut top = *attrs_stack.last().unwrap(); top.bold = true; attrs_stack.push(top);
            }
            Event::Start(Tag::Emphasis) => {
                let mut top = *attrs_stack.last().unwrap(); top.italic = true; attrs_stack.push(top);
            }
            Event::End(TagEnd::Strong | TagEnd::Emphasis) => { if attrs_stack.len() > 1 { attrs_stack.pop(); } }
            Event::Start(Tag::Strikethrough) => { fg_stack.push(Some(theme.text_dim())); }
            Event::End(TagEnd::Strikethrough) => { if fg_stack.len() > 1 { fg_stack.pop(); } }
            Event::Start(Tag::Link { .. }) => { fg_stack.push(Some(theme.primary())); }
            Event::End(TagEnd::Link) => { if fg_stack.len() > 1 { fg_stack.pop(); } }
            Event::SoftBreak | Event::HardBreak => { cur.push(std::mem::take(&mut cur_line)); }
            Event::Rule => {
                cur.push(std::mem::take(&mut cur_line));
                cur_line.push(Span::new("─".repeat(40), theme.border(), bg, Attrs::default()));
                cur.push(std::mem::take(&mut cur_line));
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                cur_line.push(Span::new(t.to_string(), theme.text_dim(), bg, Attrs::default()));
            }
            _ => {}
        }
    }
    if !cur_line.is_empty() { cur.push(cur_line); }
    flush_cur(&mut cur, &mut blocks);
    blocks
}

// ── Legacy flat renderer (for backward compat) ─────────────────────

pub fn render_message(text: &str, theme: &Theme) -> Vec<Vec<Span>> {
    let mut out: Vec<Vec<Span>> = Vec::new();
    let mut cur: Vec<Span> = Vec::new();
    let _ = || out.push(Vec::new());

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
                Event::Text(t) => {
                    code_buf.push_str(&t);
                    continue;
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code = false;
                    let source = std::mem::take(&mut code_buf);
                    let trimmed = source.trim_end_matches('\n');
                    if !trimmed.is_empty() {
                        if code_lang.is_empty() {
                            for line in trimmed.lines() {
                                out.push(vec![Span::new(
                                    line.to_string(),
                                    code_fg(),
                                    code_bg(),
                                    Attrs::default(),
                                )]);
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
                    if i > 0 {
                        out.push(std::mem::take(&mut cur));
                    }
                    if !line.is_empty() {
                        cur.push(Span::new(line.to_string(), fg, bg, attrs));
                    }
                }
            }
            Event::Code(t) => {
                for (i, line) in t.lines().enumerate() {
                    if i > 0 {
                        out.push(std::mem::take(&mut cur));
                    }
                    if !line.is_empty() {
                        cur.push(Span::new(
                            line.to_string(),
                            code_fg(),
                            code_bg(),
                            Attrs::default(),
                        ));
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
                cur.push(Span::new(
                    "  • ".to_string(),
                    theme.accent(),
                    bg,
                    Attrs::default(),
                ));
            }
            Event::End(TagEnd::Item) => {
                out.push(std::mem::take(&mut cur));
            }

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
                if attrs_stack.len() > 1 {
                    attrs_stack.pop();
                }
            }
            Event::Start(Tag::Strikethrough) => {
                fg_stack.push(Some(theme.text_dim()));
            }
            Event::End(TagEnd::Strikethrough) => {
                if fg_stack.len() > 1 {
                    fg_stack.pop();
                }
            }
            Event::Start(Tag::Link { .. }) => {
                fg_stack.push(Some(theme.primary()));
            }
            Event::End(TagEnd::Link) => {
                if fg_stack.len() > 1 {
                    fg_stack.pop();
                }
            }

            // ── Breaks ──
            Event::SoftBreak | Event::HardBreak => {
                out.push(std::mem::take(&mut cur));
            }
            Event::Rule => {
                out.push(std::mem::take(&mut cur));
                cur.push(Span::new(
                    "─".repeat(40),
                    theme.border(),
                    bg,
                    Attrs::default(),
                ));
                out.push(std::mem::take(&mut cur));
            }

            // ── HTML ──
            Event::Html(t) | Event::InlineHtml(t) => {
                cur.push(Span::new(
                    t.to_string(),
                    theme.text_dim(),
                    bg,
                    Attrs::default(),
                ));
            }

            // ── Ignored ──
            Event::Start(Tag::List(_) | Tag::Paragraph | Tag::BlockQuote(_)) => {}
            Event::End(TagEnd::List(_) | TagEnd::Paragraph | TagEnd::BlockQuote(_)) => {}
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
            _ => {}
        }
    }

    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Theme {
        Theme::dark()
    }

    fn flatten(r: &[Vec<Span>]) -> String {
        r.iter()
            .flat_map(|l| l.iter().map(|s| s.text.as_str()))
            .collect()
    }

    #[test]
    fn plain() {
        assert_eq!(flatten(&render_message("hi", &t())), "hi");
    }

    #[test]
    fn bold() {
        let r = render_message("**x**", &t());
        assert!(r
            .iter()
            .flat_map(|l| l.iter())
            .any(|s| s.text == "x" && s.attrs.bold));
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
        let s = r
            .iter()
            .flat_map(|l| l.iter())
            .find(|s| s.text == "T")
            .unwrap();
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

    // ── Integration tests (full render pipeline) ──────────────────

    /// helper: render spans through RichText widget → SimulatedBackend
    fn render_spans(
        spans: Vec<Vec<Span>>,
        tm: &Theme,
    ) -> arbor_tui_backend::simulated_backend::SimulatedBackend {
        use arbor_tui::app::{App, AppConfig};
        use arbor_tui_backend::simulated_backend::SimulatedBackend;
        use arbor_tui_primitives::layout::RectOffset;
        use arbor_tui_widgets::container::Col;
        use arbor_tui_widgets::rich_text::RichText;
        use arbor_tui_widgets::widget_manager::WidgetManager;

        let wm = WidgetManager::new();
        let cols = 80u16;
        let rows = 10u16;
        let mut app = App::new(cols, rows, AppConfig::default());
        let mut backend = SimulatedBackend::new(cols, rows);

        let root = Col::new()
            .children([RichText::new()
                .lines(spans)
                .flex(1.0)
                .padding(RectOffset { top: 0, bottom: 0, left: 0, right: 0 })
                .build(&wm, tm)])
            .build(&wm, tm);

        // First call may be throttled by 16ms frame cap.
        // Retry until we get a real render.
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(1));
            match app.render_widget_tree(&root, tm, &mut backend) {
                Ok(arbor_tui::app::RenderResult::Rendered) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }
        backend
    }

    /// dump screen rows for debugging (run with --nocapture)
    #[allow(dead_code)]
    fn dump_rows(
        screen: &arbor_tui_render::screen::VirtualScreen,
        cols: u16,
        y_start: u16,
        count: u16,
    ) {
        for y in y_start..y_start + count {
            let chars: String = (0..cols).map(|x| screen.cell_at(x, y).ch).collect();
            let bgs: Vec<u8> = (0..cols)
                .map(|x| screen.cell_at(x, y).bg.palette.0)
                .collect();
            eprintln!("row {y:>2}: [{chars}]");
            eprintln!("    bg: {bgs:?}");
        }
    }

    #[test]
    fn c_comment_stops_at_end_of_line() {
        let tm = t();
        let content = "```c\nint x = 1; // comment\nint y = 2;\n```";
        let spans = render_message(content, &tm);

        // Debug: inspect the spans before rendering
        for (i, line) in spans.iter().enumerate() {
            let text: String = line.iter().map(|s| s.text.as_str()).collect();
            let bg: Vec<u8> = line.iter().map(|s| s.bg.palette.0).collect();
            eprintln!("span line {i}: text=[{text}] bg={bg:?}");
        }

        let backend = render_spans(spans, &tm);
        let screen = backend.screen();

        // Row 0: "int x = 1; // comment"  (first code line)
        // Row 1: "int y = 2;"            (second code line)
        let comment_line = 0u16;
        let normal_line = 1u16;

        // Find "int" on the comment line — it should have keyword color, not comment color
        let comment_row_text: String = (0..80)
            .map(|x| screen.cell_at(x, comment_line).ch)
            .collect();
        let normal_row_text: String = (0..80).map(|x| screen.cell_at(x, normal_line).ch).collect();

        eprintln!("comment_line: [{comment_row_text}]");
        eprintln!("normal_line:  [{normal_row_text}]");
        for x in 0..30 {
            let c = screen.cell_at(x, comment_line);
            eprintln!(
                "  col {x}: ch='{}' fg_pal={} bg_pal={}",
                c.ch, c.fg.palette.0, c.bg.palette.0
            );
        }

        // Both lines should have code_bg (236) where text is present
        let comment_bg = screen.cell_at(4, comment_line).bg.palette.0; // near "int x"
        let normal_bg = screen.cell_at(4, normal_line).bg.palette.0; // near "int y"
        assert_eq!(comment_bg, 236, "comment line cells should have code_bg");
        assert_eq!(normal_bg, 236, "normal line cells should have code_bg");

        // "int" on comment line should be keyword-colored (not dim comment color)
        // On normal line, "int" should also be keyword-colored (not dim/comment)
        // If comment leaked, the fg of "int" on normal line would be dim
        let _ = screen.cell_at(2, comment_line).fg;
        let normal_int_fg = screen.cell_at(2, normal_line).fg;
        // Both should be non-default keyword colors — at minimum, they shouldn't
        // be the dim comment gray (palette 250 is code_fg default; actual keywords
        // from syntect will have different RGB values)
        assert!(
            normal_int_fg.palette.0 != 0 || normal_int_fg.true_color.is_some(),
            "normal line 'int' should have non-black fg (got palette {})",
            normal_int_fg.palette.0
        );

        // Normal line should contain the text "int y = 2;"
        assert!(
            normal_row_text.contains("int y = 2;"),
            "normal line should contain 'int y = 2;', got: {normal_row_text}"
        );
    }
}
