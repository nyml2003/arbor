// FocusManager — tab-order navigation through focusable widgets.
// Tracks the currently focused widget and cycles through tab_order.
//
// WidgetNode helper functions (mount_tree, unmount_tree, find_widget_mut)
// work generically through the Widget trait — zero per-type dispatch.

use std::collections::HashMap;

use crate::widget::{assign_tree_identity, WidgetNode};
use crate::widget_id::WidgetId;

/// Errors from the focus system.
#[derive(Debug, thiserror::Error)]
pub enum FocusError {
    #[error("widget {0:?} not found in tab order — was rebuild() called?")]
    NotInTabOrder(WidgetId),
}

/// Manages keyboard focus across the widget tree.
pub struct FocusManager {
    currently_focused: Option<WidgetId>,
    tab_order: Vec<WidgetId>,
    parent_map: HashMap<WidgetId, WidgetId>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            currently_focused: None,
            tab_order: Vec::new(),
            parent_map: HashMap::new(),
        }
    }

    /// Rebuild the tab-order list and parent map by traversing the widget tree.
    pub fn rebuild(&mut self, root: &WidgetNode) {
        let mut pairs: Vec<(u16, WidgetId)> = Vec::new();
        Self::collect_focusable(root, &mut pairs);
        pairs.sort_by_key(|(tab_idx, _)| *tab_idx);
        self.tab_order = pairs.into_iter().map(|(_, id)| id).collect();

        self.parent_map.clear();
        self.build_parent_map(root, None);

        if let Some(id) = self.currently_focused {
            if !self.tab_order.contains(&id) {
                self.currently_focused = None;
            }
        }
    }

    fn build_parent_map(&mut self, node: &WidgetNode, parent: Option<WidgetId>) {
        let node_id = node.id();
        if let Some(pid) = parent {
            self.parent_map.insert(node_id, pid);
        }
        for child in node.children() {
            self.build_parent_map(child, Some(node_id));
        }
    }

    fn collect_focusable(node: &WidgetNode, out: &mut Vec<(u16, WidgetId)>) {
        if node.focusable() {
            out.push((node.tab_index(), node.id()));
        }
        for child in node.children() {
            Self::collect_focusable(child, out);
        }
    }

    pub fn focus_next(&mut self) -> Result<Option<WidgetId>, FocusError> {
        if self.tab_order.is_empty() {
            return Ok(None);
        }
        let next_idx = match self.currently_focused {
            Some(id) => {
                let pos = self
                    .tab_order
                    .iter()
                    .position(|x| *x == id)
                    .ok_or(FocusError::NotInTabOrder(id))?;
                (pos + 1) % self.tab_order.len()
            }
            None => 0,
        };
        self.currently_focused = Some(self.tab_order[next_idx]);
        Ok(self.currently_focused)
    }

    pub fn focus_prev(&mut self) -> Result<Option<WidgetId>, FocusError> {
        if self.tab_order.is_empty() {
            return Ok(None);
        }
        let prev_idx = match self.currently_focused {
            Some(id) => {
                let pos = self
                    .tab_order
                    .iter()
                    .position(|x| *x == id)
                    .ok_or(FocusError::NotInTabOrder(id))?;
                if pos == 0 {
                    self.tab_order.len() - 1
                } else {
                    pos - 1
                }
            }
            None => self.tab_order.len() - 1,
        };
        self.currently_focused = Some(self.tab_order[prev_idx]);
        Ok(self.currently_focused)
    }

    pub fn focus(&mut self, id: WidgetId) -> bool {
        if self.tab_order.contains(&id) {
            self.currently_focused = Some(id);
            true
        } else {
            false
        }
    }

    pub fn blur(&mut self) {
        self.currently_focused = None;
    }

    pub fn current(&self) -> Option<WidgetId> {
        self.currently_focused
    }
    pub fn is_focused(&self, id: WidgetId) -> bool {
        self.currently_focused == Some(id)
    }
    pub fn len(&self) -> usize {
        self.tab_order.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tab_order.is_empty()
    }

    pub fn parent_of(&self, id: WidgetId) -> Option<WidgetId> {
        self.parent_map.get(&id).copied()
    }

    pub fn ancestor_chain(&self, id: WidgetId) -> Vec<WidgetId> {
        let mut chain = Vec::new();
        let mut current = id;
        while let Some(parent) = self.parent_map.get(&current) {
            chain.push(*parent);
            current = *parent;
        }
        chain
    }
}

// ── Tree traversal helpers (generic, zero per-type dispatch) ──────

/// Walk the tree and call `on_mount()` on every widget (parent before children).
pub fn mount_tree(root: &mut WidgetNode) {
    assign_tree_identity(root).expect("widget tree identity assignment failed");
    mount_tree_inner(root);
}

fn mount_tree_inner(root: &mut WidgetNode) {
    root.on_mount();
    for child in root.children_mut() {
        mount_tree_inner(child);
    }
}

/// Walk the tree and call `on_unmount()` on every widget (children before parent).
pub fn unmount_tree(root: &mut WidgetNode) {
    for child in root.children_mut() {
        unmount_tree(child);
    }
    root.on_unmount();
}

/// Find a mutable reference to a widget by ID, traversing the tree generically.
pub fn find_widget_mut(root: &mut WidgetNode, target: WidgetId) -> Option<&mut WidgetNode> {
    if root.id() == target {
        return Some(root);
    }
    for child in root.children_mut() {
        if let Some(found) = find_widget_mut(child, target) {
            return Some(found);
        }
    }
    None
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal test widget that's focusable
    struct TestWidget {
        id: WidgetId,
        props: crate::layout::LayoutProps,
    }

    impl crate::widget::Widget for TestWidget {
        fn id(&self) -> WidgetId {
            self.id
        }
        fn layout_props(&self) -> &crate::layout::LayoutProps {
            &self.props
        }
        fn focusable(&self) -> bool {
            true
        }
    }

    fn make_focusable(id: u64) -> WidgetNode {
        WidgetNode::new(TestWidget {
            id: WidgetId(id),
            props: crate::layout::LayoutProps::default(),
        })
    }

    #[test]
    fn empty_tree_no_focusable() {
        struct NonFocusable {
            id: WidgetId,
            props: crate::layout::LayoutProps,
        }
        impl crate::widget::Widget for NonFocusable {
            fn id(&self) -> WidgetId {
                self.id
            }
            fn layout_props(&self) -> &crate::layout::LayoutProps {
                &self.props
            }
            fn focusable(&self) -> bool {
                false
            }
        }
        let root = WidgetNode::new(NonFocusable {
            id: WidgetId(0),
            props: Default::default(),
        });
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert!(fm.tab_order.is_empty());
        assert_eq!(fm.current(), None);
        assert_eq!(fm.focus_next().unwrap(), None);
        assert_eq!(fm.focus_prev().unwrap(), None);
    }

    #[test]
    fn rebuild_collects_focusable_widgets() {
        let root = make_focusable(1);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.len(), 1);
    }

    #[test]
    fn next_cycles_through_focusable() {
        let mut fm = FocusManager::new();
        fm.rebuild(&make_focusable(1));
        assert_eq!(fm.focus_next().unwrap(), Some(WidgetId(1)));
        assert_eq!(fm.focus_next().unwrap(), Some(WidgetId(1)));
    }

    #[test]
    fn prev_wraps_around_to_last() {
        let mut fm = FocusManager::new();
        fm.rebuild(&make_focusable(1));
        assert_eq!(fm.focus_prev().unwrap(), Some(WidgetId(1)));
    }

    #[test]
    fn blur_clears_focus() {
        let mut fm = FocusManager::new();
        fm.rebuild(&make_focusable(1));
        let _ = fm.focus_next();
        fm.blur();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn mount_tree_calls_on_mount() {
        use std::cell::Cell;
        use std::rc::Rc;
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        struct MountWidget {
            id: WidgetId,
            props: crate::layout::LayoutProps,
            on_mount_cb: Box<dyn Fn()>,
        }
        impl crate::widget::Widget for MountWidget {
            fn id(&self) -> WidgetId {
                self.id
            }
            fn layout_props(&self) -> &crate::layout::LayoutProps {
                &self.props
            }
            fn on_mount(&mut self) {
                (self.on_mount_cb)();
            }
        }
        let mut root = WidgetNode::new(MountWidget {
            id: WidgetId(0),
            props: Default::default(),
            on_mount_cb: Box::new(move || c.set(true)),
        });
        mount_tree(&mut root);
        assert!(called.get());
    }
}
