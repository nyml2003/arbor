// ScrollViewWidget — scrollable viewport over a child.
// The child is rendered at its full natural size; the scroll widget
// copies only the visible portion. renders_children() = true so the
// engine does NOT recurse into the child during render.

use arbor_tui_domain::cell::Cell;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::layout_engine::{layout_tree, measure_tree};
use arbor_tui_domain::render::render_tree;
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::signal::ReadSignal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetId, WidgetNode};

use std::collections::HashMap;

pub struct ScrollViewWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub child: Box<WidgetNode>,
    pub scroll_x: ReadSignal<u16>,
    pub scroll_y: ReadSignal<u16>,
    /// Natural height of child content in rows. Used at render time to
    /// allocate enough space for the child to render at full height.
    pub content_h: u16,
    pub on_scroll: Option<Box<dyn Fn(u16, u16)>>,
}

impl Widget for ScrollViewWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn children(&self) -> &[WidgetNode] {
        std::slice::from_ref(&*self.child)
    }
    fn children_mut(&mut self) -> &mut [WidgetNode] {
        std::slice::from_mut(&mut *self.child)
    }

    /// ScrollView renders its own child (for clipping).
    fn renders_children(&self) -> bool {
        true
    }

    fn on_mount(&mut self) {
        self.scroll_x.subscribe(self.id);
        self.scroll_y.subscribe(self.id);
    }

    fn on_unmount(&mut self) {
        self.scroll_x.unsubscribe(self.id);
        self.scroll_y.unsubscribe(self.id);
    }

    fn measure_subtree(
        &self,
        available: Size,
        _child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        // ScrollView takes whatever space is available — child can be larger
        SizeConstraint::bounded(available)
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.render_viewport(rect, theme, None)
    }

    fn render_with_focus(
        &self,
        rect: Rect,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> VirtualScreen {
        self.render_viewport(rect, theme, focused)
    }
}

impl ScrollViewWidget {
    fn render_viewport(
        &self,
        rect: Rect,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));

        // 先用背景色填充整个视口，避免子组件比视口小时 Cell::default() 黑底覆盖父组件。
        let fill = Cell {
            bg: theme.surface(),
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), rect.h.max(1)), &fill);

        // Render child at its full natural size (larger than viewport)
        let child_h = self.content_h.max(rect.h).max(1);
        let child_rect = Rect::new(0, 0, rect.w.max(1), child_h);
        let child = self.child.as_ref();
        let child_size = Size::new(child_rect.w, child_rect.h);
        let constraints = measure_tree(child, child_size);
        let child_screen = match layout_tree(child_rect, child, &constraints) {
            Ok(layout) => render_tree((child_rect.w, child_rect.h), child, &layout, theme, focused),
            Err(_) => child.render(child_rect, theme),
        };

        // Copy visible viewport
        let copy_w = rect
            .w
            .min(child_screen.cols().saturating_sub(self.scroll_x.get()));
        let copy_h = rect
            .h
            .min(child_screen.rows().saturating_sub(self.scroll_y.get()));

        for row in 0..copy_h {
            for col in 0..copy_w {
                let src_cell =
                    child_screen.cell_at(self.scroll_x.get() + col, self.scroll_y.get() + row);
                if let Some(dest) = screen.cell_at_mut(col, row) {
                    *dest = src_cell;
                }
            }
        }
        screen
    }
}

impl Drop for ScrollViewWidget {
    fn drop(&mut self) {
        self.scroll_x.unsubscribe(self.id);
        self.scroll_y.unsubscribe(self.id);
    }
}
