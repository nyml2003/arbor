// Widget trait and WidgetNode type.
// WidgetNode is a newtype over Box<dyn Widget> — adding a new component
// requires NO changes to the core crate. Just impl the trait.

use crate::input::KeyHandleResult;
use crate::layout::{LayoutProps, Rect, Size, SizeConstraint};
// Re-exports for downstream convenience
pub use crate::cell;
pub use crate::input;
pub use crate::layout;
pub use crate::screen;
use crate::screen::VirtualScreen;
pub use crate::signal;
pub use crate::text;
pub use crate::theme;
use crate::theme::Theme;
pub use crate::widget_id::{WidgetAction, WidgetId, WidgetLayoutInfo};

use std::collections::HashMap;

// ── Widget trait ───────────────────────────────────────────────────

pub trait Widget {
    fn id(&self) -> WidgetId;
    fn layout_props(&self) -> &LayoutProps;

    fn children(&self) -> &[WidgetNode] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [WidgetNode] {
        &mut []
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        SizeConstraint::at_least_one()
    }

    fn measure_subtree(
        &self,
        available: Size,
        _child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        self.measure(available)
    }

    fn children_rect(&self, content_rect: Rect) -> Rect {
        content_rect
    }

    fn render(&self, _rect: Rect, _theme: &Theme) -> VirtualScreen {
        VirtualScreen::new(_rect.w, _rect.h)
    }

    /// Render with focus hint. Override to show cursor/selection.
    /// Default: delegates to `render()`.
    fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.render(rect, theme)
    }

    /// Render with tree-level focus context.
    ///
    /// Most widgets only care whether they are the focused node. Widgets that
    /// render their own child subtree, such as scroll containers, forward this
    /// context to that internal render pass.
    fn render_with_focus(
        &self,
        rect: Rect,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> VirtualScreen {
        if focused == Some(self.id()) {
            self.render_focused(rect, theme)
        } else {
            self.render(rect, theme)
        }
    }

    fn is_transparent(&self) -> bool {
        false
    }
    fn renders_children(&self) -> bool {
        false
    }

    fn focusable(&self) -> bool {
        false
    }
    fn tab_index(&self) -> u16 {
        0
    }
    fn perform(&mut self, _action: &WidgetAction) -> KeyHandleResult {
        KeyHandleResult::Bubble
    }

    fn on_mount(&mut self) {}
    fn on_unmount(&mut self) {}
}

// ── WidgetNode ─────────────────────────────────────────────────────

pub struct WidgetNode(Box<dyn Widget>);

impl WidgetNode {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self(Box::new(widget))
    }
    pub fn inner(&self) -> &dyn Widget {
        &*self.0
    }
    pub fn inner_mut(&mut self) -> &mut dyn Widget {
        &mut *self.0
    }
}

impl WidgetNode {
    pub fn id(&self) -> WidgetId {
        self.0.id()
    }
    pub fn layout_props(&self) -> &LayoutProps {
        self.0.layout_props()
    }
    pub fn children(&self) -> &[WidgetNode] {
        self.0.children()
    }
    pub fn children_mut(&mut self) -> &mut [WidgetNode] {
        self.0.children_mut()
    }
    pub fn focusable(&self) -> bool {
        self.0.focusable()
    }
    pub fn tab_index(&self) -> u16 {
        self.0.tab_index()
    }
    pub fn is_transparent(&self) -> bool {
        self.0.is_transparent()
    }
    pub fn renders_children(&self) -> bool {
        self.0.renders_children()
    }

    pub fn measure(&self, available: Size) -> SizeConstraint {
        self.0.measure(available)
    }
    pub fn measure_subtree(
        &self,
        available: Size,
        child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        self.0.measure_subtree(available, child_constraints)
    }
    pub fn children_rect(&self, content_rect: Rect) -> Rect {
        self.0.children_rect(content_rect)
    }
    pub fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.0.render(rect, theme)
    }
    pub fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.0.render_focused(rect, theme)
    }
    pub fn render_with_focus(
        &self,
        rect: Rect,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> VirtualScreen {
        self.0.render_with_focus(rect, theme, focused)
    }
    pub fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        self.0.perform(action)
    }
    pub fn on_mount(&mut self) {
        self.0.on_mount();
    }
    pub fn on_unmount(&mut self) {
        self.0.on_unmount();
    }
}
