// arbor-tui-backend — infrastructure adapters.
// arbor-tui-core domain traits implemented with crossterm + simulated backends.

pub mod error;
pub mod simulated_backend;

#[cfg(feature = "crossterm")]
pub mod crossterm_backend;

#[cfg(feature = "crossterm")]
pub mod stdin_reader;
