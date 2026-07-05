// arbor-tui-domain — pure TUI domain model.
// Contains terminal cells, layout, rendering model, widget protocol, input,
// reactive state, and backend ports. No concrete terminal adapter lives here.

pub mod backend;
pub mod cell;
pub mod diff;
pub mod dirty;
pub mod focus;
pub mod input;
pub mod layout;
pub mod layout_engine;
pub mod layout_error;
pub mod render;
pub mod screen;
pub mod signal;
pub mod text;
pub mod theme;
pub mod widget;
pub mod widget_id;

#[cfg(feature = "profile")]
pub mod events;

pub use layout_error::LayoutError;
pub use widget_id::{WidgetAction, WidgetId, WidgetLayoutInfo};
