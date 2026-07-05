use super::widget::{ColumnDef, TableWidget};
use crate::widget_factory::WidgetFactory;
use arbor_tui_domain::layout::{LayoutProps, RectOffset};
use arbor_tui_domain::widget::WidgetNode;

pub struct Table {
    columns: Vec<ColumnDef>,
    cells: Vec<Vec<String>>,
    padding: RectOffset,
    flex: f32,
    on_select: Option<Box<dyn Fn(Option<usize>)>>,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: vec![],
            cells: vec![],
            padding: RectOffset::default(),
            flex: 0.0,
            on_select: None,
        }
    }

    pub fn columns(mut self, columns: Vec<ColumnDef>) -> Self {
        self.columns = columns;
        self
    }

    pub fn cells(mut self, cells: Vec<Vec<String>>) -> Self {
        self.cells = cells;
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

    pub fn on_select(mut self, f: impl Fn(Option<usize>) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    pub fn build(
        self,
        factory: &WidgetFactory,
        _theme: &arbor_tui_domain::theme::Theme,
    ) -> WidgetNode {
        factory.wrap(|id| TableWidget {
            id,
            props: LayoutProps {
                padding: self.padding,
                flex: self.flex,
                ..Default::default()
            },
            columns: self.columns,
            cells: self.cells,
            selected: None,
            scroll_offset: std::cell::Cell::new(0),
            viewport_rows: std::cell::Cell::new(1),
            on_select: self.on_select,
            on_scroll: None,
            render_cell: None,
        })
    }
}
