use crate::usize_to_u16_saturating;
use arbor_tui_domain::layout::RectOffset;
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_widgets::scroll::Scroll;
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::widget_factory::WidgetFactory;

/// A widget paired with its natural content height in terminal rows.
pub struct ContentBlock {
    widget: WidgetNode,
    line_count: usize,
}

impl ContentBlock {
    pub fn new(widget: WidgetNode, line_count: usize) -> Self {
        Self { widget, line_count }
    }

    pub fn line_count(&self) -> usize {
        self.line_count
    }

    pub fn into_widget(self) -> WidgetNode {
        self.widget
    }
}

/// A vertically stacked scroll area whose content height is derived from blocks.
pub struct ScrollColumn {
    blocks: Vec<ContentBlock>,
    scroll_y: Option<ReadSignal<u16>>,
    padding: RectOffset,
    flex: f32,
}

impl Default for ScrollColumn {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollColumn {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            scroll_y: None,
            padding: RectOffset::default(),
            flex: 0.0,
        }
    }

    pub fn blocks(mut self, blocks: impl IntoIterator<Item = ContentBlock>) -> Self {
        self.blocks = blocks.into_iter().collect();
        self
    }

    pub fn scroll_y(mut self, signal: ReadSignal<u16>) -> Self {
        self.scroll_y = Some(signal);
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
        let content_h = usize_to_u16_saturating(
            self.blocks
                .iter()
                .map(ContentBlock::line_count)
                .sum::<usize>(),
        );
        let children = self
            .blocks
            .into_iter()
            .map(ContentBlock::into_widget)
            .collect::<Vec<_>>();
        let column = Col::new().children(children).build(factory, theme);
        let mut scroll = Scroll::new()
            .padding(self.padding)
            .flex(self.flex)
            .content_h(content_h)
            .child(column);
        if let Some(scroll_y) = self.scroll_y {
            scroll = scroll.scroll_y(scroll_y);
        }
        scroll.build(factory, theme)
    }
}
