// Text measurement, tab expansion, truncation, and wrapping.
// All pure functions — no I/O, no terminal dependency.

use unicode_width::UnicodeWidthStr;

/// Tab stop width in columns.
const TAB_WIDTH: u16 = 4;

/// Measure the display width of a string in terminal columns.
/// CJK characters count as 2, ASCII as 1, combining marks as 0.
pub fn measure_width(text: &str) -> u16 {
    text.width() as u16
}

/// Column offset after the first `n` characters of `text`.
/// CJK-aware — a CJK char contributes 2 columns.
pub fn column_offset(text: &str, n: usize) -> u16 {
    text.chars()
        .take(n)
        .map(|ch| unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1) as u16)
        .sum()
}

/// Expand `\t` to spaces, aligning to the next tab stop (multiples of 4).
/// Must be called before `measure_width`, `truncate`, or `wrap_lines` —
/// the terminal grid never contains raw `'\t'` characters.
pub fn expand_tabs(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut col: u16 = 0;

    for ch in text.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (col % TAB_WIDTH);
            for _ in 0..spaces {
                result.push(' ');
            }
            col += spaces;
        } else {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
            col += w;
            result.push(ch);
        }
    }

    result
}

/// Truncation strategy.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TruncateStrategy {
    /// "hello w…"
    End,
    /// "/usr/…/file.txt"
    Middle,
    /// Overflow into adjacent cells (no truncation)
    None,
}

/// Truncate text to fit within `max_width` columns, applying the given strategy.
/// The text must already have tabs expanded.
pub fn truncate(text: &str, max_width: u16, strategy: TruncateStrategy) -> String {
    if max_width == 0 {
        return String::new();
    }

    let full_width = measure_width(text);
    if full_width <= max_width {
        return text.to_string();
    }

    match strategy {
        TruncateStrategy::None => text.to_string(),
        TruncateStrategy::End => {
            let ellipsis = "…";
            let ellipsis_w = measure_width(ellipsis);
            if max_width <= ellipsis_w {
                return ellipsis.to_string();
            }
            let avail = max_width - ellipsis_w;
            let mut result = String::with_capacity(max_width as usize);
            let mut w: u16 = 0;
            for ch in text.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                if w + cw > avail {
                    break;
                }
                result.push(ch);
                w += cw;
            }
            result.push_str(ellipsis);
            result
        }
        TruncateStrategy::Middle => {
            let ellipsis = "…";
            let ellipsis_w = measure_width(ellipsis);
            if max_width <= ellipsis_w {
                return ellipsis.to_string();
            }
            let avail = max_width - ellipsis_w;
            let left_w = avail / 2;
            let right_w = avail - left_w;

            let mut left = String::new();
            let mut w: u16 = 0;
            for ch in text.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                if w + cw > left_w {
                    break;
                }
                left.push(ch);
                w += cw;
            }

            let mut right = String::new();
            let mut w: u16 = 0;
            for ch in text.chars().rev() {
                let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;
                if w + cw > right_w {
                    break;
                }
                right.push(ch);
                w += cw;
            }

            format!(
                "{}{}{}",
                left,
                ellipsis,
                right.chars().rev().collect::<String>()
            )
        }
    }
}

/// Word wrapping strategy.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WrapStrategy {
    /// No wrapping — text stays on one line, truncated if needed.
    None,
    /// Break at word boundaries (spaces for Latin text).
    /// CJK text falls back to Char since there are no spaces.
    Word,
    /// Break at any character boundary.
    Char,
}

/// Wrap text into lines, each ≤ `max_width` columns wide.
/// The text must already have tabs expanded.
/// Returns a vector of lines.
pub fn wrap_lines(text: &str, max_width: u16, strategy: WrapStrategy) -> Vec<String> {
    if max_width == 0 {
        return vec![];
    }

    match strategy {
        WrapStrategy::None => vec![text.to_string()],
        WrapStrategy::Char => wrap_char(text, max_width),
        WrapStrategy::Word => wrap_word(text, max_width),
    }
}

fn wrap_char(text: &str, max_width: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w: u16 = 0;

    for ch in text.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) as u16;

        if current_w + cw > max_width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_w = 0;
        }

        // If a single character > max_width, it still gets its own line
        current.push(ch);
        current_w += cw;
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn wrap_word(text: &str, max_width: u16) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_w: u16 = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_w = measure_width(word);

        // If the word itself exceeds max_width, fall back to character wrapping
        if word_w > max_width {
            // Flush current line
            if !current_line.is_empty() {
                lines.push(std::mem::take(&mut current_line));
                current_w = 0;
            }
            // Char-wrap this overlong word
            lines.extend(wrap_char(word, max_width));
            continue;
        }

        if current_w + word_w > max_width {
            lines.push(std::mem::take(&mut current_line));
            current_w = 0;
        }

        current_line.push_str(word);
        current_w += word_w;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_ascii() {
        assert_eq!(measure_width("hello"), 5);
    }

    #[test]
    fn measure_cjk() {
        assert_eq!(measure_width("你好"), 4);
    }

    #[test]
    fn measure_mixed() {
        assert_eq!(measure_width("hello你好"), 9);
    }

    #[test]
    fn expand_tabs_replaces_with_spaces() {
        let expanded = expand_tabs("a\tb");
        assert_eq!(expanded, "a   b"); // 1 + 3 spaces to reach col 4
        assert_eq!(measure_width(&expanded), 5);
    }

    #[test]
    fn expand_tabs_at_boundary() {
        let expanded = expand_tabs("abcd\tx");
        assert_eq!(expanded, "abcd    x"); // 4 + 4 spaces to reach col 8
    }

    #[test]
    fn truncate_end_short_text() {
        assert_eq!(truncate("hello", 10, TruncateStrategy::End), "hello");
    }

    #[test]
    fn truncate_end_long_text() {
        let result = truncate("hello world", 8, TruncateStrategy::End);
        assert!(measure_width(&result) <= 8);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_none_no_cut() {
        assert_eq!(truncate("hello", 2, TruncateStrategy::None), "hello");
    }

    #[test]
    fn wrap_none_single_line() {
        let lines = wrap_lines("hello world", 5, WrapStrategy::None);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn wrap_char_splits_anywhere() {
        let lines = wrap_lines("abcdef", 3, WrapStrategy::Char);
        assert_eq!(lines, vec!["abc", "def"]);
    }

    #[test]
    fn wrap_word_splits_on_spaces() {
        let lines = wrap_lines("hello world rust", 7, WrapStrategy::Word);
        // "hello " = 6, "world " = 6, "rust" = 4
        // "hello " + "world " = 12 > 7, so break after "hello "
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn truncate_middle_long_path() {
        let path = "/usr/local/bin/myapp";
        let result = truncate(path, 12, TruncateStrategy::Middle);
        assert!(measure_width(&result) <= 12);
        assert!(result.contains('…'));
    }
}
