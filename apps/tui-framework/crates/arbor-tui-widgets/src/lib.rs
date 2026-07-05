// arbor-tui-widgets — built-in widget components.
// Widget structs are pub(crate); only Builders + config types are public.

pub mod border;
pub mod button;
pub mod input;
pub mod list;
pub mod rich_text;
pub mod scroll;
pub mod stack;
pub mod table;
pub mod tabs;
pub mod text;
pub mod widget_factory;

pub use button::widget::ButtonStyle;
pub use table::widget::{ColumnDef, ColumnWidth};
pub use tabs::widget::TabDef;
pub use text::widget::TextStyle;
