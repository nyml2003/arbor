#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    #[default]
    Default,
    Black,
    Gray,
    White,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Rgb {
        red: u8,
        green: u8,
        blue: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TerminalCell {
    symbol: char,
    foreground: TerminalColor,
    background: TerminalColor,
}

impl TerminalCell {
    pub const fn new(symbol: char, foreground: TerminalColor, background: TerminalColor) -> Self {
        Self {
            symbol,
            foreground,
            background,
        }
    }

    pub const fn symbol(self) -> char {
        self.symbol
    }

    pub const fn foreground(self) -> TerminalColor {
        self.foreground
    }

    pub const fn background(self) -> TerminalColor {
        self.background
    }
}

impl Default for TerminalCell {
    fn default() -> Self {
        Self::new(' ', TerminalColor::Default, TerminalColor::Default)
    }
}
