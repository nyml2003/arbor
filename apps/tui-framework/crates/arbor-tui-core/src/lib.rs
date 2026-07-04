// arbor-tui-core — domain layer, zero external dependencies (except unicode-width)
//
// All types and traits in this crate are pure logic with no I/O.
// Infrastructure adapters live in arbor-tui-backend.

pub mod cell;
pub mod screen;
pub mod diff;
pub mod text;
pub mod backend;
pub mod layout;
pub mod layout_engine;
pub mod render;
pub mod signal;
pub mod dirty;
pub mod widget;
pub mod focus;
pub mod input;
pub mod theme;

#[cfg(feature = "profile")]
pub mod events;
