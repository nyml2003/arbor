#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorToken {
    Surface,
    Border,
    TextPrimary,
    TextMuted,
    Button,
    ButtonHovered,
    ButtonPressed,
    ButtonActive,
    ButtonDisabled,
    Ripple,
}

impl ColorToken {
    pub fn color(self) -> Color {
        match self {
            ColorToken::Surface => Color::rgba(0.10, 0.11, 0.13, 0.96),
            ColorToken::Border => Color::rgba(0.32, 0.34, 0.38, 1.0),
            ColorToken::TextPrimary => Color::rgba(0.94, 0.95, 0.97, 1.0),
            ColorToken::TextMuted => Color::rgba(0.68, 0.71, 0.76, 1.0),
            ColorToken::Button => Color::rgba(0.20, 0.22, 0.26, 1.0),
            ColorToken::ButtonHovered => Color::rgba(0.27, 0.30, 0.35, 1.0),
            ColorToken::ButtonPressed => Color::rgba(0.12, 0.37, 0.58, 1.0),
            ColorToken::ButtonActive => Color::rgba(0.05, 0.45, 0.65, 1.0),
            ColorToken::ButtonDisabled => Color::rgba(0.16, 0.17, 0.20, 1.0),
            ColorToken::Ripple => Color::rgba(0.70, 0.88, 1.0, 1.0),
        }
    }
}
