use super::widget::StackWidget;
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_domain::widget::WidgetNode;

pub struct Col {
    children: Vec<WidgetNode>,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
    height: Option<u16>,
}

impl Default for Col {
    fn default() -> Self {
        Self::new()
    }
}

impl Col {
    pub fn new() -> Self {
        Self {
            children: vec![],
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
            height: None,
        }
    }

    pub fn children(mut self, kids: impl IntoIterator<Item = WidgetNode>) -> Self {
        self.children = kids.into_iter().collect();
        self
    }

    pub fn padding(mut self, p: RectOffset) -> Self {
        self.padding = p;
        self
    }

    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    pub fn size(mut self, w: u16, h: u16) -> Self {
        self.width = Some(w);
        self.height = Some(h);
        self
    }

    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| StackWidget {
            id,
            props: LayoutProps {
                direction: Direction::Column,
                padding: self.padding,
                flex: self.flex,
                width: self.width,
                height: self.height,
                ..Default::default()
            },
            children: self.children,
        })
    }
}

pub struct Row {
    children: Vec<WidgetNode>,
    padding: RectOffset,
    flex: f32,
    width: Option<u16>,
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Row {
    pub fn new() -> Self {
        Self {
            children: vec![],
            padding: RectOffset::default(),
            flex: 0.0,
            width: None,
        }
    }

    pub fn children(mut self, kids: impl IntoIterator<Item = WidgetNode>) -> Self {
        self.children = kids.into_iter().collect();
        self
    }

    pub fn padding(mut self, p: RectOffset) -> Self {
        self.padding = p;
        self
    }

    pub fn flex(mut self, f: f32) -> Self {
        self.flex = f;
        self
    }

    pub fn width(mut self, w: u16) -> Self {
        self.width = Some(w);
        self
    }

    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| StackWidget {
            id,
            props: LayoutProps {
                direction: Direction::Row,
                padding: self.padding,
                flex: self.flex,
                width: self.width,
                ..Default::default()
            },
            children: self.children,
        })
    }
}
