//! Application-facing facade for arbor-tui.
//!
//! Use `arbor_tui::prelude::*` for ordinary applications. Lower-level crates
//! remain available for framework internals and advanced custom widgets.

pub mod app;
pub mod component;
pub mod prelude;
pub mod testing;
pub mod ui;

pub use app::{AppContext, ArborApp};
pub use arbor_tui_application::app::App;
pub use arbor_tui_domain::input::{KeyEvent, KeyEventKind};
pub use arbor_tui_domain::signal::{ReadSignal, Signal};
pub use component::{
    Col, ColProps, ComponentProps, FuzzyPanel, FuzzyPanelProps, Input, InputProps, Page, PageProps,
    Panel, PanelProps, PromptBar, PromptBarProps, PropsComponent, Row, RowProps, StatusLine,
    StatusLineProps, TextBlock, TextBlockProps, Transcript, TranscriptProps, UiComponent,
};
pub use ui::{Node, Ui};

pub mod advanced {
    pub use arbor_tui_composites::*;
    pub use arbor_tui_domain::widget::WidgetNode;
    pub use arbor_tui_widgets::*;
}
