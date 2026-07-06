use std::collections::HashMap;

use crate::layout::{Rect, Size, SizeConstraint};
use crate::screen::VirtualScreen;
use crate::widget_id::{WidgetId, WidgetLayoutInfo};

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FocusState {
    Focused,
    Unfocused,
}

impl FocusState {
    pub fn from_focus(widget: WidgetId, focused: Option<WidgetId>) -> Self {
        if focused == Some(widget) {
            Self::Focused
        } else {
            Self::Unfocused
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MeasureCacheEntry {
    pub available: Size,
    pub props_hash: u64,
    pub children_measure_hash: u64,
    pub output: SizeConstraint,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LayoutCacheEntry {
    pub parent_rect: Rect,
    pub props_hash: u64,
    pub children_measure_hash: u64,
    pub output: WidgetLayoutInfo,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RenderCacheEntry {
    pub rect: Rect,
    pub theme_rev: u64,
    pub focus_state: FocusState,
    pub render_rev: u64,
    pub screen: VirtualScreen,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub mismatches: usize,
}

impl CacheStats {
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn record_mismatch(&mut self) {
        self.mismatches += 1;
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayoutCacheShadow {
    measure: HashMap<WidgetId, MeasureCacheEntry>,
    layout: HashMap<WidgetId, LayoutCacheEntry>,
    pub measure_stats: CacheStats,
    pub layout_stats: CacheStats,
}

impl LayoutCacheShadow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_measure(&mut self, id: WidgetId, entry: MeasureCacheEntry) {
        match self.measure.get(&id) {
            Some(previous) if previous == &entry => self.measure_stats.record_hit(),
            Some(_) => {
                self.measure_stats.record_mismatch();
                self.measure.insert(id, entry);
            }
            None => {
                self.measure_stats.record_miss();
                self.measure.insert(id, entry);
            }
        }
    }

    pub fn observe_layout(&mut self, id: WidgetId, entry: LayoutCacheEntry) {
        match self.layout.get(&id) {
            Some(previous) if previous == &entry => self.layout_stats.record_hit(),
            Some(_) => {
                self.layout_stats.record_mismatch();
                self.layout.insert(id, entry);
            }
            None => {
                self.layout_stats.record_miss();
                self.layout.insert(id, entry);
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderCacheShadow {
    entries: HashMap<WidgetId, RenderCacheEntry>,
    pub stats: CacheStats,
}

impl RenderCacheShadow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, id: WidgetId, entry: RenderCacheEntry) {
        match self.entries.get(&id) {
            Some(previous) if previous == &entry => self.stats.record_hit(),
            Some(_) => {
                self.stats.record_mismatch();
                self.entries.insert(id, entry);
            }
            None => {
                self.stats.record_miss();
                self.entries.insert(id, entry);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::AxisConstraint;

    #[test]
    fn layout_shadow_records_hit_miss_and_mismatch() {
        let mut shadow = LayoutCacheShadow::new();
        let entry = MeasureCacheEntry {
            available: Size::new(10, 2),
            props_hash: 1,
            children_measure_hash: 2,
            output: SizeConstraint {
                min_w: 1,
                min_h: 1,
                max_w: AxisConstraint::Fixed(10),
                max_h: AxisConstraint::Fixed(2),
            },
        };

        shadow.observe_measure(WidgetId(1), entry.clone());
        shadow.observe_measure(WidgetId(1), entry.clone());
        shadow.observe_measure(
            WidgetId(1),
            MeasureCacheEntry {
                props_hash: 3,
                ..entry
            },
        );

        assert_eq!(
            shadow.measure_stats,
            CacheStats {
                hits: 1,
                misses: 1,
                mismatches: 1,
            }
        );
    }

    #[test]
    fn render_shadow_records_equal_fragments_as_hits() {
        let mut shadow = RenderCacheShadow::new();
        let entry = RenderCacheEntry {
            rect: Rect::new(0, 0, 3, 1),
            theme_rev: 1,
            focus_state: FocusState::Unfocused,
            render_rev: 1,
            screen: VirtualScreen::new(3, 1),
        };

        shadow.observe(WidgetId(1), entry.clone());
        shadow.observe(WidgetId(1), entry);

        assert_eq!(
            shadow.stats,
            CacheStats {
                hits: 1,
                misses: 1,
                mismatches: 0,
            }
        );
    }
}
