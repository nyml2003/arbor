// arbor-tui-primitives — Shared Kernel for the TUI framework.
// Pure data types and traits with zero I/O dependencies (except unicode-width).
// Every other crate in the framework depends on this one.

pub mod cell;
pub mod layout;
pub mod input;
pub mod text;
pub mod widget_id;
pub mod layout_error;

#[cfg(feature = "profile")]
pub mod events;
