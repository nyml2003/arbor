// arbor-tui-widget — Widget Component Protocol.
// Widget trait, WidgetNode, layout engine, render engine, focus management.
//
// Re-exports commonly-needed types so widget authors only need one import.

pub mod focus;
pub mod layout_engine;
pub mod render;
pub mod widget;

// Convenience re-exports from upstream crates
pub use arbor_tui_primitives::layout_error::LayoutError;
pub use arbor_tui_primitives::widget_id::{WidgetAction, WidgetId, WidgetLayoutInfo};
