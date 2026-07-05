// Structured error types used by the layout engine.

use crate::widget_id::WidgetId;

/// Errors from the layout engine.
#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    /// A widget's constraints were not found in the constraints map.
    #[error("widget {0:?} has no measured constraints — was measure_tree() called?")]
    MissingConstraints(WidgetId),

    /// A widget's flex value is NaN — flex values must be finite.
    #[error("widget {0:?} has NaN flex value")]
    NaN(WidgetId),
}
