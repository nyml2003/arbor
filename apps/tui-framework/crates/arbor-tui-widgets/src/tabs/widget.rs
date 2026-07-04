// TabsWidget — tabbed container. Reserves 2 rows for header + separator.

use arbor_tui_primitives::cell::Cell;
use arbor_tui_primitives::input::KeyHandleResult;
use arbor_tui_primitives::layout::{LayoutProps, Rect, RectOffset, Size, SizeCalc, SizeConstraint};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_primitives::text::{self};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetAction, WidgetId, WidgetNode};

use std::collections::HashMap;

pub struct TabsWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub tabs: Vec<TabDef>,
    pub active: usize,
    pub on_switch: Option<Box<dyn Fn(usize)>>,
}

pub struct TabDef {
    pub label: String,
    pub content: WidgetNode,
}

impl Widget for TabsWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn focusable(&self) -> bool { true }

    fn children(&self) -> &[WidgetNode] {
        if self.tabs.is_empty() {
            &[]
        } else {
            let idx = self.active.min(self.tabs.len() - 1);
            std::slice::from_ref(&self.tabs[idx].content)
        }
    }

    fn children_mut(&mut self) -> &mut [WidgetNode] {
        if self.tabs.is_empty() {
            &mut []
        } else {
            let idx = self.active.min(self.tabs.len() - 1);
            std::slice::from_mut(&mut self.tabs[idx].content)
        }
    }

    /// Reserve 2 rows for tab header + separator.
    fn children_rect(&self, content_rect: Rect) -> Rect {
        let h = content_rect.h.saturating_sub(2);
        Rect::new(content_rect.x, content_rect.y + 2, content_rect.w, h)
    }

    fn on_mount(&mut self) {}

    fn measure_subtree(
        &self,
        available: Size,
        child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        let _ = SizeCalc::content_available(available, self.props.padding, RectOffset::default());
        let header_h: u16 = 2; // header row + separator
        let mut max_w: u16 = 0;
        let mut max_h: u16 = 0;

        for tab in &self.tabs {
            // Check if tab's child was measured (it should be, through children())
            if let Some(cc) = child_constraints.get(&tab.content.id()) {
                max_w = max_w.max(cc.min_w);
                max_h = max_h.max(cc.min_h);
            }
        }

        SizeConstraint {
            min_w: self.props.width.unwrap_or(max_w).max(1),
            min_h: self.props.height.unwrap_or(header_h + max_h).max(1),
            max_w: arbor_tui_primitives::layout::AxisConstraint::Fixed(available.w),
            max_h: arbor_tui_primitives::layout::AxisConstraint::Fixed(available.h),
        }
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
        let tab_bg = theme.surface_alt();
        let active_bg = theme.primary();
        let text = theme.text();
        let active_text = theme.surface();

        // 先用背景色填充整个区域（含子组件区域），避免 Cell::default() 黑底覆盖父组件。
        let fill = Cell { bg: theme.surface(), ..Default::default() };
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), rect.h.max(1)), &fill);

        // Tab headers
        let mut cx: u16 = 0;
        for (i, tab) in self.tabs.iter().enumerate() {
            let label = format!(" {} ", tab.label);
            let label_w = text::measure_width(&label);
            let is_active = i == self.active;
            let (fg, row_bg) = if is_active { (active_text, active_bg) } else { (text, tab_bg) };

            let cell = Cell { bg: row_bg, ..Default::default() };
            screen.fill_rect(Rect::new(cx, 0, label_w, 1), &cell);
            screen.write_str(cx, 0, &label, fg, row_bg, Default::default());
            cx += label_w;
        }

        // Separator
        let sep_cell = Cell { bg: theme.border(), ..Default::default() };
        screen.fill_rect(Rect::new(0, 1, rect.w, 1), &sep_cell);

        screen
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        let old = self.active;
        match action {
            WidgetAction::NavigateRight => {
                self.active = (old + 1) % self.tabs.len().max(1);
            }
            WidgetAction::NavigateLeft => {
                self.active = if old == 0 { self.tabs.len().saturating_sub(1) } else { old - 1 };
            }
            _ => return KeyHandleResult::Bubble,
        }
        if self.active != old {
            if let Some(ref cb) = self.on_switch { cb(self.active); }
        }
        KeyHandleResult::Handled
    }
}
