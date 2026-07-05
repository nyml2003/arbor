// ListWidget — scrollable item list.

use arbor_tui_primitives::cell::{Attrs, Cell};
use arbor_tui_primitives::input::KeyHandleResult;
use arbor_tui_primitives::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_primitives::text::{self, TruncateStrategy};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetAction, WidgetId};

pub struct ListWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub items: Vec<String>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub on_select: Option<Box<dyn Fn(Option<usize>)>>,
    pub on_scroll: Option<Box<dyn Fn(usize)>>,
    pub render_item: Option<Box<dyn Fn(usize, bool) -> String>>,
}

impl Widget for ListWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }
    fn focusable(&self) -> bool {
        true
    }

    fn measure(&self, available: Size) -> SizeConstraint {
        let avail = arbor_tui_primitives::layout::SizeCalc::content_available(
            available,
            self.props.padding,
            self.props.margin,
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
        let accent = theme.accent();
        let text = theme.text();

        let bg_cell = Cell {
            bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w, rect.h), &bg_cell);

        let visible_count = rect.h as usize;
        let start = self.scroll_offset;
        let end = (start + visible_count).min(self.items.len());

        for (i, item_idx) in (start..end).enumerate() {
            let row = i as u16;
            let is_selected = self.selected == Some(item_idx);
            let (fg, row_bg) = if is_selected {
                (theme.surface(), accent)
            } else {
                (text, bg)
            };

            if is_selected {
                let sel_cell = Cell {
                    bg: row_bg,
                    ..Default::default()
                };
                screen.fill_rect(Rect::new(0, row, rect.w, 1), &sel_cell);
            }

            let item_text = if let Some(ref render) = self.render_item {
                render(item_idx, is_selected)
            } else {
                self.items[item_idx].clone()
            };
            let display = text::truncate(&item_text, rect.w, TruncateStrategy::End);
            screen.write_str(1, row, &display, fg, row_bg, Attrs::default());
        }

        // Scroll indicator
        if self.items.len() > visible_count {
            let pct = start as f64 / self.items.len() as f64;
            let bar_y = (pct * (rect.h - 1) as f64) as u16;
            if let Some(cell) = screen.cell_at_mut(rect.w.saturating_sub(1), bar_y) {
                cell.fg = accent;
                cell.bg = accent;
                cell.ch = ' ';
            }
        }
        screen
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        let old = self.selected;
        match action {
            WidgetAction::NavigateDown => {
                let max = self.items.len().saturating_sub(1);
                self.selected = Some(self.selected.map_or(0, |s| (s + 1).min(max)));
            }
            WidgetAction::NavigateUp => {
                if let Some(s) = self.selected {
                    if s > 0 {
                        self.selected = Some(s - 1);
                    }
                }
            }
            _ => return KeyHandleResult::Bubble,
        }
        if self.selected != old {
            if let Some(ref cb) = self.on_select {
                cb(self.selected);
            }
        }
        KeyHandleResult::Handled
    }
}
