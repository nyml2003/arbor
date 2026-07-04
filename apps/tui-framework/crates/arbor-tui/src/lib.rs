// arbor-tui — public API re-exports + App runtime.

pub mod app;
pub mod event_loop;
pub mod signal_manager;
pub mod terminal;

// Re-export common types for single-import convenience
pub use arbor_tui_backend::crossterm_backend::CrosstermBackend;
pub use arbor_tui_backend::simulated_backend::SimulatedBackend;
pub use arbor_tui_backend::stdin_reader::StdinReader;
pub use arbor_tui_core::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
