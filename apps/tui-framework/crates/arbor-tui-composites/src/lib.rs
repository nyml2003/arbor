//! Higher-level reusable TUI composites built from `arbor-tui-widgets`.
//!
//! This crate owns framework-level molecule components. It stays business
//! agnostic: applications provide state, text, callbacks, and semantics.

mod divider_block;
mod fuzzy_panel;
mod panel;
mod prompt_bar;
mod scroll_column;
mod section_divider;
mod sectioned_panel;
mod status_line;
mod transcript;

pub use divider_block::DividerBlock;
pub use fuzzy_panel::{FuzzyPanel, FuzzyPanelSelection};
pub use panel::Panel;
pub use prompt_bar::PromptBar;
pub use scroll_column::{ContentBlock, ScrollColumn};
pub use section_divider::SectionDivider;
pub use sectioned_panel::{SectionedPanel, SectionedPanelSection};
pub use status_line::StatusLine;
pub use transcript::{Transcript, TranscriptMessage, TranscriptNotice};

fn usize_to_u16_saturating(value: usize) -> u16 {
    value.min(u16::MAX as usize) as u16
}
