//! Crossterm-backed presentation and input for Punctum surfaces.

#![forbid(unsafe_code)]

mod cell;
mod input;
mod plan;
mod runtime;

pub use cell::{TerminalCell, TerminalColor};
pub use input::normalize_key_event;
pub use plan::{TerminalPlanError, TerminalRun, plan_patch};
pub use runtime::{TerminalPresentError, TerminalPresenter, TerminalSession};
