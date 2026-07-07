use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_EFFECT_ID: AtomicUsize = AtomicUsize::new(1);

thread_local! {
    static ACTIVE_EFFECT: RefCell<Vec<Rc<EffectInner>>> = const { RefCell::new(Vec::new()) };
    static CURRENT_SCOPE: RefCell<Vec<Scope>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone)]
pub struct Scope {
    inner: Rc<ScopeInner>,
}

struct ScopeInner {
    effects: RefCell<Vec<Rc<EffectInner>>>,
    cleanups: RefCell<Vec<Box<dyn FnOnce()>>>,
    disposed: Cell<bool>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(ScopeInner {
                effects: RefCell::new(Vec::new()),
                cleanups: RefCell::new(Vec::new()),
                disposed: Cell::new(false),
            }),
        }
    }

    pub fn enter<R>(&self, run: impl FnOnce() -> R) -> R {
        CURRENT_SCOPE.with(|current| current.borrow_mut().push(self.clone()));
        let result = run();
        CURRENT_SCOPE.with(|current| {
            current.borrow_mut().pop();
        });
        result
    }

    pub(crate) fn current() -> Option<Self> {
        CURRENT_SCOPE.with(|current| current.borrow().last().cloned())
    }

    pub fn create_signal<T>(&self, value: T) -> Signal<T> {
        Signal {
            inner: Rc::new(SignalInner {
                value: RefCell::new(value),
                subscribers: RefCell::new(Vec::new()),
            }),
        }
    }

    pub fn create_effect(&self, effect: impl FnMut() + 'static) {
        if self.inner.disposed.get() {
            return;
        }

        let effect = Rc::new(EffectInner {
            id: NEXT_EFFECT_ID.fetch_add(1, Ordering::Relaxed),
            callback: RefCell::new(Box::new(effect)),
            dependencies: RefCell::new(Vec::new()),
            alive: Cell::new(true),
        });

        self.inner.effects.borrow_mut().push(effect.clone());
        EffectInner::run(effect);
    }

    pub fn on_cleanup(&self, cleanup: impl FnOnce() + 'static) {
        if self.inner.disposed.get() {
            cleanup();
            return;
        }

        self.inner.cleanups.borrow_mut().push(Box::new(cleanup));
    }

    pub fn dispose(&self) {
        if self.inner.disposed.replace(true) {
            return;
        }

        for effect in self.inner.effects.borrow_mut().drain(..).rev() {
            effect.dispose();
        }

        for cleanup in self.inner.cleanups.borrow_mut().drain(..).rev() {
            cleanup();
        }
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ScopeInner {
    fn drop(&mut self) {
        self.disposed.set(true);
        for effect in self.effects.borrow_mut().drain(..).rev() {
            effect.dispose();
        }
        for cleanup in self.cleanups.borrow_mut().drain(..).rev() {
            cleanup();
        }
    }
}

#[derive(Clone)]
pub struct Signal<T> {
    inner: Rc<SignalInner<T>>,
}

#[derive(Clone)]
pub struct ReadSignal<T> {
    inner: Rc<SignalInner<T>>,
}

struct SignalInner<T> {
    value: RefCell<T>,
    subscribers: RefCell<Vec<Weak<EffectInner>>>,
}

impl<T> Signal<T> {
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Clone + 'static> Signal<T> {
    pub fn get(&self) -> T {
        track(&self.inner);
        self.inner.value.borrow().clone()
    }
}

impl<T: Clone + PartialEq + 'static> Signal<T> {
    pub fn set(&self, value: T) {
        let changed = {
            let mut current = self.inner.value.borrow_mut();
            if *current == value {
                false
            } else {
                *current = value;
                true
            }
        };

        if changed {
            notify(&self.inner);
        }
    }

    pub fn update(&self, update: impl FnOnce(&mut T)) {
        let next = {
            let mut value = self.inner.value.borrow().clone();
            update(&mut value);
            value
        };
        self.set(next);
    }
}

impl<T: Clone + 'static> ReadSignal<T> {
    pub fn get(&self) -> T {
        track(&self.inner);
        self.inner.value.borrow().clone()
    }
}

fn track<T: 'static>(signal: &Rc<SignalInner<T>>) {
    ACTIVE_EFFECT.with(|active| {
        if let Some(effect) = active.borrow().last() {
            signal.subscribers.borrow_mut().push(Rc::downgrade(effect));
            let weak_signal = Rc::downgrade(signal);
            let effect_id = effect.id;
            effect.dependencies.borrow_mut().push(Box::new(move || {
                if let Some(signal) = weak_signal.upgrade() {
                    signal.subscribers.borrow_mut().retain(|subscriber| {
                        subscriber
                            .upgrade()
                            .map(|effect| effect.id != effect_id)
                            .unwrap_or(false)
                    });
                }
            }));
        }
    });
}

fn notify<T>(signal: &SignalInner<T>) {
    let subscribers: Vec<_> = signal
        .subscribers
        .borrow()
        .iter()
        .filter_map(Weak::upgrade)
        .collect();

    for effect in subscribers {
        EffectInner::run(effect);
    }
}

struct EffectInner {
    id: usize,
    callback: RefCell<Box<dyn FnMut()>>,
    dependencies: RefCell<Vec<Box<dyn Fn()>>>,
    alive: Cell<bool>,
}

impl EffectInner {
    fn run(effect: Rc<Self>) {
        if !effect.alive.get() {
            return;
        }

        for remove_dependency in effect.dependencies.borrow_mut().drain(..) {
            remove_dependency();
        }

        ACTIVE_EFFECT.with(|active| active.borrow_mut().push(effect.clone()));
        (effect.callback.borrow_mut())();
        ACTIVE_EFFECT.with(|active| {
            active.borrow_mut().pop();
        });
    }

    fn dispose(&self) {
        self.alive.set(false);
        for remove_dependency in self.dependencies.borrow_mut().drain(..) {
            remove_dependency();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn signal_set_triggers_dependent_effect() {
        let scope = Scope::new();
        let count = scope.create_signal(0usize);
        let seen = Rc::new(Cell::new(0usize));
        let seen_in_effect = seen.clone();
        let count_in_effect = count.clone();

        scope.create_effect(move || {
            seen_in_effect.set(count_in_effect.get());
        });

        count.set(1);
        assert_eq!(seen.get(), 1);
    }

    #[test]
    fn same_value_set_does_not_trigger_effect() {
        let scope = Scope::new();
        let count = scope.create_signal(0usize);
        let runs = Rc::new(Cell::new(0usize));
        let runs_in_effect = runs.clone();
        let count_in_effect = count.clone();

        scope.create_effect(move || {
            count_in_effect.get();
            runs_in_effect.set(runs_in_effect.get() + 1);
        });

        count.set(0);
        assert_eq!(runs.get(), 1);
    }

    #[test]
    fn effect_rerun_cleans_old_dependencies() {
        let scope = Scope::new();
        let use_a = scope.create_signal(true);
        let a = scope.create_signal(0usize);
        let b = scope.create_signal(0usize);
        let runs = Rc::new(Cell::new(0usize));
        let runs_in_effect = runs.clone();
        let use_a_in_effect = use_a.clone();
        let a_in_effect = a.clone();
        let b_in_effect = b.clone();

        scope.create_effect(move || {
            runs_in_effect.set(runs_in_effect.get() + 1);
            if use_a_in_effect.get() {
                a_in_effect.get();
            } else {
                b_in_effect.get();
            }
        });

        use_a.set(false);
        a.set(1);
        assert_eq!(runs.get(), 2);
        b.set(1);
        assert_eq!(runs.get(), 3);
    }

    #[test]
    fn nested_effect_restores_active_stack() {
        let scope = Scope::new();
        let outer_signal = scope.create_signal(0usize);
        let inner_signal = scope.create_signal(0usize);
        let outer_runs = Rc::new(Cell::new(0usize));
        let inner_runs = Rc::new(Cell::new(0usize));
        let outer_runs_in_effect = outer_runs.clone();
        let inner_runs_in_effect = inner_runs.clone();
        let outer_in_effect = outer_signal.clone();
        let inner_in_effect = inner_signal.clone();
        let scope_in_effect = scope.clone();

        scope.create_effect(move || {
            outer_in_effect.get();
            outer_runs_in_effect.set(outer_runs_in_effect.get() + 1);
            let inner = inner_in_effect.clone();
            let runs = inner_runs_in_effect.clone();
            scope_in_effect.create_effect(move || {
                inner.get();
                runs.set(runs.get() + 1);
            });
        });

        inner_signal.set(1);
        assert_eq!(outer_runs.get(), 1);
        assert_eq!(inner_runs.get(), 2);
    }

    #[test]
    fn scope_dispose_stops_effects_and_runs_cleanups_in_reverse_order() {
        let scope = Scope::new();
        let count = scope.create_signal(0usize);
        let seen = Rc::new(Cell::new(0usize));
        let order = Rc::new(RefCell::new(Vec::new()));
        let seen_in_effect = seen.clone();
        let count_in_effect = count.clone();
        let first = order.clone();
        let second = order.clone();

        scope.create_effect(move || {
            seen_in_effect.set(count_in_effect.get());
        });
        scope.on_cleanup(move || first.borrow_mut().push(1));
        scope.on_cleanup(move || second.borrow_mut().push(2));

        scope.dispose();
        count.set(1);

        assert_eq!(seen.get(), 0);
        assert_eq!(*order.borrow(), vec![2, 1]);
    }
}
