// Widget trait and WidgetNode type.
// WidgetNode wraps a Widget implementation plus runtime tree metadata.
// Adding a new component requires NO changes to the core crate. Just impl the trait.

use crate::identity::{DirtyKind, IdentityError, NodeIdentity, WidgetKey};
use crate::input::KeyHandleResult;
use crate::layout::{LayoutProps, Rect, Size, SizeConstraint};
use crate::signal::SignalDep;
use crate::PropsRevision;
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

    fn widget_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn props_revision(&self) -> PropsRevision {
        PropsRevision::ZERO
    }

    fn signal_deps(&self) -> Vec<SignalDep> {
        Vec::new()
    }

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

    fn dirty_on_action(&self, _action: &WidgetAction) -> DirtyKind {
        DirtyKind::Render
    }

    fn on_mount(&mut self) {}
    fn on_unmount(&mut self) {}
}

// ── WidgetNode ─────────────────────────────────────────────────────

pub struct WidgetNode {
    widget: Box<dyn Widget>,
    key: Option<WidgetKey>,
    path_identity: Option<Vec<u16>>,
}

impl WidgetNode {
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self {
            widget: Box::new(widget),
            key: None,
            path_identity: None,
        }
    }
    pub fn inner(&self) -> &dyn Widget {
        &*self.widget
    }
    pub fn inner_mut(&mut self) -> &mut dyn Widget {
        &mut *self.widget
    }

    pub fn with_key(mut self, key: impl Into<WidgetKey>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn set_key(&mut self, key: impl Into<WidgetKey>) {
        self.key = Some(key.into());
    }

    pub fn key(&self) -> Option<&WidgetKey> {
        self.key.as_ref()
    }

    pub(crate) fn set_path_identity(&mut self, path: Vec<u16>) {
        self.path_identity = Some(path);
    }

    pub fn path_identity(&self) -> Option<&[u16]> {
        self.path_identity.as_deref()
    }

    pub fn identity(&self) -> Option<NodeIdentity> {
        if let Some(key) = &self.key {
            return Some(NodeIdentity::Keyed(key.clone()));
        }
        self.path_identity
            .as_ref()
            .map(|path| NodeIdentity::Path(path.clone()))
    }

    pub fn widget_type_name(&self) -> &'static str {
        self.widget.widget_type_name()
    }

    pub fn props_revision(&self) -> PropsRevision {
        self.widget.props_revision()
    }

    pub fn signal_deps(&self) -> Vec<SignalDep> {
        self.widget.signal_deps()
    }
}

impl WidgetNode {
    pub fn id(&self) -> WidgetId {
        self.widget.id()
    }
    pub fn layout_props(&self) -> &LayoutProps {
        self.widget.layout_props()
    }
    pub fn children(&self) -> &[WidgetNode] {
        self.widget.children()
    }
    pub fn children_mut(&mut self) -> &mut [WidgetNode] {
        self.widget.children_mut()
    }
    pub fn focusable(&self) -> bool {
        self.widget.focusable()
    }
    pub fn tab_index(&self) -> u16 {
        self.widget.tab_index()
    }
    pub fn is_transparent(&self) -> bool {
        self.widget.is_transparent()
    }
    pub fn renders_children(&self) -> bool {
        self.widget.renders_children()
    }

    pub fn measure(&self, available: Size) -> SizeConstraint {
        self.widget.measure(available)
    }
    pub fn measure_subtree(
        &self,
        available: Size,
        child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        self.widget.measure_subtree(available, child_constraints)
    }
    pub fn children_rect(&self, content_rect: Rect) -> Rect {
        self.widget.children_rect(content_rect)
    }
    pub fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.widget.render(rect, theme)
    }
    pub fn render_focused(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        self.widget.render_focused(rect, theme)
    }
    pub fn render_with_focus(
        &self,
        rect: Rect,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> VirtualScreen {
        self.widget.render_with_focus(rect, theme, focused)
    }
    pub fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        self.widget.perform(action)
    }
    pub fn dirty_on_action(&self, action: &WidgetAction) -> DirtyKind {
        self.widget.dirty_on_action(action)
    }
    pub fn on_mount(&mut self) {
        self.widget.on_mount();
    }
    pub fn on_unmount(&mut self) {
        self.widget.on_unmount();
    }
}

pub fn assign_tree_identity(root: &mut WidgetNode) -> Result<(), IdentityError> {
    assign_node_identity(root, &mut Vec::new())
}

fn assign_node_identity(node: &mut WidgetNode, path: &mut Vec<u16>) -> Result<(), IdentityError> {
    node.set_path_identity(path.clone());
    validate_sibling_keys(node)?;

    for (index, child) in node.children_mut().iter_mut().enumerate() {
        path.push(index as u16);
        assign_node_identity(child, path)?;
        path.pop();
    }

    Ok(())
}

fn validate_sibling_keys(node: &WidgetNode) -> Result<(), IdentityError> {
    let mut seen: HashMap<WidgetKey, usize> = HashMap::new();
    for (index, child) in node.children().iter().enumerate() {
        let Some(key) = child.key().cloned() else {
            continue;
        };
        if let Some(first_index) = seen.insert(key.clone(), index) {
            return Err(IdentityError::DuplicateSiblingKey {
                parent: node.id(),
                key,
                first_index,
                second_index: index,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestWidget {
        id: WidgetId,
        props: LayoutProps,
        children: Vec<WidgetNode>,
    }

    impl TestWidget {
        fn leaf(id: u64) -> WidgetNode {
            WidgetNode::new(Self {
                id: WidgetId(id),
                props: LayoutProps::default(),
                children: Vec::new(),
            })
        }

        fn branch(id: u64, children: Vec<WidgetNode>) -> WidgetNode {
            WidgetNode::new(Self {
                id: WidgetId(id),
                props: LayoutProps::default(),
                children,
            })
        }
    }

    impl Widget for TestWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn layout_props(&self) -> &LayoutProps {
            &self.props
        }

        fn children(&self) -> &[WidgetNode] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [WidgetNode] {
            &mut self.children
        }
    }

    #[test]
    fn assigned_identity_uses_key_before_path() {
        let mut root = TestWidget::branch(
            0,
            vec![TestWidget::leaf(1).with_key("title"), TestWidget::leaf(2)],
        );

        assign_tree_identity(&mut root).unwrap();

        assert_eq!(root.identity(), Some(NodeIdentity::Path(vec![])));
        assert_eq!(
            root.children()[0].identity(),
            Some(NodeIdentity::Keyed(WidgetKey::new("title")))
        );
        assert_eq!(
            root.children()[1].identity(),
            Some(NodeIdentity::Path(vec![1]))
        );
    }

    #[test]
    fn path_identity_tracks_tree_position() {
        let mut root = TestWidget::branch(
            0,
            vec![
                TestWidget::leaf(1),
                TestWidget::branch(2, vec![TestWidget::leaf(3)]),
            ],
        );

        assign_tree_identity(&mut root).unwrap();

        assert_eq!(root.path_identity(), Some(&[][..]));
        assert_eq!(root.children()[0].path_identity(), Some(&[0][..]));
        assert_eq!(root.children()[1].path_identity(), Some(&[1][..]));
        assert_eq!(
            root.children()[1].children()[0].path_identity(),
            Some(&[1, 0][..])
        );
    }

    #[test]
    fn duplicate_sibling_keys_are_rejected() {
        let mut root = TestWidget::branch(
            0,
            vec![
                TestWidget::leaf(1).with_key("dup"),
                TestWidget::leaf(2).with_key("dup"),
            ],
        );

        let err = assign_tree_identity(&mut root).unwrap_err();

        assert_eq!(
            err,
            IdentityError::DuplicateSiblingKey {
                parent: WidgetId(0),
                key: WidgetKey::new("dup"),
                first_index: 0,
                second_index: 1,
            }
        );
    }
}
