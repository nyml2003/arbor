use std::collections::HashMap;

use crate::signal::{SignalDep, SignalId};

#[derive(Clone, Debug, Default)]
pub struct FrameSnapshot {
    seq: u64,
    signal_generations: HashMap<SignalId, u64>,
}

impl FrameSnapshot {
    pub fn new(seq: u64, deps: impl IntoIterator<Item = SignalDep>) -> Self {
        let mut signal_generations: HashMap<SignalId, u64> = HashMap::new();
        for dep in deps {
            signal_generations
                .entry(dep.signal_id)
                .and_modify(|existing| *existing = (*existing).max(dep.generation))
                .or_insert(dep.generation);
        }
        Self {
            seq,
            signal_generations,
        }
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn signal_count(&self) -> usize {
        self.signal_generations.len()
    }

    pub fn generation(&self, signal_id: SignalId) -> Option<u64> {
        self.signal_generations.get(&signal_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::DirtyKind;
    use crate::signal::Signal;

    #[test]
    fn snapshot_records_signal_generation_by_id() {
        let signal = Signal::new("before".to_string());
        let dep = signal.read_only().dep(DirtyKind::Render);

        let snapshot = FrameSnapshot::new(7, [dep]);

        assert_eq!(snapshot.seq(), 7);
        assert_eq!(snapshot.signal_count(), 1);
        assert_eq!(snapshot.generation(signal.id()), Some(signal.generation()));
    }

    #[test]
    fn snapshot_deduplicates_signal_deps() {
        let signal = Signal::new("before".to_string());
        let read = signal.read_only();

        let snapshot = FrameSnapshot::new(
            1,
            [
                read.dep(DirtyKind::Render),
                read.dep(DirtyKind::Layout),
                read.dep(DirtyKind::Structure),
            ],
        );

        assert_eq!(snapshot.signal_count(), 1);
        assert_eq!(snapshot.generation(signal.id()), Some(0));
    }
}
