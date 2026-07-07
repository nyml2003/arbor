use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Palette(u8),
    Rgb { r: u8, g: u8, b: u8, fallback: u8 },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Surface,
    SurfaceAlt,
    Text,
    TextMuted,
    Border,
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Focus,
    Selection,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ColorSource {
    Token(Token),
    Color(Color),
}

impl From<Token> for ColorSource {
    fn from(value: Token) -> Self {
        Self::Token(value)
    }
}

impl From<Color> for ColorSource {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

#[derive(Clone, Debug)]
pub struct Theme {
    colors: HashMap<Token, Color>,
}

impl Theme {
    pub fn dark() -> Self {
        Self::from_pairs([
            (Token::Surface, Color::Palette(0)),
            (Token::SurfaceAlt, Color::Palette(236)),
            (Token::Text, Color::Palette(252)),
            (Token::TextMuted, Color::Palette(244)),
            (Token::Border, Color::Palette(240)),
            (Token::Primary, Color::Palette(75)),
            (Token::Accent, Color::Palette(73)),
            (Token::Success, Color::Palette(78)),
            (Token::Warning, Color::Palette(220)),
            (Token::Danger, Color::Palette(203)),
            (Token::Focus, Color::Palette(63)),
            (Token::Selection, Color::Palette(238)),
        ])
    }

    pub fn light() -> Self {
        Self::from_pairs([
            (Token::Surface, Color::Palette(15)),
            (Token::SurfaceAlt, Color::Palette(254)),
            (Token::Text, Color::Palette(16)),
            (Token::TextMuted, Color::Palette(244)),
            (Token::Border, Color::Palette(250)),
            (Token::Primary, Color::Palette(25)),
            (Token::Accent, Color::Palette(31)),
            (Token::Success, Color::Palette(28)),
            (Token::Warning, Color::Palette(136)),
            (Token::Danger, Color::Palette(160)),
            (Token::Focus, Color::Palette(153)),
            (Token::Selection, Color::Palette(189)),
        ])
    }

    pub fn high_contrast() -> Self {
        Self::from_pairs([
            (Token::Surface, Color::Palette(0)),
            (Token::SurfaceAlt, Color::Palette(8)),
            (Token::Text, Color::Palette(15)),
            (Token::TextMuted, Color::Palette(7)),
            (Token::Border, Color::Palette(15)),
            (Token::Primary, Color::Palette(14)),
            (Token::Accent, Color::Palette(11)),
            (Token::Success, Color::Palette(10)),
            (Token::Warning, Color::Palette(11)),
            (Token::Danger, Color::Palette(9)),
            (Token::Focus, Color::Palette(14)),
            (Token::Selection, Color::Palette(4)),
        ])
    }

    pub fn with(mut self, token: Token, color: Color) -> Self {
        self.colors.insert(token, color);
        self
    }

    pub fn resolve(&self, source: impl Into<ColorSource>) -> Color {
        match source.into() {
            ColorSource::Color(color) => color,
            ColorSource::Token(token) => self
                .colors
                .get(&token)
                .copied()
                .unwrap_or(Color::Palette(0)),
        }
    }

    fn from_pairs(pairs: impl IntoIterator<Item = (Token, Color)>) -> Self {
        Self {
            colors: pairs.into_iter().collect(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_resolve_tokens() {
        assert_eq!(Theme::dark().resolve(Token::Surface), Color::Palette(0));
        assert_eq!(Theme::light().resolve(Token::Surface), Color::Palette(15));
        assert_eq!(
            Theme::high_contrast().resolve(Token::Text),
            Color::Palette(15)
        );
    }

    #[test]
    fn token_override_and_concrete_color_work() {
        let theme = Theme::dark().with(Token::Accent, Color::Palette(99));
        assert_eq!(theme.resolve(Token::Accent), Color::Palette(99));
        assert_eq!(
            theme.resolve(Color::Rgb {
                r: 1,
                g: 2,
                b: 3,
                fallback: 4
            }),
            Color::Rgb {
                r: 1,
                g: 2,
                b: 3,
                fallback: 4
            }
        );
    }
}
