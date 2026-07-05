// arbor-tui-primitives — Shared Kernel for the TUI framework.
// Pure data types and traits with zero I/O dependencies (except unicode-width).
// Every other crate in the framework depends on this one.

pub mod cell;
pub mod input;
pub mod layout;
pub mod layout_error;
pub mod text;
pub mod widget_id;

#[cfg(feature = "profile")]
pub mod events;
