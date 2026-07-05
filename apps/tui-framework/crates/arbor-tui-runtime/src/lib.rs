// arbor-tui-runtime — high-level runtime facades.
// Composes application runtime with concrete adapters for app entry points.

pub mod terminal_app;

pub use arbor_tui_application::terminal_app::{TerminalApp, TerminalAppResult};
pub use terminal_app::run_crossterm_terminal_app;
