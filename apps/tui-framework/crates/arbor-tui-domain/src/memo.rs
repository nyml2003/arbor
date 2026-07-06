// MemoStore — dependency keyed memo policy for component-level useMemo.
//
// The store tracks dependency vectors, byte estimates, and eviction policy.
// It intentionally stores no UI object here; concrete render/layout caches must
// first pass shadow parity before reusing memoized output.

use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct MemoSlot(u64);

impl MemoSlot {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MemoRetention {
    Normal,
    Pinned,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MemoStatus {
    Hit,
    Miss,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct MemoStats {
    pub hits: usize,
    pub misses: usize,
    pub evictions: usize,
    pub entries: usize,
    pub bytes: usize,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct MemoEntry {
    deps: Vec<u64>,
    estimated_bytes: usize,
    last_used: u64,
    retention: MemoRetention,
}

#[derive(Clone, Debug)]
pub struct MemoStore {
    entries: HashMap<MemoSlot, MemoEntry>,
    max_entries: usize,
    max_bytes: usize,
    clock: u64,
    hits: usize,
    misses: usize,
    evictions: usize,
}

impl MemoStore {
    pub fn new(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            max_bytes: max_bytes.max(1),
            clock: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    pub fn use_memo(
        &mut self,
        slot: MemoSlot,
        deps: &[u64],
        estimated_bytes: usize,
        retention: MemoRetention,
    ) -> MemoStatus {
        self.clock = self.clock.saturating_add(1);

        if let Some(entry) = self.entries.get_mut(&slot) {
            entry.last_used = self.clock;
            entry.retention = retention;
            entry.estimated_bytes = estimated_bytes;
            if entry.deps.as_slice() == deps {
                self.hits += 1;
                self.evict_to_limits(slot);
                return MemoStatus::Hit;
            }
            entry.deps = deps.to_vec();
            self.misses += 1;
            self.evict_to_limits(slot);
            return MemoStatus::Miss;
        }

        self.entries.insert(
            slot,
            MemoEntry {
                deps: deps.to_vec(),
                estimated_bytes,
                last_used: self.clock,
                retention,
            },
        );
        self.misses += 1;
        self.evict_to_limits(slot);
        MemoStatus::Miss
    }

    pub fn contains(&self, slot: MemoSlot) -> bool {
        self.entries.contains_key(&slot)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current_bytes(&self) -> usize {
        self.entries
            .values()
            .map(|entry| entry.estimated_bytes)
            .sum()
    }

    pub fn stats(&self) -> MemoStats {
        MemoStats {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            entries: self.len(),
            bytes: self.current_bytes(),
        }
    }

    fn evict_to_limits(&mut self, protected: MemoSlot) {
        while self.len() > self.max_entries || self.current_bytes() > self.max_bytes {
            let candidate = self
                .entries
                .iter()
                .filter(|(slot, entry)| {
                    **slot != protected && entry.retention == MemoRetention::Normal
                })
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(slot, _)| *slot)
                .or_else(|| {
                    self.entries
                        .iter()
                        .filter(|(_, entry)| entry.retention == MemoRetention::Normal)
                        .min_by_key(|(_, entry)| entry.last_used)
                        .map(|(slot, _)| *slot)
                });

            let Some(candidate) = candidate else {
                break;
            };
            self.entries.remove(&candidate);
            self.evictions += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_deps_hit_after_initial_miss() {
        let mut store = MemoStore::new(4, 1024);
        let slot = MemoSlot::new(1);

        assert_eq!(
            store.use_memo(slot, &[1, 2], 10, MemoRetention::Normal),
            MemoStatus::Miss
        );
        assert_eq!(
            store.use_memo(slot, &[1, 2], 10, MemoRetention::Normal),
            MemoStatus::Hit
        );

        let stats = store.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn changed_deps_miss_without_adding_entry() {
        let mut store = MemoStore::new(4, 1024);
        let slot = MemoSlot::new(1);

        store.use_memo(slot, &[1], 10, MemoRetention::Normal);
        assert_eq!(
            store.use_memo(slot, &[2], 10, MemoRetention::Normal),
            MemoStatus::Miss
        );

        assert_eq!(store.len(), 1);
        assert_eq!(store.stats().misses, 2);
    }

    #[test]
    fn entry_limit_evicts_least_recently_used_normal_entry() {
        let mut store = MemoStore::new(2, 1024);
        let one = MemoSlot::new(1);
        let two = MemoSlot::new(2);
        let three = MemoSlot::new(3);

        store.use_memo(one, &[1], 10, MemoRetention::Normal);
        store.use_memo(two, &[2], 10, MemoRetention::Normal);
        store.use_memo(one, &[1], 10, MemoRetention::Normal);
        store.use_memo(three, &[3], 10, MemoRetention::Normal);

        assert!(store.contains(one));
        assert!(!store.contains(two));
        assert!(store.contains(three));
        assert_eq!(store.stats().evictions, 1);
    }

    #[test]
    fn byte_limit_evicts_until_under_budget() {
        let mut store = MemoStore::new(4, 30);
        let one = MemoSlot::new(1);
        let two = MemoSlot::new(2);
        let three = MemoSlot::new(3);

        store.use_memo(one, &[1], 15, MemoRetention::Normal);
        store.use_memo(two, &[2], 15, MemoRetention::Normal);
        store.use_memo(three, &[3], 15, MemoRetention::Normal);

        assert!(!store.contains(one));
        assert!(store.contains(two));
        assert!(store.contains(three));
        assert!(store.current_bytes() <= 30);
    }

    #[test]
    fn pinned_entries_survive_capacity_pressure() {
        let mut store = MemoStore::new(1, 1024);
        let pinned = MemoSlot::new(1);
        let normal = MemoSlot::new(2);

        store.use_memo(pinned, &[1], 10, MemoRetention::Pinned);
        store.use_memo(normal, &[2], 10, MemoRetention::Normal);

        assert!(store.contains(pinned));
        assert!(!store.contains(normal));
        assert_eq!(store.stats().evictions, 1);
    }
}
