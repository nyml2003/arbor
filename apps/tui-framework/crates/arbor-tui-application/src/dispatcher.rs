// Action dispatcher bridge.
//
// Business dispatchers write Signals through a batch context. The context
// collects SignalChange records and applies them to App after dispatch returns,
// keeping mutation batching separate from rendering.

use arbor_tui_domain::signal::{Signal, SignalChange};

use crate::app::App;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ActionStatus {
    Ignored,
    Handled,
}

impl ActionStatus {
    pub fn is_handled(self) -> bool {
        matches!(self, Self::Handled)
    }
}

pub trait ActionDispatcher<Action> {
    fn dispatch(&mut self, action: &Action, ctx: &mut SignalDispatchBatch) -> ActionStatus;
}

#[derive(Default, Debug)]
pub struct SignalDispatchBatch {
    changes: Vec<SignalChange>,
}

impl SignalDispatchBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_signal<T: Clone + PartialEq>(&mut self, signal: &Signal<T>, value: T) -> bool {
        let Some(change) = signal.set_collect(value) else {
            return false;
        };
        self.changes.push(change);
        true
    }

    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn drain(self) -> Vec<SignalChange> {
        self.changes
    }

    pub fn apply_to(self, app: &mut App) {
        for change in self.changes {
            app.enqueue_signal_change(change);
        }
    }
}

pub fn dispatch_to_app<Action>(
    dispatcher: &mut impl ActionDispatcher<Action>,
    action: &Action,
    app: &mut App,
) -> ActionStatus {
    let mut batch = SignalDispatchBatch::new();
    let status = dispatcher.dispatch(action, &mut batch);
    batch.apply_to(app);
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::identity::DirtyKind;
    use arbor_tui_domain::widget::WidgetId;

    #[test]
    fn signal_batch_collects_changed_writes_only() {
        let signal = Signal::new(1);
        let mut batch = SignalDispatchBatch::new();

        assert!(!batch.set_signal(&signal, 1));
        assert!(batch.set_signal(&signal, 2));

        assert_eq!(batch.pending_changes(), 1);
        assert_eq!(signal.get(), 2);
    }

    #[test]
    fn batch_applies_signal_dirty_to_app_once() {
        let signal = Signal::new("before".to_string());
        signal.subscribe_with_dirty_kind(WidgetId(7), DirtyKind::Layout);
        let mut batch = SignalDispatchBatch::new();
        let mut app = App::new(20, 1);

        batch.set_signal(&signal, "middle".to_string());
        batch.set_signal(&signal, "after".to_string());
        batch.apply_to(&mut app);

        assert!(app.has_pending_render());
        assert_eq!(app.take_dirty_widgets(), vec![WidgetId(7)]);
    }

    enum CounterAction {
        Increment,
        Ignore,
    }

    struct CounterDispatcher {
        value: Signal<u64>,
    }

    impl ActionDispatcher<CounterAction> for CounterDispatcher {
        fn dispatch(
            &mut self,
            action: &CounterAction,
            ctx: &mut SignalDispatchBatch,
        ) -> ActionStatus {
            match action {
                CounterAction::Increment => {
                    let next = self.value.get() + 1;
                    ctx.set_signal(&self.value, next);
                    ActionStatus::Handled
                }
                CounterAction::Ignore => ActionStatus::Ignored,
            }
        }
    }

    #[test]
    fn dispatch_to_app_applies_batch_after_action() {
        let signal = Signal::new(0);
        signal.subscribe(WidgetId(1));
        let mut dispatcher = CounterDispatcher {
            value: signal.clone(),
        };
        let mut app = App::new(20, 1);

        let status = dispatch_to_app(&mut dispatcher, &CounterAction::Increment, &mut app);

        assert!(status.is_handled());
        assert_eq!(signal.get(), 1);
        assert_eq!(app.take_dirty_widgets(), vec![WidgetId(1)]);
    }

    #[test]
    fn ignored_action_can_leave_batch_empty() {
        let signal = Signal::new(0);
        let mut dispatcher = CounterDispatcher { value: signal };
        let mut app = App::new(20, 1);

        let status = dispatch_to_app(&mut dispatcher, &CounterAction::Ignore, &mut app);

        assert_eq!(status, ActionStatus::Ignored);
        assert!(!app.has_pending_render());
    }
}
