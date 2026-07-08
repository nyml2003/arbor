mod app;
mod backend;
mod element;
mod host;
mod input;
mod layout;
mod paint;
mod screen;

pub use app::{AppContext, Theme, ThornApp};
pub use backend::{
    BackendCapabilities, BackendError, BackendFeature, BackendPresenter, PresentedFrame,
    UnsupportedBackendFeature,
};
pub use element::{
    column, row, text, view, Axis, Element, ElementNode, IntoChildren, StackElement, TextElement,
    ViewElement,
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
pub use layout::{layout_tree, LayoutNode, Rect, Size};
pub use paint::{paint_tree, PaintPrimitive};
pub use screen::{
    diff_screens, render_pipeline, render_to_screen, Cell, CellAttrs, CellPatch, Color,
    DirtyRegion, RenderedFrame, Screen, ScreenPatch, WideCell,
};
