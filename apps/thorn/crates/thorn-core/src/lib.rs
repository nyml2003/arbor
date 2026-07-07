pub mod layout;
pub mod reactive;
pub mod render;
pub mod testing;
pub mod theme;
pub mod view;
pub mod widgets;

pub mod prelude {
    pub use crate::layout::{Align, Edge, Justify, Rect, Size};
    pub use crate::reactive::{ReadSignal, Scope, Signal};
    pub use crate::render::{diff, Cell, DirtyRegion, Screen};
    pub use crate::testing::TestApp;
    pub use crate::theme::{Color, ColorSource, Theme, Token};
    pub use crate::view::{ButtonPress, NodeId, View};
    pub use crate::widgets::{button, col, panel, row, text};
}
