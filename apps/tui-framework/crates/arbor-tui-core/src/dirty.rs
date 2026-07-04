// DirtyTracker — tracks which widgets need re-rendering.
// Owned by the App context, NOT a global static.

use std::collections::HashSet;
use crate::widget::WidgetId;

/// Accumulates dirty widget IDs during an event processing cycle.
/// At the end of the cycle, `drain()` is called to get the set and clear it.
pub struct DirtyTracker {
    dirty_widgets: HashSet<WidgetId>,
}

impl DirtyTracker {
    pub fn new() -> Self {
        Self {
            dirty_widgets: HashSet::new(),
        }
    }

    /// Mark a widget as needing re-render.
    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty_widgets.insert(widget_id);
    }

    /// Check if a widget is dirty.
    pub fn is_dirty(&self, widget_id: WidgetId) -> bool {
        self.dirty_widgets.contains(&widget_id)
    }

    /// Take all dirty widget IDs and reset the tracker.
    pub fn drain(&mut self) -> HashSet<WidgetId> {
        std::mem::take(&mut self.dirty_widgets)
    }

    /// Mark all widgets dirty (used after SIGTSTP resume / SIGWINCH).
    pub fn mark_all(&mut self, widget_ids: &[WidgetId]) {
        for id in widget_ids {
            self.dirty_widgets.insert(*id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.dirty_widgets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_drain() {
        let mut dt = DirtyTracker::new();
        dt.mark_dirty(WidgetId(1));
        dt.mark_dirty(WidgetId(2));
        assert!(dt.is_dirty(WidgetId(1)));

        let drained = dt.drain();
        assert_eq!(drained.len(), 2);
        assert!(dt.is_empty());
    }
}
