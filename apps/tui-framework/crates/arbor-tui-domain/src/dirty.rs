// DirtyTracker — tracks which widgets need re-rendering.
// Owned by the App context, NOT a global static.

use crate::identity::DirtyKind;
use crate::widget_id::WidgetId;
use std::collections::HashMap;

/// Accumulates dirty widget IDs during an event processing cycle.
/// At the end of the cycle, `drain()` is called to get the set and clear it.
pub struct DirtyTracker {
    dirty_widgets: HashMap<WidgetId, DirtyKind>,
    /// When true, forces the next render regardless of dirty set.
    /// Set by SIGWINCH / SIGTSTP resume to ensure full relayout.
    force: bool,
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DirtyTracker {
    pub fn new() -> Self {
        Self {
            dirty_widgets: HashMap::new(),
            force: false,
        }
    }

    /// Mark a widget as needing re-render.
    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.mark_dirty_kind(widget_id, DirtyKind::Render);
    }

    pub fn mark_dirty_kind(&mut self, widget_id: WidgetId, kind: DirtyKind) {
        self.dirty_widgets
            .entry(widget_id)
            .and_modify(|existing| *existing = existing.merge(kind))
            .or_insert(kind);
    }

    /// Check if a widget is dirty.
    pub fn is_dirty(&self, widget_id: WidgetId) -> bool {
        self.dirty_widgets.contains_key(&widget_id)
    }

    /// Take all dirty widget IDs and reset the tracker.
    pub fn drain(&mut self) -> HashMap<WidgetId, DirtyKind> {
        self.force = false;
        std::mem::take(&mut self.dirty_widgets)
    }

    /// Mark all widgets dirty (used after SIGTSTP resume / SIGWINCH).
    pub fn mark_all(&mut self, widget_ids: &[WidgetId]) {
        for id in widget_ids {
            self.mark_dirty_kind(*id, DirtyKind::Full);
        }
    }

    /// Force the next render to proceed even if no individual widgets are dirty.
    /// Used for full relayout after terminal resize or SIGTSTP resume.
    pub fn force_render(&mut self) {
        self.force = true;
    }

    pub fn is_empty(&self) -> bool {
        self.dirty_widgets.is_empty() && !self.force
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_drain() {
        let mut dt = DirtyTracker::new();
        dt.mark_dirty(WidgetId(1));
        dt.mark_dirty_kind(WidgetId(2), DirtyKind::Layout);
        assert!(dt.is_dirty(WidgetId(1)));

        let drained = dt.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[&WidgetId(1)], DirtyKind::Render);
        assert_eq!(drained[&WidgetId(2)], DirtyKind::Layout);
        assert!(dt.is_empty());
    }

    #[test]
    fn dirty_kind_upgrades_existing_entry() {
        let mut dt = DirtyTracker::new();

        dt.mark_dirty_kind(WidgetId(1), DirtyKind::Render);
        dt.mark_dirty_kind(WidgetId(1), DirtyKind::Structure);

        let drained = dt.drain();
        assert_eq!(drained[&WidgetId(1)], DirtyKind::Structure);
    }

    #[test]
    fn force_render_bypasses_empty_check() {
        let mut dt = DirtyTracker::new();
        assert!(dt.is_empty());
        dt.force_render();
        assert!(!dt.is_empty(), "force should make is_empty() return false");
        let drained = dt.drain();
        assert!(
            drained.is_empty(),
            "drain with force should return empty set"
        );
        assert!(dt.is_empty(), "after drain, force should be cleared");
    }
}
