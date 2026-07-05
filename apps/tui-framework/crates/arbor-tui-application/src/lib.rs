// arbor-tui-application — runtime orchestration.
// Reads domain inputs, mutates App state, and returns render decisions.

pub mod app;
pub mod event_loop;
pub mod runtime;
pub mod signal_manager;
pub mod terminal;

pub use arbor_tui_domain::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
