// TableWidget — columnar data table with header row.

use arbor_tui_primitives::cell::{Attrs, Cell};
use arbor_tui_primitives::input::KeyHandleResult;
use arbor_tui_primitives::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_primitives::text::{self, TruncateStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetAction, WidgetId};

pub struct TableWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub columns: Vec<ColumnDef>,
    pub cells: Vec<Vec<String>>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub on_select: Option<Box<dyn Fn(Option<usize>)>>,
    pub on_scroll: Option<Box<dyn Fn(usize)>>,
    pub render_cell: Option<Box<dyn Fn(usize, usize) -> String>>,
}

pub struct ColumnDef {
    pub header: String,
    pub width: ColumnWidth,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColumnWidth {
    Fixed(u16),
    Flex(f32),
}

impl Widget for TableWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn focusable(&self) -> bool { true }

    fn measure(&self, available: Size) -> SizeConstraint {
        let avail = arbor_tui_primitives::layout::SizeCalc::content_available(
            available, self.props.padding, self.props.margin,
        );
        SizeConstraint {
            min_w: 1,
            min_h: 1,
            max_w: arbor_tui_primitives::layout::AxisConstraint::Fixed(avail.w.max(1)),
            max_h: arbor_tui_primitives::layout::AxisConstraint::Fixed(avail.h.max(1)),
        }
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
        let bg = theme.surface();
        let header_bg = theme.surface_alt();
        let border_fg = theme.border();
        let text = theme.text();
        let accent = theme.accent();

        let bg_cell = Cell { bg, ..Default::default() };
        screen.fill_rect(Rect::new(0, 0, rect.w, rect.h), &bg_cell);
        if rect.h == 0 { return screen; }

        // Header row
        let mut col_x: u16 = 0;
        for (ci, col) in self.columns.iter().enumerate() {
            let col_w = resolve_col_width(col.width, rect.w, &self.columns, ci);
            let header_text = text::truncate(&col.header, col_w, TruncateStrategy::End);
            let hdr_cell = Cell { bg: header_bg, ..Default::default() };
            screen.fill_rect(Rect::new(col_x, 0, col_w, 1), &hdr_cell);
            screen.write_str(col_x, 0, &header_text, text, header_bg, Attrs { bold: true, ..Default::default() });
            col_x += col_w;
        }

        // Separator
        let sep_cell = Cell { bg: border_fg, ..Default::default() };
        screen.fill_rect(Rect::new(0, 1, rect.w, 1), &sep_cell);

        // Data rows
        let data_start: u16 = 2;
        let visible_rows = (rect.h.saturating_sub(data_start)) as usize;
        let start = self.scroll_offset;
        let end = (start + visible_rows).min(self.cells.len());

        for (i, row_idx) in (start..end).enumerate() {
            let screen_row = data_start + i as u16;
            let is_selected = self.selected == Some(row_idx);
            let row_bg = if is_selected { accent } else { bg };
            let row_fg = if is_selected { theme.surface() } else { text };

            let mut cx: u16 = 0;
            for ci in 0..self.columns.len() {
                let col_w = resolve_col_width(self.columns[ci].width, rect.w, &self.columns, ci);
                let cell_text = if let Some(ref render) = self.render_cell {
                    render(row_idx, ci)
                } else if ci < self.cells.get(row_idx).map_or(0, |r| r.len()) {
                    self.cells[row_idx][ci].clone()
                } else {
                    String::new()
                };
                let display = text::truncate(&cell_text, col_w, TruncateStrategy::End);
                screen.write_str(cx, screen_row, &display, row_fg, row_bg, Attrs::default());
                cx += col_w;
            }
        }
        screen
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        let old = self.selected;
        match action {
            WidgetAction::NavigateDown => {
                let max = self.cells.len().saturating_sub(1);
                self.selected = Some(self.selected.map_or(0, |s| (s + 1).min(max)));
            }
            WidgetAction::NavigateUp => {
                if let Some(s) = self.selected {
                    if s > 0 { self.selected = Some(s - 1); }
                }
            }
            _ => return KeyHandleResult::Bubble,
        }
        if self.selected != old {
            if let Some(ref cb) = self.on_select { cb(self.selected); }
        }
        KeyHandleResult::Handled
    }
}

fn resolve_col_width(col: ColumnWidth, total_w: u16, all_cols: &[ColumnDef], _idx: usize) -> u16 {
    match col {
        ColumnWidth::Fixed(w) => w,
        ColumnWidth::Flex(_) => {
            let fixed_total: u16 = all_cols.iter().filter_map(|c| match c.width {
                ColumnWidth::Fixed(w) => Some(w),
                _ => None,
            }).sum();
            let flex_count = all_cols.iter().filter(|c| matches!(c.width, ColumnWidth::Flex(_))).count() as u16;
            let remaining = total_w.saturating_sub(fixed_total);
            if flex_count > 0 { remaining / flex_count } else { 0 }
        }
    }
}
