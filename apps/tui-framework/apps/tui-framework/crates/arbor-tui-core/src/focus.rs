// FocusManager — tab-order navigation through focusable widgets.
// Tracks the currently focused widget and cycles through tab_order.

use crate::widget::{WidgetId, WidgetNode};

/// Manages keyboard focus across the widget tree.
pub struct FocusManager {
    /// Currently focused widget, if any.
    current_focus: Option<WidgetId>,
    /// Ordered list of all focusable widget IDs (by tab_index then DFS order).
    tab_order: Vec<WidgetId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            current_focus: None,
            tab_order: Vec::new(),
        }
    }

    /// Rebuild the tab-order list by traversing the widget tree.
    /// Sorts by tab_index, preserving DFS order for equal indices.
    pub fn rebuild(&mut self, root: &WidgetNode) {
        let mut pairs: Vec<(u16, WidgetId)> = Vec::new();
        self.collect_focusable(root, &mut pairs);

        // Stable sort by tab_index — DFS order preserved for equal indices
        pairs.sort_by_key(|(tab_idx, _)| *tab_idx);
        self.tab_order = pairs.into_iter().map(|(_, id)| id).collect();

        // If current_focus is not in new tab_order, clear it
        if let Some(id) = self.current_focus {
            if !self.tab_order.contains(&id) {
                self.current_focus = None;
            }
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
    /// Returns the new focused widget ID, or None if nothing is focusable.
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
    /// Returns the new focused widget ID, or None if nothing is focusable.
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

    /// Number of focusable widgets in the current tab order.
    pub fn len(&self) -> usize {
        self.tab_order.len()
    }

    /// Whether there are no focusable widgets.
    pub fn is_empty(&self) -> bool {
        self.tab_order.is_empty()
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

    /// Tab index for focus ordering. Lower values receive focus first.
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

    pub fn children(&self) -> &[WidgetNode] {
        match self {
            WidgetNode::Box(w) => &w.children,
            WidgetNode::Tabs(_w) => &[],
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
        })
    }

    fn make_button(id: u64, label: &str) -> WidgetNode {
        WidgetNode::Button(ButtonWidget {
            id: WidgetId(id),
            props: LayoutProps::default(),
            label: label.to_string(),
            style: ButtonStyle::Default,
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
        fm.next(); // advance to second
        let third = fm.next().unwrap(); // should wrap to first
        assert_eq!(third, first);
    }

    #[test]
    fn prev_wraps_around_to_last() {
        let root = make_box(0, vec![make_input(1), make_button(2, "OK")]);
        let mut fm = FocusManager::new();
        fm.rebuild(&root);

        // With no current focus, prev selects last
        let last = fm.prev().unwrap();
        assert_eq!(last, WidgetId(2));
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
        fm.next(); // now focused on 1

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
    fn single_focusable_next_prev_stays_same() {
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

        // Rebuild with different tree — widget 1 gone
        let new_root = make_box(0, vec![make_button(2, "OK")]);
        fm.rebuild(&new_root);
        assert_eq!(fm.current(), None, "focus should be cleared when widget is removed");
    }
}
