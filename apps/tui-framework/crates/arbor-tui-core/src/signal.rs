// Signal — fine-grained reactive values with explicit subscription.
// Uses Rc<RefCell<SignalInner<T>>> to share state between Signal (write handle)
// and ReadSignal (read handle). Single-threaded, no atomics.
//
// Signal<T>:     Writable, held by business layer. Components NEVER hold this.
// ReadSignal<T>: Read-only view, held by components. Has `.get()` directly.

use std::cell::RefCell;
use std::rc::Rc;

use crate::dirty::DirtyTracker;
use crate::widget::WidgetId;

/// Internal state shared between Signal and all its ReadSignal clones.
struct SignalInner<T: Clone + PartialEq> {
    value: T,
    subscribers: Vec<WidgetId>,
    generation: u64,
}

/// Writable reactive value. Owned by the business layer.
///
/// Components receive a `ReadSignal<T>` view via `signal.read_only()`.
/// `Signal::set()` requires `&mut DirtyTracker` — the caller is responsible
/// for passing the app's tracker. No global state.
pub struct Signal<T: Clone + PartialEq> {
    inner: Rc<RefCell<SignalInner<T>>>,
}

impl<T: Clone + PartialEq> Signal<T> {
    /// Create a new Signal with the given initial value.
    pub fn new(initial: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(SignalInner {
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
            for id in &inner.subscribers {
                dirty.mark_dirty(*id);
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
        let mut inner = self.inner.borrow_mut();
        if !inner.subscribers.contains(&widget_id) {
            inner.subscribers.push(widget_id);
        }
    }

    /// Unsubscribe a widget.
    pub fn unsubscribe(&self, widget_id: WidgetId) {
        let mut inner = self.inner.borrow_mut();
        inner.subscribers.retain(|id| *id != widget_id);
    }

    /// Current generation counter — incremented on each change.
    pub fn generation(&self) -> u64 {
        self.inner.borrow().generation
    }

    /// Snapshot of current subscriber list.
    pub fn subscribers(&self) -> Vec<WidgetId> {
        self.inner.borrow().subscribers.clone()
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

    /// Subscribe a widget to the source signal.
    pub fn subscribe(&self, widget_id: WidgetId) {
        let mut inner = self.inner.borrow_mut();
        if !inner.subscribers.contains(&widget_id) {
            inner.subscribers.push(widget_id);
        }
    }

    /// Unsubscribe a widget from the source signal.
    pub fn unsubscribe(&self, widget_id: WidgetId) {
        let mut inner = self.inner.borrow_mut();
        inner.subscribers.retain(|id| *id != widget_id);
    }
}

/// Convenience helper: from a `Signal<T>`, create a `(ReadSignal<T>, Box<dyn Fn(T)>)` pair.
///
/// The `ReadSignal` goes to the component. The write closure captures the signal's
/// inner state and sets the value directly — the caller must still call
/// `signal.set()` with a `DirtyTracker` to trigger re-render.
///
/// For full reactivity, use the returned `ReadSignal` in a widget and call
/// `signal.set(new_value, &mut app.dirty_tracker)` in the write closure.
pub fn bind_signal<T: Clone + PartialEq + 'static>(
    sig: &Signal<T>,
) -> (ReadSignal<T>, Box<dyn Fn(T)>) {
    let read = sig.read_only();
    let inner = Rc::clone(&sig.inner);
    let write: Box<dyn Fn(T)> = Box::new(move |v| {
        let mut data = inner.borrow_mut();
        if v != data.value {
            data.value = v;
            data.generation += 1;
            // Note: DirtyTracker notification happens via the generation bump.
            // The caller should drain generations in the event loop.
        }
    });
    (read, write)
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
        assert!(dt.is_dirty(id), "subscriber should be marked dirty after set");
    }

    #[test]
    fn signal_subscribe_and_unsubscribe() {
        let s = Signal::new("hello".to_string());
        let id = WidgetId(1);
        s.subscribe(id);
        assert_eq!(s.subscribers().len(), 1);
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
