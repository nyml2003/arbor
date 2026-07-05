//! Application-facing facade for arbor-tui.
//!
//! Use `arbor_tui::prelude::*` for ordinary applications. Lower-level crates
//! remain available for framework internals and advanced custom widgets.

pub mod app;
pub mod prelude;
pub mod testing;
pub mod ui;

pub use app::{AppContext, ArborApp};
pub use ui::{Node, Ui};
