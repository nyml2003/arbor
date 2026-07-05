use arbor_tui_domain::cell::AnsiColor;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use super::widget::{SectionedPanelStyle, SectionedPanelWidget};
use crate::SectionedPanelSection;

/// A rounded panel that connects multiple text sections with border dividers.
pub struct SectionedPanel {
    sections: Vec<SectionedPanelSection>,
    fg: Option<AnsiColor>,
    bg: Option<AnsiColor>,
    title_fg: Option<AnsiColor>,
    text_fg: Option<AnsiColor>,
    width: Option<u16>,
    height: Option<u16>,
    padding: RectOffset,
    flex: f32,
}

impl SectionedPanel {
    pub fn new(sections: impl IntoIterator<Item = SectionedPanelSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            fg: None,
            bg: None,
            title_fg: None,
            text_fg: None,
            width: None,
            height: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }

    pub fn section(mut self, section: SectionedPanelSection) -> Self {
        self.sections.push(section);
        self
    }

    pub fn fg(mut self, color: AnsiColor) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: AnsiColor) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn title_fg(mut self, color: AnsiColor) -> Self {
        self.title_fg = Some(color);
        self
    }

    pub fn text_fg(mut self, color: AnsiColor) -> Self {
        self.text_fg = Some(color);
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width.max(4));
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height.max(2));
        self
    }

    pub fn padding(mut self, padding: RectOffset) -> Self {
        self.padding = padding;
        self
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    pub fn build(self, factory: &WidgetFactory, theme: &Theme) -> WidgetNode {
        let fg = self.fg.unwrap_or_else(|| theme.border());
        let bg = self.bg.unwrap_or_else(|| theme.surface());
        factory.wrap(|id| SectionedPanelWidget {
            id,
            props: LayoutProps {
                flex: self.flex,
                width: self.width,
                height: self.height,
                padding: self.padding,
                ..Default::default()
            },
            sections: self.sections,
            style: SectionedPanelStyle {
                border_fg: fg,
                bg,
                title_fg: self.title_fg.unwrap_or_else(|| theme.text_dim()),
                text_fg: self.text_fg.unwrap_or_else(|| theme.text()),
            },
        })
    }
}
