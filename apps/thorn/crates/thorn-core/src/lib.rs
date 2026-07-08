mod app;
mod backend;
mod element;
mod host;
mod input;
mod layout;
mod paint;
mod screen;

pub use app::{AppContext, ThornApp};
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
    BoundedInputQueue, ControlKeyAction, Direction, IntentMapper, Key, KeyAction, KeyEvent,
    KeyEventKind, KeyIntent, KeyMap, KeyMapError, KeyMapLayer, KeyMapResult, KeyModifiers,
    RuntimeInput,
};
pub use layout::{layout_tree, LayoutNode, Rect, Size};
pub use paint::{paint_tree, PaintPrimitive};
pub use screen::{
    diff_screens, render_pipeline, render_to_screen, Cell, CellPatch, RenderedFrame, Screen,
    ScreenPatch,
};
