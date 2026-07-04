// arbor-tui-widget — Widget Component Protocol.
// Widget trait, WidgetNode, layout engine, render engine, focus management.
//
// Re-exports commonly-needed types so widget authors only need one import.

pub mod widget;
pub mod layout_engine;
pub mod render;
pub mod focus;

// Convenience re-exports from upstream crates
pub use arbor_tui_primitives::widget_id::{WidgetAction, WidgetId, WidgetLayoutInfo};
pub use arbor_tui_primitives::layout_error::LayoutError;
