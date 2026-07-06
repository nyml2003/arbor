// arbor-tui-application — runtime orchestration.
// Reads domain inputs, mutates App state, and returns render decisions.

pub mod app;
pub mod component_runtime;
pub mod dispatcher;
pub mod event_loop;
pub mod runtime;
pub mod signal_manager;
pub mod terminal;
pub mod terminal_app;

pub use arbor_tui_domain::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
