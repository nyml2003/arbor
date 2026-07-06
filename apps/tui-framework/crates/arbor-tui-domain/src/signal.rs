// Signal — fine-grained reactive values with explicit subscription.
// Uses Rc<RefCell<SignalInner<T>>> to share state between Signal (write handle)
// and ReadSignal (read handle). Single-threaded, no atomics.
//
// Signal<T>:     Writable, held by business layer. Components NEVER hold this.
// ReadSignal<T>: Read-only view, held by components. Has `.get()` directly.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::dirty::DirtyTracker;
use crate::identity::DirtyKind;
use crate::widget_id::WidgetId;

thread_local! {
    static NEXT_SIGNAL_ID: Cell<u64> = Cell::new(1);
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SignalId(u64);

impl SignalId {
    pub fn get(self) -> u64 {
        self.0
    }
}

pub trait SignalSource {
    fn id(&self) -> SignalId;
    fn generation(&self) -> u64;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SignalDep {
    pub signal_id: SignalId,
    pub generation: u64,
    pub dirty_kind: DirtyKind,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct SignalSubscriber {
    widget_id: WidgetId,
    dirty_kind: DirtyKind,
}

/// Internal state shared between Signal and all its ReadSignal clones.
struct SignalInner<T: Clone + PartialEq> {
    id: SignalId,
    value: T,
    subscribers: Vec<SignalSubscriber>,
    generation: u64,
}

/// Writable reactive value. Owned by the business layer.
///
/// Components receive a `ReadSignal<T>` view via `signal.read_only()`.
/// `Signal::set()` requires an explicit dirty tracker so state writes stay
/// visible to tests and application runtime code.
pub struct Signal<T: Clone + PartialEq> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: Clone + PartialEq> Signal<T> {
    /// Create a new Signal with the given initial value.
    pub fn new(initial: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SignalInner {
                id: next_signal_id(),
                value: initial,
                subscribers: Vec::new(),
                generation: 0,
            })),
        }
    }

    /// Read the current value. Pure, no side effects.
    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    /// Set a new value. If the value actually changes (per `PartialEq`),
    /// increment the generation counter and mark all subscribers dirty.
    ///
    /// The `PartialEq` check prevents infinite loops in input-component
    /// scenarios where the same value is written back.
    pub fn set(&self, new_value: T, dirty: &mut DirtyTracker) {
        let mut inner = self.inner.borrow_mut();
        if new_value != inner.value {
            inner.value = new_value;
            inner.generation += 1;
            for subscriber in &inner.subscribers {
                dirty.mark_dirty_kind(subscriber.widget_id, subscriber.dirty_kind);
            }
        }
    }

    /// Create a read-only view for passing to components.
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            inner: Rc::clone(&self.inner),
        }
    }

    /// Subscribe a widget to value changes.
    pub fn subscribe(&self, widget_id: WidgetId) {
        self.subscribe_with_dirty_kind(widget_id, DirtyKind::Render);
    }

    /// Subscribe a widget with the dirty level declared by its SignalDep.
    pub fn subscribe_with_dirty_kind(&self, widget_id: WidgetId, dirty_kind: DirtyKind) {
        let mut inner = self.inner.borrow_mut();
        if let Some(subscriber) = inner
            .subscribers
            .iter_mut()
            .find(|subscriber| subscriber.widget_id == widget_id)
        {
            subscriber.dirty_kind = subscriber.dirty_kind.merge(dirty_kind);
        } else {
            inner.subscribers.push(SignalSubscriber {
                widget_id,
                dirty_kind,
            });
        }
    }

    /// Unsubscribe a widget.
    pub fn unsubscribe(&self, widget_id: WidgetId) {
        let mut inner = self.inner.borrow_mut();
        inner
            .subscribers
            .retain(|subscriber| subscriber.widget_id != widget_id);
    }

    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    /// Current generation counter — incremented on each change.
    pub fn generation(&self) -> u64 {
        self.inner.borrow().generation
    }

    /// Snapshot of current subscriber list.
    pub fn subscribers(&self) -> Vec<WidgetId> {
        self.inner
            .borrow()
            .subscribers
            .iter()
            .map(|subscriber| subscriber.widget_id)
            .collect()
    }

    pub fn subscriber_dirty_kind(&self, widget_id: WidgetId) -> Option<DirtyKind> {
        self.inner
            .borrow()
            .subscribers
            .iter()
            .find(|subscriber| subscriber.widget_id == widget_id)
            .map(|subscriber| subscriber.dirty_kind)
    }
}

impl<T: Clone + PartialEq> SignalSource for Signal<T> {
    fn id(&self) -> SignalId {
        self.id()
    }

    fn generation(&self) -> u64 {
        self.generation()
    }
}

/// Read-only reactive value — components consume this.
///
/// Has `.get()` directly — no need to go through App's signal store.
/// Cloning a `ReadSignal` is cheap (Rc::clone).
#[derive(Clone)]
pub struct ReadSignal<T: Clone + PartialEq> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: Clone + PartialEq> ReadSignal<T> {
    /// Create a constant read signal that never changes.
    /// Use this for static text ("labels", "help") that doesn't need a backing Signal.
    pub fn constant(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SignalInner {
                id: next_signal_id(),
                value,
                subscribers: Vec::new(),
                generation: 0,
            })),
        }
    }

    /// Read the current value from the shared inner.
    pub fn get(&self) -> T {
        self.inner.borrow().value.clone()
    }

    /// Current generation counter.
    pub fn generation(&self) -> u64 {
        self.inner.borrow().generation
    }

    pub fn id(&self) -> SignalId {
        self.inner.borrow().id
    }

    pub fn dep(&self, dirty_kind: DirtyKind) -> SignalDep {
        SignalDep {
            signal_id: self.id(),
            generation: self.generation(),
            dirty_kind,
        }
    }

    /// Subscribe a widget to the source signal.
    pub fn subscribe(&self, widget_id: WidgetId) {
        self.subscribe_with_dirty_kind(widget_id, DirtyKind::Render);
    }

    /// Subscribe a widget with the dirty level declared by its SignalDep.
    pub fn subscribe_with_dirty_kind(&self, widget_id: WidgetId, dirty_kind: DirtyKind) {
        let mut inner = self.inner.borrow_mut();
        if let Some(subscriber) = inner
            .subscribers
            .iter_mut()
            .find(|subscriber| subscriber.widget_id == widget_id)
        {
            subscriber.dirty_kind = subscriber.dirty_kind.merge(dirty_kind);
        } else {
            inner.subscribers.push(SignalSubscriber {
                widget_id,
                dirty_kind,
            });
        }
    }

    /// Unsubscribe a widget from the source signal.
    pub fn unsubscribe(&self, widget_id: WidgetId) {
        let mut inner = self.inner.borrow_mut();
        inner
            .subscribers
            .retain(|subscriber| subscriber.widget_id != widget_id);
    }
}

impl<T: Clone + PartialEq> SignalSource for ReadSignal<T> {
    fn id(&self) -> SignalId {
        self.id()
    }

    fn generation(&self) -> u64 {
        self.generation()
    }
}

fn next_signal_id() -> SignalId {
    NEXT_SIGNAL_ID.with(|next| {
        let id = next.get();
        let next_id = id.checked_add(1).expect("signal id counter overflowed");
        next.set(next_id);
        SignalId(id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_set_changes_value() {
        let s = Signal::new(42);
        let mut dt = DirtyTracker::new();
        assert_eq!(s.get(), 42);
        s.set(99, &mut dt);
        assert_eq!(s.get(), 99);
    }

    #[test]
    fn signal_set_same_value_no_change() {
        let s = Signal::new(42);
        let mut dt = DirtyTracker::new();
        let gen_before = s.generation();
        s.set(42, &mut dt); // same value
        assert_eq!(s.generation(), gen_before); // no increment
    }

    #[test]
    fn signal_set_marks_subscribers_dirty() {
        let s = Signal::new("hello".to_string());
        let mut dt = DirtyTracker::new();
        let id = WidgetId(1);
        s.subscribe(id);
        assert_eq!(s.subscribers().len(), 1);

        s.set("world".to_string(), &mut dt);
        assert!(
            dt.is_dirty(id),
            "subscriber should be marked dirty after set"
        );
    }

    #[test]
    fn signal_set_uses_subscriber_dirty_kind() {
        let s = Signal::new("hello".to_string());
        let mut dt = DirtyTracker::new();
        s.subscribe_with_dirty_kind(WidgetId(1), DirtyKind::Layout);

        s.set("world".to_string(), &mut dt);

        let dirty = dt.drain();
        assert_eq!(dirty[&WidgetId(1)], DirtyKind::Layout);
    }

    #[test]
    fn signal_dep_captures_id_generation_and_dirty_kind() {
        let s = Signal::new("hello".to_string());
        let r = s.read_only();

        let dep = r.dep(DirtyKind::Structure);

        assert_eq!(dep.signal_id, s.id());
        assert_eq!(dep.generation, s.generation());
        assert_eq!(dep.dirty_kind, DirtyKind::Structure);
    }

    #[test]
    fn signal_id_is_stable_per_handle_and_unique_between_signals() {
        let left = Signal::new(1);
        let right = Signal::new(1);
        let left_read = left.read_only();

        assert_eq!(left.id(), left_read.id());
        assert_ne!(left.id(), right.id());
    }

    #[test]
    fn signal_subscribe_and_unsubscribe() {
        let s = Signal::new("hello".to_string());
        let id = WidgetId(1);
        s.subscribe(id);
        assert_eq!(s.subscribers().len(), 1);
        assert_eq!(s.subscriber_dirty_kind(id), Some(DirtyKind::Render));
        s.unsubscribe(id);
        assert_eq!(s.subscribers().len(), 0);
    }

    #[test]
    fn read_signal_get_reflects_source() {
        let s = Signal::new(10);
        let mut dt = DirtyTracker::new();
        let r = s.read_only();
        assert_eq!(r.get(), 10);
        s.set(20, &mut dt);
        assert_eq!(r.get(), 20);
    }

    #[test]
    fn read_only_is_cloneable() {
        let s = Signal::new(10);
        let r = s.read_only();
        let r2 = r.clone();
        assert_eq!(r.get(), r2.get());
    }

    #[test]
    fn constant_read_signal() {
        let r: ReadSignal<String> = ReadSignal::constant("static".to_string());
        assert_eq!(r.get(), "static");
        // generation never changes
        assert_eq!(r.generation(), 0);
    }

    #[test]
    fn read_signal_subscribe_through_read_handle() {
        let s = Signal::new("test".to_string());
        let mut dt = DirtyTracker::new();
        let r = s.read_only();
        r.subscribe(WidgetId(7));
        // Setting through Signal should mark WidgetId(7) dirty
        s.set("changed".to_string(), &mut dt);
        assert!(dt.is_dirty(WidgetId(7)));
    }
}
