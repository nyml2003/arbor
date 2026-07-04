/// 256-color palette index (0-255).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct PaletteColor(pub u8);

impl Default for PaletteColor {
    /// Default foreground: white (index 7)
    fn default() -> Self {
        PaletteColor(7)
    }
}

impl PaletteColor {
    pub const BLACK: PaletteColor = PaletteColor(0);
    pub const WHITE: PaletteColor = PaletteColor(7);
}

/// ANSI color — 256-color palette is primary, TrueColor RGB is optional attachment.
///
/// When the terminal does not support 24-bit RGB, the `true_color` field is
/// silently dropped and only the palette index is emitted. The framework does
/// NOT perform RGB→256 mapping — that responsibility belongs to the theme layer.
#[derive(Copy, Clone, Debug)]
pub struct AnsiColor {
    pub palette: PaletteColor,
    pub true_color: Option<Rgb>,
}

impl PartialEq for AnsiColor {
    fn eq(&self, other: &Self) -> bool {
        self.palette == other.palette
    }
}

impl Eq for AnsiColor {}

impl Default for AnsiColor {
    fn default() -> Self {
        Self {
            palette: PaletteColor::default(),
            true_color: None,
        }
    }
}

impl AnsiColor {
    pub const fn from_palette(index: u8) -> Self {
        Self {
            palette: PaletteColor(index),
            true_color: None,
        }
    }

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self {
            palette: PaletteColor(7), // fallback
            true_color: Some(Rgb(r, g, b)),
        }
    }
}

/// 24-bit RGB color.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Character attributes stored as bitflags-compatible booleans.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Attrs {
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
}

impl Default for Attrs {
    fn default() -> Self {
        Self {
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            reverse: false,
        }
    }
}

/// A single character cell on the terminal grid.
///
/// Derives `PartialEq` so the diff algorithm can directly compare cells.
/// `true_color` is excluded from equality — only `palette` matters for
/// determining whether a cell has changed.
/// `phantom` is excluded from equality — it only affects backend emission.
#[derive(Clone, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
    /// When true, this cell is the second column of a wide (CJK) character
    /// and should be skipped during backend emission.
    pub phantom: bool,
}

// Manual PartialEq — phantom excluded so diff doesn't flag phantom cells as changed
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.ch == other.ch
            && self.fg == other.fg
            && self.bg == other.bg
            && self.attrs == other.attrs
        // phantom intentionally excluded
    }
}

impl Default for Cell {
    /// Default blank cell: space, white foreground, black background, no attributes.
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: AnsiColor::default(),
            bg: AnsiColor {
                palette: PaletteColor(0),
                true_color: None,
            },
            attrs: Attrs::default(),
            phantom: false,
        }
    }
}

impl Cell {
    pub fn new(ch: char) -> Self {
        Self { ch, ..Default::default() }
    }

    pub fn with_fg(mut self, fg: AnsiColor) -> Self {
        self.fg = fg;
        self
    }

    pub fn with_bg(mut self, bg: AnsiColor) -> Self {
        self.bg = bg;
        self
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.attrs.bold = bold;
        self
    }
}

/// A styled text fragment — the building block for rich text.
/// Multiple Spans concatenate to form a single line with inline styling.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}

impl Span {
    pub fn new(text: impl Into<String>, fg: AnsiColor, bg: AnsiColor, attrs: Attrs) -> Self {
        Self { text: text.into(), fg, bg, attrs }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self { text: text.into(), fg: AnsiColor::default(), bg: AnsiColor::default(), attrs: Attrs::default() }
    }

    pub fn bold(text: impl Into<String>) -> Self {
        Self { text: text.into(), fg: AnsiColor::default(), bg: AnsiColor::default(), attrs: Attrs { bold: true, ..Default::default() } }
    }

    pub fn italic(text: impl Into<String>) -> Self {
        Self { text: text.into(), fg: AnsiColor::default(), bg: AnsiColor::default(), attrs: Attrs { italic: true, ..Default::default() } }
    }

    pub fn with_fg(mut self, fg: AnsiColor) -> Self { self.fg = fg; self }
    pub fn with_bg(mut self, bg: AnsiColor) -> Self { self.bg = bg; self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_blank() {
        let c = Cell::default();
        assert_eq!(c.ch, ' ');
        assert_eq!(c.fg.palette.0, 7);
        assert_eq!(c.bg.palette.0, 0);
        assert!(!c.attrs.bold);
    }

    #[test]
    fn cell_equality_ignores_truecolor() {
        let a = Cell { fg: AnsiColor::from_palette(1), ..Default::default() };
        let b = Cell { fg: AnsiColor::from_rgb(255, 0, 0), ..Default::default() };
        // palette indices differ (1 vs 7), so they are NOT equal
        assert_ne!(a, b);

        let c = Cell { fg: AnsiColor::from_palette(7), ..Default::default() };
        let d = Cell { fg: AnsiColor::from_rgb(255, 0, 0), ..Default::default() };
        // same palette index (7), true_color ignored → equal
        assert_eq!(c, d);
    }
}
