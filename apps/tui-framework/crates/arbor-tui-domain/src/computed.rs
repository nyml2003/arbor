// ComputedSignal — lazy derived value with explicit dependency generations.
//
// The runtime supplies the dependency generation vector from a frame snapshot
// or a component protocol. The computed node only recomputes when that vector
// changes, and its own generation changes only when the derived value changes.

#[derive(Clone, Debug)]
pub struct ComputedSignal<T> {
    value: Option<T>,
    generation: u64,
    dep_generations: Vec<u64>,
    recomputations: usize,
}

impl<T> Default for ComputedSignal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ComputedSignal<T> {
    pub fn new() -> Self {
        Self {
            value: None,
            generation: 0,
            dep_generations: Vec::new(),
            recomputations: 0,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn recomputations(&self) -> usize {
        self.recomputations
    }

    pub fn is_initialized(&self) -> bool {
        self.value.is_some()
    }
}

impl<T: PartialEq> ComputedSignal<T> {
    pub fn get_or_compute(
        &mut self,
        dep_generations: &[u64],
        compute: impl FnOnce() -> T,
    ) -> ComputedRead<'_, T> {
        let should_recompute =
            self.value.is_none() || self.dep_generations.as_slice() != dep_generations;
        let mut recomputed = false;
        let mut generation_changed = false;

        if should_recompute {
            let next = compute();
            let value_changed = self.value.as_ref() != Some(&next);
            if value_changed {
                self.value = Some(next);
                self.generation += 1;
                generation_changed = true;
            }
            self.dep_generations = dep_generations.to_vec();
            self.recomputations += 1;
            recomputed = true;
        }

        ComputedRead {
            value: self
                .value
                .as_ref()
                .expect("computed value should be initialized after get_or_compute"),
            generation: self.generation,
            recomputed,
            generation_changed,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ComputedRead<'a, T> {
    value: &'a T,
    generation: u64,
    recomputed: bool,
    generation_changed: bool,
}

impl<'a, T> ComputedRead<'a, T> {
    pub fn value(&self) -> &'a T {
        self.value
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn recomputed(&self) -> bool {
        self.recomputed
    }

    pub fn generation_changed(&self) -> bool {
        self.generation_changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_recomputes_lazily_when_deps_change() {
        let mut computed = ComputedSignal::new();
        let mut calls = 0;

        let first = computed.get_or_compute(&[0, 1], || {
            calls += 1;
            42
        });
        assert_eq!(*first.value(), 42);
        assert!(first.recomputed());
        assert!(first.generation_changed());
        assert_eq!(first.generation(), 1);

        let second = computed.get_or_compute(&[0, 1], || {
            calls += 1;
            100
        });
        assert_eq!(*second.value(), 42);
        assert!(!second.recomputed());
        assert!(!second.generation_changed());
        assert_eq!(calls, 1);

        let third = computed.get_or_compute(&[0, 2], || {
            calls += 1;
            100
        });
        assert_eq!(*third.value(), 100);
        assert!(third.recomputed());
        assert!(third.generation_changed());
        assert_eq!(third.generation(), 2);
        assert_eq!(calls, 2);
    }

    #[test]
    fn computed_updates_deps_without_bumping_generation_when_output_is_equal() {
        let mut computed = ComputedSignal::new();

        computed.get_or_compute(&[1], || "same".to_string());
        let read = computed.get_or_compute(&[2], || "same".to_string());

        assert!(read.recomputed());
        assert!(!read.generation_changed());
        assert_eq!(read.generation(), 1);
        assert_eq!(computed.recomputations(), 2);
    }
}
