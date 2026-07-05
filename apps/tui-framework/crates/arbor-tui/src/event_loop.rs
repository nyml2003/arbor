// Event loop — the main application loop.
// Reads input events with blocking timeout, dispatches, triggers render.
// Errors from the render pipeline are logged (eprintln) rather than panicking.

use std::time::Duration;

use arbor_tui_primitives::input::{InputReader, Key, KeyEvent};
use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::focus::mount_tree;
use arbor_tui_widget::widget::{WidgetAction, WidgetNode};

use crate::app::App;
use crate::runtime::{runtime_step, RuntimeInput};

/// Merge consecutive duplicate events per the rules in TEP-0004.
pub fn merge_events(events: &[KeyEvent]) -> Vec<KeyEvent> {
    if events.is_empty() {
        return vec![];
    }
    let mut merged: Vec<KeyEvent> = Vec::with_capacity(events.len());
    merged.push(events[0].clone());
    for next in &events[1..] {
        let last = merged
            .last()
            .expect("merged is non-empty after initial push");
        if can_merge(last, next) {
            *merged.last_mut().expect("merged is non-empty") = next.clone();
        } else {
            merged.push(next.clone());
        }
    }
    merged
}

fn can_merge(a: &KeyEvent, b: &KeyEvent) -> bool {
    use arbor_tui_primitives::input::Key;
    if a.key != b.key || a.modifiers != b.modifiers {
        return false;
    }
    match &a.key {
        Key::ArrowUp
        | Key::ArrowDown
        | Key::ArrowLeft
        | Key::ArrowRight
        | Key::PageUp
        | Key::PageDown
        | Key::Char(_) => true,
        Key::Enter
        | Key::Tab
        | Key::Backspace
        | Key::Escape
        | Key::Home
        | Key::End
        | Key::Insert
        | Key::Delete
        | Key::F(_) => false,
    }
}

/// Blocking poll — waits up to 100ms for input, then returns.
pub fn poll_events(input: &dyn InputReader) -> Vec<KeyEvent> {
    input.poll_timeout(Duration::from_millis(100))
}

/// Map a KeyEvent to a WidgetAction.
///
/// This is the single place where physical keys are bound to logical actions.
/// Widgets receive ONLY actions — they never see KeyEvents.
/// Applications can override, extend, or replace this mapping entirely.
pub fn default_keymap(event: &KeyEvent) -> Option<WidgetAction> {
    if event.modifiers.ctrl || event.modifiers.alt {
        // Modifier chords are handled at the app level (e.g. Ctrl+C → quit),
        // not mapped to widget actions.
        return None;
    }
    match &event.key {
        Key::ArrowUp => Some(WidgetAction::NavigateUp),
        Key::ArrowDown => Some(WidgetAction::NavigateDown),
        Key::ArrowLeft => Some(WidgetAction::NavigateLeft),
        Key::ArrowRight => Some(WidgetAction::NavigateRight),
        Key::Enter => Some(WidgetAction::Activate),
        Key::Escape => Some(WidgetAction::Cancel),
        Key::Home => Some(WidgetAction::Home),
        Key::End => Some(WidgetAction::End),
        Key::PageUp => Some(WidgetAction::PageUp),
        Key::PageDown => Some(WidgetAction::PageDown),
        Key::Delete => Some(WidgetAction::Delete),
        Key::Backspace => Some(WidgetAction::Backspace),
        Key::Tab => None, // handled by focus system, not widgets
        Key::Char(c) => Some(WidgetAction::TypeChar(*c)),
        Key::Insert => None,
        Key::F(_) => None,
    }
}

/// Run the main event loop.
///
/// Errors from the render pipeline are printed to stderr and the loop continues.
/// Only fatal backend errors (e.g., terminal disconnected) cause the loop to exit.
pub fn run_event_loop(
    app: &mut App,
    root: &mut WidgetNode,
    input: &dyn InputReader,
    backend: &mut dyn TerminalBackend,
    theme: &Theme,
) {
    app.run();
    mount_tree(root);

    let mut first_frame = true;
    while app.is_running() {
        let events = poll_events(input);
        let input = if first_frame {
            RuntimeInput {
                events,
                first_frame: true,
                resize: None,
            }
        } else {
            RuntimeInput::new(events)
        };

        let step = match runtime_step(app, root, backend, input) {
            Ok(step) => step,
            Err(e) => {
                eprintln!("[arbor-tui] runtime step failed: {e:?}");
                app.quit();
                break;
            }
        };

        if step.should_clear {
            let _ = backend.clear();
        }

        if step.should_render {
            match app.render_widget_tree(root, theme, backend) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[arbor-tui] render failed: {e:?}");
                    app.quit();
                    break;
                }
            }
        }
        first_frame = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_primitives::input::{Key, KeyEvent, KeyEventKind, Modifiers};

    fn ke(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }
    }

    #[test]
    fn merge_repeated_arrows() {
        let events = vec![ke(Key::ArrowUp), ke(Key::ArrowUp), ke(Key::ArrowUp)];
        assert_eq!(merge_events(&events).len(), 1);
    }

    #[test]
    fn dont_merge_enter() {
        let events = vec![ke(Key::Enter), ke(Key::Enter)];
        assert_eq!(merge_events(&events).len(), 2);
    }

    #[test]
    fn chain_break_on_different_keys() {
        let events = vec![
            KeyEvent::char('a'),
            KeyEvent::char('a'),
            ke(Key::ArrowUp),
            KeyEvent::char('a'),
        ];
        assert_eq!(merge_events(&events).len(), 3);
    }
}
