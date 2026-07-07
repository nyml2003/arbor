pub mod layout;
pub mod reactive;
pub mod render;
pub mod runtime;
pub mod testing;
pub mod theme;
pub mod view;
pub mod widgets;

pub mod prelude {
    pub use crate::layout::{Align, Edge, Justify, Rect, Size};
    pub use crate::reactive::{ReadSignal, Scope, Signal};
    pub use crate::render::{diff, Cell, DirtyRegion, Screen};
    pub use crate::runtime::{Key, KeyEvent, KeyEventKind, KeyMap, KeyModifiers, RuntimeInput};
    pub use crate::testing::{TestApp, TestRuntime};
    pub use crate::theme::{Color, ColorSource, Theme, Token};
    pub use crate::view::{FuzzyPanelSelection, NodeId, TranscriptMessage, TranscriptNotice, View};
    pub use crate::widgets::{
        col, fuzzy_panel, input, panel, row, scroll_area, text, transcript, transcript_line_count,
        FuzzyPanel, Input, ScrollArea, Transcript,
    };
}
