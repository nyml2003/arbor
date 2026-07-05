use arbor_tui_primitives::layout::{LayoutProps, RectOffset};
use arbor_tui_widget::widget::WidgetNode;
use super::widget::{ColumnDef, TableWidget};
use crate::widget_factory::WidgetFactory;

pub struct Table {
    columns: Vec<ColumnDef>,
    cells: Vec<Vec<String>>,
    padding: RectOffset,
    flex: f32,
}

impl Table {
    pub fn new() -> Self { Self { columns: vec![], cells: vec![], padding: RectOffset::default(), flex: 0.0 } }
    pub fn columns(mut self, c: Vec<ColumnDef>) -> Self { self.columns = c; self }
    pub fn cells(mut self, c: Vec<Vec<String>>) -> Self { self.cells = c; self }
    pub fn padding(mut self, p: RectOffset) -> Self { self.padding = p; self }
    pub fn flex(mut self, f: f32) -> Self { self.flex = f; self }
    pub fn build(self, wm: &WidgetFactory, _t: &arbor_tui_render::theme::Theme) -> WidgetNode {
        wm.wrap(|id| TableWidget { id, props: LayoutProps { padding: self.padding, flex: self.flex, ..Default::default() }, columns: self.columns, cells: self.cells, selected: None, scroll_offset: 0, on_select: None, on_scroll: None, render_cell: None })
    }
}
