// SimulatedInput — in-memory keyboard input for testing.
// Implements InputReader. Tests push KeyEvents into a queue;
// poll / poll_timeout / read_blocking drain the queue as if from a real terminal.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use arbor_tui_domain::input::{InputReader, KeyEvent};

/// Shared event queue behind an Arc so the harness can push while the
/// reader trait methods drain.
#[derive(Clone)]
struct SharedQueue {
    queue: Arc<(Mutex<VecDeque<Vec<KeyEvent>>>, Condvar)>,
}

impl SharedQueue {
    fn new() -> Self {
        Self {
            queue: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
        }
    }

    fn push_batch(&self, events: Vec<KeyEvent>) {
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        q.push_back(events);
        cvar.notify_one();
    }

    fn drain(&self) -> Vec<KeyEvent> {
        let (lock, _cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        let mut out = Vec::new();
        while let Some(batch) = q.pop_front() {
            out.extend(batch);
        }
        out
    }

    fn drain_timeout(&self, timeout: Duration) -> Vec<KeyEvent> {
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        if q.is_empty() {
            let result = cvar.wait_timeout(q, timeout).unwrap();
            q = result.0;
            // Timed out or notified — drain whatever we have
        }
        let mut out = Vec::new();
        while let Some(batch) = q.pop_front() {
            out.extend(batch);
        }
        out
    }

    fn drain_blocking(&self) -> Vec<KeyEvent> {
        let (lock, cvar) = &*self.queue;
        let mut q = lock.lock().unwrap();
        while q.is_empty() {
            q = cvar.wait(q).unwrap();
        }
        let mut out = Vec::new();
        while let Some(batch) = q.pop_front() {
            out.extend(batch);
        }
        out
    }

    fn notify_all(&self) {
        let (_lock, cvar) = &*self.queue;
        cvar.notify_all();
    }
}

/// An in-memory keyboard input source for testing.
///
/// Tests push [`KeyEvent`] batches via [`SimulatedInput::push`], then the
/// framework's event loop drains them through the [`InputReader`] trait methods.
///
/// # Example
///
/// ```ignore
/// let input = SimulatedInput::new();
/// input.push(KeyEvent::char('a'));
/// input.push(KeyEvent::char('/'));
///
/// let events = input.poll(); // drains both events
/// assert_eq!(events.len(), 2);
/// ```
pub struct SimulatedInput {
    queue: SharedQueue,
    shutdown: Arc<Mutex<bool>>,
}

impl SimulatedInput {
    pub fn new() -> Self {
        Self {
            queue: SharedQueue::new(),
            shutdown: Arc::new(Mutex::new(false)),
        }
    }

    /// Push a single key event into the queue. Wakes a blocked `read_blocking`
    /// or `poll_timeout` if one is waiting.
    pub fn push(&self, event: KeyEvent) {
        self.queue.push_batch(vec![event]);
    }

    /// Push a batch of key events (e.g. a multi-char paste).
    pub fn push_batch(&self, events: impl IntoIterator<Item = KeyEvent>) {
        let batch: Vec<KeyEvent> = events.into_iter().collect();
        if !batch.is_empty() {
            self.queue.push_batch(batch);
        }
    }
}

impl InputReader for SimulatedInput {
    fn poll(&self) -> Vec<KeyEvent> {
        self.queue.drain()
    }

    fn poll_timeout(&self, timeout: Duration) -> Vec<KeyEvent> {
        self.queue.drain_timeout(timeout)
    }

    fn read_blocking(&self) -> KeyEvent {
        loop {
            let mut events = self.queue.drain_blocking();
            if let Some(first) = events.pop() {
                // Put remaining events back so they're not lost
                if !events.is_empty() {
                    self.queue.push_batch(events);
                }
                return first;
            }
            // Spurious wakeup — loop and wait again
        }
    }

    fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
        self.queue.notify_all();
    }
}

impl Default for SimulatedInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_domain::input::{Key, KeyEvent};

    #[test]
    fn poll_drains_pushed_events() {
        let input = SimulatedInput::new();
        input.push(KeyEvent::char('a'));
        input.push(KeyEvent::char('b'));

        let events = input.poll();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].key, Key::Char('a'));
        assert_eq!(events[1].key, Key::Char('b'));

        // Queue is now empty
        assert!(input.poll().is_empty());
    }

    #[test]
    fn poll_timeout_returns_immediately_when_events_queued() {
        let input = SimulatedInput::new();
        input.push(KeyEvent::char('x'));

        let events = input.poll_timeout(Duration::from_secs(10));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn poll_timeout_returns_empty_on_timeout() {
        let input = SimulatedInput::new();
        let events = input.poll_timeout(Duration::from_millis(1));
        assert!(events.is_empty());
    }

    #[test]
    fn read_blocking_gets_pushed_event() {
        let input = SimulatedInput::new();
        input.push(KeyEvent::char('z'));

        let event = input.read_blocking();
        assert_eq!(event.key, Key::Char('z'));
    }

    #[test]
    fn push_batch_inserts_multiple_events() {
        let input = SimulatedInput::new();
        input.push_batch([
            KeyEvent::char('1'),
            KeyEvent::char('2'),
            KeyEvent::char('3'),
        ]);

        let events = input.poll();
        assert_eq!(events.len(), 3);
    }
}
