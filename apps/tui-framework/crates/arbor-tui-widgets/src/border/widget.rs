// BorderWidget — wraps a child with a Unicode-box border.
// Supports rounded/sharp corners and optional title.

use arbor_tui_primitives::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_primitives::layout::{LayoutProps, Rect, Size, SizeCalc, SizeConstraint};
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{Widget, WidgetId, WidgetNode};

use std::collections::HashMap;

pub(crate) struct BorderWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub child: Box<WidgetNode>,
    pub title: Option<String>,
    pub rounded: bool,
    pub fg: AnsiColor,
    pub bg: AnsiColor,
}

impl Widget for BorderWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn children(&self) -> &[WidgetNode] { std::slice::from_ref(&*self.child) }
    fn children_mut(&mut self) -> &mut [WidgetNode] { std::slice::from_mut(&mut *self.child) }
    fn is_transparent(&self) -> bool { false }

    fn measure_subtree(&self, available: Size, child_constraints: &HashMap<WidgetId, SizeConstraint>) -> SizeConstraint {
        // Border is 1 cell thick on each side. child was measured with (available - padding).
        // The padding includes border space by convention.
        if let Some(cc) = child_constraints.get(&self.child.id()) {
            let border_w = 2u16;
            let border_h = 2u16;
            SizeConstraint {
                min_w: cc.min_w.saturating_add(border_w).max(3),
                min_h: cc.min_h.saturating_add(border_h).max(2),
                max_w: arbor_tui_primitives::layout::AxisConstraint::Fixed(available.w),
                max_h: arbor_tui_primitives::layout::AxisConstraint::Fixed(available.h),
            }
        } else {
            SizeConstraint::bounded(available)
        }
    }

    fn children_rect(&self, content_rect: Rect) -> Rect {
        // Content rect already accounts for padding. Border takes the outer 1 cell
        // per side from the content rect. Child gets the interior.
        Rect {
            x: content_rect.x + 1,
            y: content_rect.y + 1,
            w: content_rect.w.saturating_sub(2),
            h: content_rect.h.saturating_sub(2),
        }
    }

    fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(3), rect.h.max(2));
        let w = rect.w;
        let h = rect.h;

        // Fill entire area with border bg so interior has consistent color
        let fill = Cell { bg: self.bg, ..Default::default() };
        screen.fill_rect(Rect::new(0, 0, w, h), &fill);

        // Corners
        let (tl, tr, bl, br) = if self.rounded {
            ('\u{256D}', '\u{256E}', '\u{2570}', '\u{256F}') // ╭ ╮ ╰ ╯
        } else {
            ('\u{250C}', '\u{2510}', '\u{2514}', '\u{2518}') // ┌ ┐ └ ┘
        };
        let h_line = '\u{2500}'; // ─
        let v_line = '\u{2502}'; // │

        // Top border
        screen.cell_at_mut(0, 0).map(|c| { *c = Cell { ch: tl, fg: self.fg, bg: self.bg, ..Default::default() }; });
        screen.cell_at_mut(w - 1, 0).map(|c| { *c = Cell { ch: tr, fg: self.fg, bg: self.bg, ..Default::default() }; });
        for x in 1..w - 1 {
            screen.cell_at_mut(x, 0).map(|c| { *c = Cell { ch: h_line, fg: self.fg, bg: self.bg, ..Default::default() }; });
        }

        // Bottom border
        screen.cell_at_mut(0, h - 1).map(|c| { *c = Cell { ch: bl, fg: self.fg, bg: self.bg, ..Default::default() }; });
        screen.cell_at_mut(w - 1, h - 1).map(|c| { *c = Cell { ch: br, fg: self.fg, bg: self.bg, ..Default::default() }; });
        for x in 1..w - 1 {
            screen.cell_at_mut(x, h - 1).map(|c| { *c = Cell { ch: h_line, fg: self.fg, bg: self.bg, ..Default::default() }; });
        }

        // Side borders
        for y in 1..h - 1 {
            screen.cell_at_mut(0, y).map(|c| { *c = Cell { ch: v_line, fg: self.fg, bg: self.bg, ..Default::default() }; });
            screen.cell_at_mut(w - 1, y).map(|c| { *c = Cell { ch: v_line, fg: self.fg, bg: self.bg, ..Default::default() }; });
        }

        // Title in top border
        if let Some(ref title) = self.title {
            let max_title_w = (w as usize).saturating_sub(4);
            let display: String = title.chars().take(max_title_w).collect();
            for (i, ch) in display.chars().enumerate() {
                let col = 2 + i as u16;
                if col < w - 1 {
                    screen.cell_at_mut(col, 0).map(|c| { c.ch = ch; c.fg = self.fg; c.bg = self.bg; });
                }
            }
        }

        screen
    }
}
