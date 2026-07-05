//! Higher-level reusable TUI composites built from `arbor-tui-widgets`.
//!
//! This crate owns framework-level molecule components. It stays business
//! agnostic: applications provide state, text, callbacks, and semantics.

mod panel;
mod prompt_bar;
mod scroll_column;
mod status_line;

pub use panel::Panel;
pub use prompt_bar::PromptBar;
pub use scroll_column::{ContentBlock, ScrollColumn};
pub use status_line::StatusLine;

fn usize_to_u16_saturating(value: usize) -> u16 {
    value.min(u16::MAX as usize) as u16
}
