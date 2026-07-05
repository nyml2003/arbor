// Event loop — the main application loop.
// Reads input events with blocking timeout, dispatches, triggers render.
// Errors from the render pipeline are logged (eprintln) rather than panicking.

use std::time::Duration;

use arbor_tui_domain::backend::TerminalBackend;
use arbor_tui_domain::focus::mount_tree;
use arbor_tui_domain::input::{InputReader, KeyEvent};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;

use crate::app::App;
use crate::runtime::{runtime_step, RuntimeInput};

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
    app.run();
    mount_tree(root);

    let mut first_frame = true;
    while app.is_running() {
        let events = poll_events(input);
        let input = if first_frame {
            RuntimeInput::first_frame_with_events(events)
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
