use std::io::IsTerminal;

use ratatui::style::{Color, Style};
use supports_color::{Stream, on};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorMode {
    None,
    Basic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconMode {
    Ascii,
    Unicode,
    Nerd,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTheme {
    pub color_mode: ColorMode,
    pub icon_mode: IconMode,
}

impl UiTheme {
    pub fn detect() -> Self {
        Self {
            color_mode: detect_color_mode(),
            icon_mode: detect_icon_mode(),
        }
    }

    pub fn supports_color(self) -> bool {
        self.color_mode != ColorMode::None
    }

    pub fn style(self, color: Color) -> Style {
        if self.supports_color() {
            Style::default().fg(color)
        } else {
            Style::default()
        }
    }

    pub fn icon(self, name: IconName) -> &'static str {
        match (self.icon_mode, name) {
            (IconMode::Nerd, IconName::Status) => "󰓥",
            (IconMode::Nerd, IconName::Battlefield) => "󰩈",
            (IconMode::Nerd, IconName::Action) => "󰆋",
            (IconMode::Nerd, IconName::Agent) => "󰭹",
            (IconMode::Nerd, IconName::Events) => "󰍩",
            (IconMode::Nerd, IconName::Help) => "󰞋",
            (IconMode::Nerd, IconName::Hp) => "󰊠",
            (IconMode::Nerd, IconName::Player) => "󰀘",
            (IconMode::Nerd, IconName::Opponent) => "󰀛",
            (IconMode::Unicode, IconName::Status) => "🧭",
            (IconMode::Unicode, IconName::Battlefield) => "⚔",
            (IconMode::Unicode, IconName::Action) => "🎮",
            (IconMode::Unicode, IconName::Agent) => "🤖",
            (IconMode::Unicode, IconName::Events) => "📜",
            (IconMode::Unicode, IconName::Help) => "❓",
            (IconMode::Unicode, IconName::Hp) => "❤",
            (IconMode::Unicode, IconName::Player) => "🙂",
            (IconMode::Unicode, IconName::Opponent) => "😈",
            (IconMode::Ascii, _) => "",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconName {
    Status,
    Battlefield,
    Action,
    Agent,
    Events,
    Help,
    Hp,
    Player,
    Opponent,
}

fn detect_color_mode() -> ColorMode {
    if std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal() {
        return ColorMode::None;
    }

    if let Some(level) = on(Stream::Stdout) {
        if level.has_basic {
            return ColorMode::Basic;
        }
    }

    ColorMode::None
}

fn detect_icon_mode() -> IconMode {
    match std::env::var("SHAMROCK_TUI_ICONS").ok().as_deref() {
        Some("ascii") => return IconMode::Ascii,
        Some("nerd") => return IconMode::Nerd,
        Some("unicode") | Some("emoji") => return IconMode::Unicode,
        _ => {}
    }

    if !std::io::stdout().is_terminal() {
        IconMode::Ascii
    } else {
        IconMode::Unicode
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorMode, IconMode, IconName, UiTheme};

    #[test]
    fn ascii_theme_returns_empty_icons() {
        let theme = UiTheme { color_mode: ColorMode::None, icon_mode: IconMode::Ascii };
        assert_eq!(theme.icon(IconName::Status), "");
    }

    #[test]
    fn unicode_theme_has_visible_help_icon() {
        let theme = UiTheme { color_mode: ColorMode::Basic, icon_mode: IconMode::Unicode };
        assert_eq!(theme.icon(IconName::Help), "❓");
    }
}
