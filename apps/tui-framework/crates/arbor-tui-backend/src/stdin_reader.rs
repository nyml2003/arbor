// StdinReader — keyboard input via crossterm event::read().
// Runs a dedicated thread for blocking stdin reads, sends KeyEvents via mpsc.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::time::Duration;
use std::thread::{self, JoinHandle};

use crossterm::event::{read, Event, KeyCode, KeyModifiers};

use arbor_tui_core::input::{InputReader, Key, KeyEvent, Modifiers};

/// Bounded channel capacity per TEP-0004: 256 batches max.
const EVENT_CHANNEL_CAP: usize = 256;

pub struct StdinReader {
    event_rx: Receiver<Vec<KeyEvent>>,
    shutdown_tx: SyncSender<()>,
    handle: Option<JoinHandle<()>>,
}

impl StdinReader {
    /// Create a new StdinReader with a dedicated polling thread.
    ///
    /// The thread blocks on `crossterm::event::read()` and pushes KeyEvents
    /// through an mpsc channel. A shutdown channel allows graceful termination.
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::sync_channel::<Vec<KeyEvent>>(EVENT_CHANNEL_CAP);
        let (shutdown_tx, shutdown_rx) = mpsc::sync_channel::<()>(1);

        let handle = thread::spawn(move || {
            loop {
                // Check for shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    break;
                }

                // Blocking read — crossterm handles timeout internally
                match read() {
                    Ok(event) => {
                        let key_event = map_crossterm_event(event);
                        if let Some(ke) = key_event {
                            // Bounded channel (256): drop event if buffer is full.
                            // Main thread polls every 100ms, so buffer never fills
                            // under normal human input rates.
                            let _ = event_tx.try_send(vec![ke]);
                        }
                    }
                    Err(_) => {
                        // Read error — break the loop
                        break;
                    }
                }
            }
        });

        Self {
            event_rx,
            shutdown_tx,
            handle: Some(handle),
        }
    }
}

impl InputReader for StdinReader {
    fn poll(&self) -> Vec<KeyEvent> {
        let mut events = Vec::new();
        // Drain all available events without blocking
        loop {
            match self.event_rx.try_recv() {
                Ok(batch) => events.extend(batch),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        events
    }

    fn poll_timeout(&self, timeout: Duration) -> Vec<KeyEvent> {
        match self.event_rx.recv_timeout(timeout) {
            Ok(batch) => batch,
            Err(RecvTimeoutError::Timeout) => Vec::new(),
            Err(RecvTimeoutError::Disconnected) => Vec::new(),
        }
    }

    fn read_blocking(&self) -> KeyEvent {
        // Block until at least one event arrives
        loop {
            match self.event_rx.recv() {
                Ok(mut batch) => {
                    if let Some(first) = batch.pop() {
                        return first;
                    }
                }
                Err(_) => {
                    // Channel disconnected — return a dummy event
                    return KeyEvent::char(' ');
                }
            }
        }
    }

    fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

impl Drop for StdinReader {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Map a crossterm Event to the framework's KeyEvent.
/// Returns None for events that should be filtered out (mouse, focus, unsupported keys).
fn map_crossterm_event(event: Event) -> Option<KeyEvent> {
    match event {
        Event::Key(key) => {
            let k = match key.code {
                KeyCode::Char(c) => Key::Char(c),
                KeyCode::Enter => Key::Enter,
                KeyCode::Tab => Key::Tab,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Esc => Key::Escape,
                KeyCode::Up => Key::ArrowUp,
                KeyCode::Down => Key::ArrowDown,
                KeyCode::Left => Key::ArrowLeft,
                KeyCode::Right => Key::ArrowRight,
                KeyCode::Home => Key::Home,
                KeyCode::End => Key::End,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::PageDown => Key::PageDown,
                KeyCode::Insert => Key::Insert,
                KeyCode::Delete => Key::Delete,
                KeyCode::F(n) => Key::F(n),
                _ => return None, // Filter unsupported keys
            };

            let modifiers = Modifiers {
                ctrl: key.modifiers.contains(KeyModifiers::CONTROL),
                alt: key.modifiers.contains(KeyModifiers::ALT),
                shift: key.modifiers.contains(KeyModifiers::SHIFT),
            };

            Some(KeyEvent { key: k, modifiers })
        }
        Event::Resize(w, h) => {
            // Resize events are handled separately via the SignalManager.
            // For now, return None — the app layer checks crossterm size directly.
            let _ = (w, h);
            None
        }
        _ => None, // Mouse, focus, paste — all filtered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_char_event() {
        let crossterm_key = crossterm::event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let result = map_crossterm_event(Event::Key(crossterm_key));
        assert!(result.is_some());
        let ke = result.unwrap();
        assert_eq!(ke.key, Key::Char('a'));
        assert!(!ke.modifiers.ctrl);
    }

    #[test]
    fn map_ctrl_c() {
        let crossterm_key = crossterm::event::KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        );
        let result = map_crossterm_event(Event::Key(crossterm_key));
        assert!(result.is_some());
        let ke = result.unwrap();
        assert_eq!(ke.key, Key::Char('c'));
        assert!(ke.modifiers.ctrl);
    }

    #[test]
    fn map_mouse_event_is_filtered() {
        use crossterm::event::MouseEventKind;
        let mouse = crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let result = map_crossterm_event(Event::Mouse(mouse));
        assert!(result.is_none());
    }
}
