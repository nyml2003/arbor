// Structured error types for arbor-tui-core.
// Library crate → thiserror, no anyhow.

use crate::widget::WidgetId;

/// Errors from the layout engine.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    /// A widget's constraints were not found in the constraints map.
    /// This means `layout_tree()` was called before or without `measure_tree()`.
    #[error("widget {0:?} has no measured constraints — was measure_tree() called?")]
    MissingConstraints(WidgetId),

    /// A widget's flex value is NaN — flex values must be finite.
    #[error("widget {0:?} has NaN flex value")]
    NaN(WidgetId),
}

/// Errors from the focus system.
#[derive(Debug, thiserror::Error)]
pub enum FocusError {
    /// A widget that should be focusable is not in the tab order.
    /// This means `rebuild()` was not called after tree mutation.
    #[error("widget {0:?} not found in tab order — was rebuild() called?")]
    NotInTabOrder(WidgetId),
}
