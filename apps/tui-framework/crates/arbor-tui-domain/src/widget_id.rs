// Cross-cutting types used by all bounded contexts.
// WidgetId, WidgetAction, WidgetLayoutInfo — pure data, no behavior.

use crate::layout::Rect;

/// Unique widget identifier — auto-assigned by the App on creation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WidgetId(pub u64);

/// Widget-level action — what a widget can DO, independent of any key binding.
///
/// Key→Action mapping is owned by the application layer (event_loop).
/// Widgets receive actions, not keys.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WidgetAction {
    NavigateUp,
    NavigateDown,
    NavigateLeft,
    NavigateRight,
    Activate,
    Cancel,
    Home,
    End,
    PageUp,
    PageDown,
    Delete,
    Backspace,
    TypeChar(char),
}

/// Layout info for a single widget — DTO between layout engine and render engine.
#[derive(Clone, Debug)]
pub struct WidgetLayoutInfo {
    pub id: WidgetId,
    /// Outer rect including margin.
    pub outer_rect: Rect,
    /// Inner content rect (minus padding) — where render() draws.
    pub content_rect: Rect,
}
