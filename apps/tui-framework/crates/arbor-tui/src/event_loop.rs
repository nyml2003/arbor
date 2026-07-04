// Event loop — the main application loop.
// Reads input events with blocking timeout, dispatches, triggers render.
// Errors from the render pipeline are logged (eprintln) rather than panicking.

use std::time::Duration;

use arbor_tui_core::backend::TerminalBackend;
use arbor_tui_core::focus::mount_tree;
use arbor_tui_core::input::{InputReader, KeyEvent};
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::WidgetNode;

use crate::app::App;
use crate::signal_manager::check_resize;

/// Merge consecutive duplicate events per the rules in TEP-0004.
pub fn merge_events(events: &[KeyEvent]) -> Vec<KeyEvent> {
    if events.is_empty() {
        return vec![];
    }
    let mut merged: Vec<KeyEvent> = Vec::with_capacity(events.len());
    merged.push(events[0].clone());
    for next in &events[1..] {
        let last = merged.last().expect("merged is non-empty after initial push");
        if can_merge(last, next) {
            *merged.last_mut().expect("merged is non-empty") = next.clone();
        } else {
            merged.push(next.clone());
        }
    }
    merged
}

fn can_merge(a: &KeyEvent, b: &KeyEvent) -> bool {
    use arbor_tui_core::input::Key;
    if a.key != b.key || a.modifiers != b.modifiers { return false; }
    match &a.key {
        Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
        | Key::PageUp | Key::PageDown | Key::Char(_) => true,
        Key::Enter | Key::Tab | Key::Backspace | Key::Escape
        | Key::Home | Key::End | Key::Insert | Key::Delete | Key::F(_) => false,
    }
}

/// Blocking poll — waits up to 100ms for input, then returns.
pub fn poll_events(input: &dyn InputReader) -> Vec<KeyEvent> {
    input.poll_timeout(Duration::from_millis(100))
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
    app.run(backend);
    mount_tree(root);

    let mut first_frame = true;
    while app.is_running() {
        // SIGWINCH: detect terminal size change → force full relayout
        if let Err(e) = check_resize(app, backend) {
            eprintln!("[arbor-tui] resize check failed: {e}");
            app.quit();
            break;
        }

        let events = poll_events(input);
        if !events.is_empty() {
            let merged = merge_events(&events);
            for event in &merged {
                use arbor_tui_core::input::Key;
                match &event.key {
                    Key::Char('c') if event.modifiers.ctrl => app.quit(),
                    Key::Char('q') if event.modifiers.ctrl => app.quit(),
                    Key::Escape => app.quit(),
                    Key::Tab if event.modifiers.shift => {
                        if let Err(e) = app.focus_prev() {
                            eprintln!("[arbor-tui] focus_prev: {e}");
                        }
                    }
                    Key::Tab => {
                        if let Err(e) = app.focus_next() {
                            eprintln!("[arbor-tui] focus_next: {e}");
                        }
                    }
                    _ => app.dispatch_key(root, event),
                }
            }
        }

        if first_frame || !app.dirty_tracker.is_empty() {
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
    use arbor_tui_core::input::{Key, KeyEvent, Modifiers};

    #[test]
    fn merge_repeated_arrows() {
        let events = vec![
            KeyEvent { key: Key::ArrowUp, modifiers: Modifiers::default() },
            KeyEvent { key: Key::ArrowUp, modifiers: Modifiers::default() },
            KeyEvent { key: Key::ArrowUp, modifiers: Modifiers::default() },
        ];
        assert_eq!(merge_events(&events).len(), 1);
    }

    #[test]
    fn dont_merge_enter() {
        let events = vec![
            KeyEvent { key: Key::Enter, modifiers: Modifiers::default() },
            KeyEvent { key: Key::Enter, modifiers: Modifiers::default() },
        ];
        assert_eq!(merge_events(&events).len(), 2);
    }

    #[test]
    fn chain_break_on_different_keys() {
        let events = vec![
            KeyEvent::char('a'), KeyEvent::char('a'),
            KeyEvent { key: Key::ArrowUp, modifiers: Modifiers::default() },
            KeyEvent::char('a'),
        ];
        assert_eq!(merge_events(&events).len(), 3);
    }
}
