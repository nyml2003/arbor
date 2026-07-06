use crate::widget_id::WidgetId;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DirtyKind {
    Render,
    Layout,
    Structure,
    Theme,
    Full,
}

impl DirtyKind {
    pub fn merge(self, other: Self) -> Self {
        self.max(other)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WidgetKey(String);

impl WidgetKey {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "widget key must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for WidgetKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for WidgetKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for WidgetKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum NodeIdentity {
    Keyed(WidgetKey),
    Path(Vec<u16>),
}

#[derive(Clone, PartialEq, Eq, Debug, thiserror::Error)]
pub enum IdentityError {
    #[error(
        "duplicate widget key {key:?} under parent {parent:?}: child {first_index} and child {second_index}"
    )]
    DuplicateSiblingKey {
        parent: WidgetId,
        key: WidgetKey,
        first_index: usize,
        second_index: usize,
    },
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ReconcileReport {
    pub reused: usize,
    pub replaced: usize,
    pub mounted: usize,
    pub unmounted: usize,
    pub dirty: Vec<(WidgetId, DirtyKind)>,
    pub focus_dirty: bool,
}

impl ReconcileReport {
    pub fn record_dirty(&mut self, id: WidgetId, kind: DirtyKind) {
        self.dirty.push((id, kind));
        if matches!(kind, DirtyKind::Structure | DirtyKind::Full) {
            self.focus_dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_kind_merge_keeps_strongest_level() {
        assert_eq!(
            DirtyKind::Render.merge(DirtyKind::Layout),
            DirtyKind::Layout
        );
        assert_eq!(
            DirtyKind::Structure.merge(DirtyKind::Theme),
            DirtyKind::Theme
        );
        assert_eq!(DirtyKind::Full.merge(DirtyKind::Render), DirtyKind::Full);
    }

    #[test]
    fn structure_dirty_marks_focus_dirty() {
        let mut report = ReconcileReport::default();

        report.record_dirty(WidgetId(7), DirtyKind::Structure);

        assert!(report.focus_dirty);
        assert_eq!(report.dirty, vec![(WidgetId(7), DirtyKind::Structure)]);
    }
}
