mod app;
mod backend;
mod element;
mod host;
mod input;
mod layout;
mod paint;
mod screen;
mod theme;

pub use app::{AppContext, DirtyKind, FrameInvalidation, ThornApp};
pub use backend::{
    BackendCapabilities, BackendError, BackendFeature, BackendPresenter, PresentedFrame,
    UnsupportedBackendFeature,
};
pub use element::{
    border, clip, column, layer, row, scroll_view, text, view, Axis, BorderElement, Element,
    ElementNode, IntoChildren, LayerElement, StackElement, TextElement, ViewElement,
};
pub use host::{lower_element, HostKind, HostNode, HostNodeId};
pub use input::{
    BackendEventSource, BackendInputEvent, BackendKey, BackendKeyEvent, BoundedInputQueue,
    ControlKeyAction, DefaultKeyMap, Direction, EmacsTextKeyMap, FocusedControlKind,
    InputShutdownSignal, InputThreadDriver, InputThreadStep, IntentContext, IntentMapper,
    IntentResolver, Key, KeyAction, KeyEvent, KeyEventKind, KeyIntent, KeyMap, KeyMapError,
    KeyMapLayer, KeyMapLayerKind, KeyMapResult, KeyModifiers, LayeredKeyMap,
    LayeredKeyMapResolution, PlatformFallbackKeyMap, ReadOnlyNavigationKeyMap, RuntimeInput,
    TextInputKeyMap, VimNavigationKeyMap,
};
pub use layout::{
    layout_tree, layout_tree_with_metrics, text_display_width, BackendMetrics, CrossAxisAlignment,
    LayoutConstraints, LayoutNode, LayoutOverflow, LayoutStyle, MainAxisAlignment, Margin, Padding,
    Rect, ScrollOffset, Size, TextMetrics,
};
pub use paint::{
    paint_tree, paint_tree_with_theme, PaintAttrs, PaintColor, PaintPrimitive, PaintStyle,
};
pub use screen::{
    diff_screens, render_pipeline, render_pipeline_with_theme, render_to_screen,
    render_to_screen_with_theme, Cell, CellAttrs, CellPatch, Color, DirtyRegion, RenderedFrame,
    Screen, ScreenPatch, WideCell,
};
pub use theme::Theme;
