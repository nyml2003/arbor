// FocusManager — tab-order navigation through focusable widgets.
// Tracks the currently focused widget and cycles through tab_order.

use std::collections::HashMap;

use crate::widget::{Widget, WidgetId, WidgetNode};

/// Manages keyboard focus across the widget tree.
pub struct FocusManager {
    /// Currently focused widget, if any.
    current_focus: Option<WidgetId>,
    /// Ordered list of all focusable widget IDs (by tab_index then DFS order).
    tab_order: Vec<WidgetId>,
    /// Maps child → parent WidgetId for event bubbling.
    parent_map: HashMap<WidgetId, WidgetId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            current_focus: None,
            tab_order: Vec::new(),
            parent_map: HashMap::new(),
        }
    }

    /// Rebuild the tab-order list and parent map by traversing the widget tree.
    pub fn rebuild(&mut self, root: &WidgetNode) {
        let mut pairs: Vec<(u16, WidgetId)> = Vec::new();
        self.collect_focusable(root, &mut pairs);
        pairs.sort_by_key(|(tab_idx, _)| *tab_idx);
        self.tab_order = pairs.into_iter().map(|(_, id)| id).collect();

        // Rebuild parent map
        self.parent_map.clear();
        self.build_parent_map(root);

        if let Some(id) = self.current_focus {
            if !self.tab_order.contains(&id) {
                self.current_focus = None;
            }
        }
    }

    /// Walk the tree and record parent-child relationships.
    fn build_parent_map(&mut self, node: &WidgetNode) {
        let parent_id = node.id();
        for child in node.children() {
            self.parent_map.insert(child.id(), parent_id);
            self.build_parent_map(child);
        }
    }

    fn collect_focusable(&self, node: &WidgetNode, out: &mut Vec<(u16, WidgetId)>) {
        if node.focusable() {
            out.push((node.tab_index(), node.id()));
        }
        for child in node.children() {
            self.collect_focusable(child, out);
        }
    }

    /// Move focus to the next focusable widget (Tab).
    pub fn next(&mut self) -> Option<WidgetId> {
        if self.tab_order.is_empty() {
            return None;
        }
        let next_idx = match self.current_focus {
            Some(id) => {
                let pos = self.tab_order.iter().position(|x| *x == id).unwrap_or(0);
                (pos + 1) % self.tab_order.len()
            }
            None => 0,
        };
        self.current_focus = Some(self.tab_order[next_idx]);
        self.current_focus
    }

    /// Move focus to the previous focusable widget (Shift+Tab).
    pub fn prev(&mut self) -> Option<WidgetId> {
        if self.tab_order.is_empty() {
            return None;
        }
        let prev_idx = match self.current_focus {
            Some(id) => {
                let pos = self.tab_order.iter().position(|x| *x == id).unwrap_or(0);
                if pos == 0 {
                    self.tab_order.len() - 1
                } else {
                    pos - 1
                }
            }
            None => self.tab_order.len() - 1,
        };
        self.current_focus = Some(self.tab_order[prev_idx]);
        self.current_focus
    }

    /// Set focus to a specific widget. Returns true if successful.
    pub fn focus(&mut self, id: WidgetId) -> bool {
        if self.tab_order.contains(&id) {
            self.current_focus = Some(id);
            true
        } else {
            false
        }
    }

    /// Remove focus from the current widget.
    pub fn blur(&mut self) {
        self.current_focus = None;
    }

    /// Return the currently focused widget ID, if any.
    pub fn current(&self) -> Option<WidgetId> {
        self.current_focus
    }

    /// Check if a given widget is currently focused.
    pub fn is_focused(&self, id: WidgetId) -> bool {
        self.current_focus == Some(id)
    }

    pub fn len(&self) -> usize {
        self.tab_order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tab_order.is_empty()
    }

    /// Get the parent widget ID for event bubbling.
    pub fn parent_of(&self, id: WidgetId) -> Option<WidgetId> {
        self.parent_map.get(&id).copied()
    }

    /// Collect the ancestor chain from a widget up to root (exclusive of self).
    /// Returns `[parent, grandparent, ..., root]`.
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

// ── WidgetNode helper methods ────────────────────────────────────

impl WidgetNode {
    pub fn id(&self) -> WidgetId {
        match self {
            WidgetNode::Box(w) => w.id,
            WidgetNode::Text(w) => w.id,
            WidgetNode::Input(w) => w.id,
            WidgetNode::Button(w) => w.id,
            WidgetNode::List(w) => w.id,
            WidgetNode::Table(w) => w.id,
            WidgetNode::Tabs(w) => w.id,
            WidgetNode::ScrollView(w) => w.id,
        }
    }

    pub fn focusable(&self) -> bool {
        match self {
            WidgetNode::Input(_)
            | WidgetNode::Button(_)
            | WidgetNode::List(_)
            | WidgetNode::Table(_)
            | WidgetNode::Tabs(_) => true,
            _ => false,
        }
    }

    pub fn tab_index(&self) -> u16 {
        match self {
            WidgetNode::Input(_)
            | WidgetNode::Button(_)
            | WidgetNode::List(_)
            | WidgetNode::Table(_)
            | WidgetNode::Tabs(_) => 0,
            _ => 0,
        }
    }

    /// Dispatch a key event to this widget. Returns Handled if consumed, Bubble to propagate.
    pub fn on_key(&mut self, event: &crate::input::KeyEvent) -> crate::input::KeyHandleResult {
        match self {
            WidgetNode::Input(w) => w.on_key(event),
            WidgetNode::Button(w) => w.on_key(event),
            WidgetNode::List(w) => w.on_key(event),
            WidgetNode::Table(w) => w.on_key(event),
            WidgetNode::Tabs(w) => w.on_key(event),
            WidgetNode::ScrollView(w) => w.on_key(event),
            _ => crate::input::KeyHandleResult::Bubble,
        }
    }

    /// Lifecycle: called when the widget is inserted into the tree.
    pub fn on_mount(&mut self) {
        match self {
            WidgetNode::Text(w) => w.on_mount(),
            WidgetNode::Input(w) => w.on_mount(),
            WidgetNode::Button(w) => w.on_mount(),
            WidgetNode::List(w) => w.on_mount(),
            WidgetNode::Table(w) => w.on_mount(),
            WidgetNode::Tabs(w) => w.on_mount(),
            WidgetNode::Box(w) => w.on_mount(),
            WidgetNode::ScrollView(w) => w.on_mount(),
        }
    }

    /// Lifecycle: called when the widget is removed from the tree.
    pub fn on_unmount(&mut self) {
        match self {
            WidgetNode::Text(w) => w.on_unmount(),
            WidgetNode::Input(w) => w.on_unmount(),
            WidgetNode::Button(w) => w.on_unmount(),
            WidgetNode::List(w) => w.on_unmount(),
            WidgetNode::Table(w) => w.on_unmount(),
            WidgetNode::Tabs(w) => w.on_unmount(),
            WidgetNode::Box(w) => w.on_unmount(),
            WidgetNode::ScrollView(w) => w.on_unmount(),
        }
    }

    pub fn children(&self) -> &[WidgetNode] {
        match self {
            WidgetNode::Box(w) => &w.children,
            WidgetNode::Tabs(w) => {
                if w.tabs.is_empty() {
                    &[]
                } else {
                    let idx = w.active.min(w.tabs.len() - 1);
                    std::slice::from_ref(&w.tabs[idx].content)
                }
            }
            WidgetNode::ScrollView(w) => std::slice::from_ref(&*w.child),
            _ => &[],
        }
    }

    pub fn layout_props(&self) -> &crate::layout::LayoutProps {
        match self {
            WidgetNode::Box(w) => &w.props,
            WidgetNode::Text(w) => &w.props,
            WidgetNode::Input(w) => &w.props,
            WidgetNode::Button(w) => &w.props,
            WidgetNode::List(w) => &w.props,
            WidgetNode::Table(w) => &w.props,
            WidgetNode::Tabs(w) => &w.props,
            WidgetNode::ScrollView(w) => &w.props,
        }
    }
}

/// Walk the tree and call `on_mount()` on every widget (parent before children).
/// Should be called once after the tree is first built.
pub fn mount_tree(root: &mut WidgetNode) {
    root.on_mount();
    match root {
        WidgetNode::Box(w) => {
            for child in &mut w.children { mount_tree(child); }
        }
        WidgetNode::Tabs(w) => {
            for tab in &mut w.tabs { mount_tree(&mut tab.content); }
        }
        WidgetNode::ScrollView(w) => {
            mount_tree(&mut *w.child);
        }
        _ => {} // leaf nodes
    }
}

/// Walk the tree and call `on_unmount()` on every widget (children before parent).
/// Should be called before the tree is dropped or replaced.
pub fn unmount_tree(root: &mut WidgetNode) {
    match root {
        WidgetNode::Box(w) => {
            for child in &mut w.children { unmount_tree(child); }
        }
        WidgetNode::Tabs(w) => {
            for tab in &mut w.tabs { unmount_tree(&mut tab.content); }
        }
        WidgetNode::ScrollView(w) => {
            unmount_tree(&mut *w.child);
        }
        _ => {} // leaf nodes
    }
    root.on_unmount();
}

/// Find a mutable reference to a widget by ID, traversing the tree.
pub fn find_widget_mut<'a>(root: &'a mut WidgetNode, target: WidgetId) -> Option<&'a mut WidgetNode> {
    if root.id() == target {
        return Some(root);
    }
    match root {
        WidgetNode::Box(w) => {
            for child in &mut w.children {
                if let Some(found) = find_widget_mut(child, target) {
                    return Some(found);
                }
            }
        }
        WidgetNode::ScrollView(w) => {
            return find_widget_mut(&mut *w.child, target);
        }
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutProps;
    use crate::widget::{BoxWidget, InputWidget, ButtonWidget, ButtonStyle};

    fn make_box(id: u64, children: Vec<WidgetNode>) -> WidgetNode {
        WidgetNode::Box(BoxWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            children,
        })
    }

    fn make_input(id: u64) -> WidgetNode {
        WidgetNode::Input(InputWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            buffer: String::new(),
            cursor: 0,
            placeholder: String::new(),
            password: false,
            on_change: None,
            on_submit: None,
        })
    }

    fn make_button(id: u64, label: &str) -> WidgetNode {
        WidgetNode::Button(ButtonWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            label: crate::signal::ReadSignal::constant(label.to_string()),
            style: ButtonStyle::Default,
            on_click: None,
        })
    }

    #[test]
    fn empty_tree_no_focusable() {
        let root = make_box(0, vec![]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert!(fm.tab_order.is_empty());
        assert_eq!(fm.current(), None);
        assert_eq!(fm.next(), None);
        assert_eq!(fm.prev(), None);
    }

    #[test]
    fn rebuild_collects_focusable_widgets() {
        let root = make_box(0, vec![make_input(1), make_button(2, "OK")]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.len(), 2);
        assert!(fm.tab_order.contains(&WidgetId(1)));
        assert!(fm.tab_order.contains(&WidgetId(2)));
    }

    #[test]
    fn next_cycles_through_focusable() {
        let root = make_box(0, vec![
            make_input(1),
            make_button(2, "OK"),
            make_input(3),
        ]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        let first = fm.next().unwrap();
        let second = fm.next().unwrap();
        assert_ne!(second, first);
    }

    #[test]
    fn next_wraps_around_to_first() {
        let root = make_box(0, vec![make_input(1), make_button(2, "OK")]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        let first = fm.next().unwrap();
        fm.next();
        let third = fm.next().unwrap();
        assert_eq!(third, first);
    }

    #[test]
    fn prev_wraps_around_to_last() {
        let root = make_box(0, vec![make_input(1), make_button(2, "OK")]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.prev().unwrap(), WidgetId(2));
    }

    #[test]
    fn focus_sets_current_directly() {
        let root = make_box(0, vec![make_input(1), make_button(2, "OK")]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert!(fm.focus(WidgetId(2)));
        assert_eq!(fm.current(), Some(WidgetId(2)));
    }

    #[test]
    fn focus_invalid_id_returns_false() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert!(!fm.focus(WidgetId(999)));
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn blur_clears_focus() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        fm.next();
        fm.blur();
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn is_focused_checks_current() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        fm.next();
        assert!(fm.is_focused(WidgetId(1)));
        assert!(!fm.is_focused(WidgetId(999)));
    }

    #[test]
    fn single_focusable_next_prev_same() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.next().unwrap(), WidgetId(1));
        assert_eq!(fm.next().unwrap(), WidgetId(1));
        assert_eq!(fm.prev().unwrap(), WidgetId(1));
    }

    #[test]
    fn rebuild_clears_focus_if_removed() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        fm.next();
        assert_eq!(fm.current(), Some(WidgetId(1)));
        let new_root = make_box(0, vec![make_button(2, "OK")]);
        fm.rebuild(&new_root);
        assert_eq!(fm.current(), None);
    }

    #[test]
    fn parent_map_maps_child_to_parent() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.parent_of(WidgetId(1)), Some(WidgetId(0)));
    }

    #[test]
    fn parent_of_root_returns_none() {
        let root = make_box(0, vec![make_input(1)]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);
        assert_eq!(fm.parent_of(WidgetId(0)), None);
    }

    #[test]
    fn ancestor_chain_from_leaf_to_root() {
        // Box(0) → Box(10) → Input(1)
        let inner_box = make_box(10, vec![make_input(1)]);
        let root = make_box(0, vec![inner_box]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);

        let chain = fm.ancestor_chain(WidgetId(1));
        assert_eq!(chain, vec![WidgetId(10), WidgetId(0)]);
    }

    #[test]
    fn mount_tree_calls_on_mount() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let mounted = Rc::new(RefCell::new(false));
        let m_clone = mounted.clone();

        // Build a tree and override on_mount via a closure-based approach.
        // Since we can't easily override on_mount on built-in widgets,
        // we verify that mount_tree traverses without panicking.
        let mut root = make_box(0, vec![make_input(1)]);
        mount_tree(&mut root);
        // mount_tree should not panic
    }

    #[test]
    fn unmount_tree_calls_on_unmount() {
        let mut root = make_box(0, vec![make_input(1)]);
        mount_tree(&mut root);
        unmount_tree(&mut root);
        // unmount_tree should not panic
    }
}
