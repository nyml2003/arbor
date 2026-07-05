// arbor-tui-adapters — infrastructure adapters.
// Implements domain backend and input ports with crossterm or in-memory fakes.

pub mod error;
pub mod simulated_backend;
pub mod simulated_input;

#[cfg(feature = "crossterm")]
pub mod crossterm_backend;

#[cfg(feature = "crossterm")]
pub mod stdin_reader;
