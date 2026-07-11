//! Crossterm-backed presentation and input for Punctum surfaces.

#![forbid(unsafe_code)]

mod cell;
mod input;
mod plan;
mod runtime;
mod text;

pub use cell::{TerminalCell, TerminalCellError, TerminalColor};
pub use input::{normalize_key_event, normalize_text_event};
pub use plan::{TerminalPlan, TerminalPlanError, TerminalRun, plan_patch};
pub use runtime::{TerminalPresentError, TerminalPresenter, TerminalSession};
pub use text::{TerminalTextError, resize_text_surface, write_text};
