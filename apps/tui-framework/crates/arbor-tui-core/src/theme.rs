// Theme system — global singleton mapping semantic colors to AnsiColor.

use crate::cell::{AnsiColor, PaletteColor};

/// Global theme singleton.
///
/// Components receive `&Theme` through their render method.
/// v1: global singleton. Multi-instance support is deferred.
#[derive(Clone)]
pub struct Theme {
    /// Color palette: semantic name → 256-color index.
    colors: ThemeColors,
    /// Whether this is a light or dark theme.
    pub variant: ThemeVariant,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ThemeVariant {
    Dark,
    Light,
    HighContrast,
}

#[derive(Clone)]
struct ThemeColors {
    surface: PaletteColor,
    surface_alt: PaletteColor,
    text: PaletteColor,
    text_dim: PaletteColor,
    primary: PaletteColor,
    secondary: PaletteColor,
    danger: PaletteColor,
    success: PaletteColor,
    warning: PaletteColor,
    border: PaletteColor,
    accent: PaletteColor,
}

impl Theme {
    /// Dark theme — default.
    pub fn dark() -> Self {
        Self {
            variant: ThemeVariant::Dark,
            colors: ThemeColors {
                surface: PaletteColor(0),      // black
                surface_alt: PaletteColor(8),  // dark gray
                text: PaletteColor(7),         // white
                text_dim: PaletteColor(8),     // dark gray
                primary: PaletteColor(12),     // blue
                secondary: PaletteColor(8),    // gray
                danger: PaletteColor(9),       // red
                success: PaletteColor(10),     // green
                warning: PaletteColor(11),     // yellow
                border: PaletteColor(8),       // gray
                accent: PaletteColor(14),      // cyan
            },
        }
    }

    /// Light theme.
    pub fn light() -> Self {
        Self {
            variant: ThemeVariant::Light,
            colors: ThemeColors {
                surface: PaletteColor(7),      // white
                surface_alt: PaletteColor(15), // light gray
                text: PaletteColor(0),         // black
                text_dim: PaletteColor(8),     // dark gray
                primary: PaletteColor(4),      // dark blue
                secondary: PaletteColor(8),
                danger: PaletteColor(1),       // red
                success: PaletteColor(2),      // green
                warning: PaletteColor(3),      // yellow
                border: PaletteColor(8),
                accent: PaletteColor(6),       // cyan
            },
        }
    }

    /// High-contrast variant — for accessibility.
    pub fn high_contrast() -> Self {
        Self {
            variant: ThemeVariant::HighContrast,
            colors: ThemeColors {
                surface: PaletteColor(0),
                surface_alt: PaletteColor(0),
                text: PaletteColor(15),        // bright white
                text_dim: PaletteColor(7),     // white
                primary: PaletteColor(12),     // bright blue
                secondary: PaletteColor(7),
                danger: PaletteColor(9),       // bright red
                success: PaletteColor(10),     // bright green
                warning: PaletteColor(11),     // bright yellow
                border: PaletteColor(7),
                accent: PaletteColor(14),
            },
        }
    }

    // Semantic color accessors

    pub fn surface(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.surface, true_color: None }
    }

    pub fn surface_alt(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.surface_alt, true_color: None }
    }

    pub fn text(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.text, true_color: None }
    }

    pub fn text_dim(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.text_dim, true_color: None }
    }

    pub fn primary(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.primary, true_color: None }
    }

    pub fn danger(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.danger, true_color: None }
    }

    pub fn success(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.success, true_color: None }
    }

    pub fn warning(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.warning, true_color: None }
    }

    pub fn secondary(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.secondary, true_color: None }
    }

    pub fn border(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.border, true_color: None }
    }

    pub fn accent(&self) -> AnsiColor {
        AnsiColor { palette: self.colors.accent, true_color: None }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
