// ScrollViewWidget — scrollable viewport over a child.
// The child is laid out at its natural height, then only the visible viewport
// is rendered. renders_children() = true so the engine does NOT recurse into
// the child during render.

use arbor_tui_domain::cell::Cell;
use arbor_tui_domain::component::PropsRevisionBuilder;
use arbor_tui_domain::identity::DirtyKind;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::layout_engine::{layout_tree, measure_tree};
use arbor_tui_domain::render::render_tree_viewport;
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::signal::{ReadSignal, SignalDep};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetId, WidgetNode};
use arbor_tui_domain::PropsRevision;

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

    fn props_revision(&self) -> PropsRevision {
        let mut revision = PropsRevisionBuilder::new();
        revision
            .field_tag(1)
            .write_u16(self.content_h)
            .field_tag(2)
            .write_f32(self.props.flex)
            .field_tag(3)
            .write_option_u16(self.props.width)
            .field_tag(4)
            .write_option_u16(self.props.height)
            .field_tag(5)
            .write_u16(self.props.padding.top)
            .write_u16(self.props.padding.right)
            .write_u16(self.props.padding.bottom)
            .write_u16(self.props.padding.left)
            .finish()
    }

    fn signal_deps(&self) -> Vec<SignalDep> {
        vec![
            self.scroll_x.dep(DirtyKind::Render),
            self.scroll_y.dep(DirtyKind::Render),
        ]
    }

    fn on_mount(&mut self) {
        self.scroll_x
            .subscribe_with_dirty_kind(self.id, DirtyKind::Render);
        self.scroll_y
            .subscribe_with_dirty_kind(self.id, DirtyKind::Render);
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
        let viewport_w = rect.w.max(1);
        let viewport_h = rect.h.max(1);

        // Layout child at its full natural size, but render only the visible window.
        let child_h = self.content_h.max(rect.h).max(1);
        let child_rect = Rect::new(0, 0, viewport_w, child_h);
        let child = self.child.as_ref();
        let child_size = Size::new(child_rect.w, child_rect.h);
        let constraints = measure_tree(child, child_size);
        match layout_tree(child_rect, child, &constraints) {
            Ok(layout) => render_tree_viewport(
                (viewport_w, viewport_h),
                child,
                &layout,
                theme,
                focused,
                Rect::new(
                    self.scroll_x.get(),
                    self.scroll_y.get(),
                    viewport_w,
                    viewport_h,
                ),
            ),
            Err(_) => {
                let mut screen = VirtualScreen::new(viewport_w, viewport_h);
                let fill = Cell {
                    bg: theme.surface(),
                    ..Default::default()
                };
                screen.fill_rect(Rect::new(0, 0, viewport_w, viewport_h), &fill);
                let child_screen = child.render(child_rect, theme);
                screen.blit_region(
                    Rect::new(0, 0, viewport_w, viewport_h),
                    &child_screen,
                    (self.scroll_x.get(), self.scroll_y.get()),
                );
                screen
            }
        }
    }
}

impl Drop for ScrollViewWidget {
    fn drop(&mut self) {
        self.scroll_x.unsubscribe(self.id);
        self.scroll_y.unsubscribe(self.id);
    }
}
