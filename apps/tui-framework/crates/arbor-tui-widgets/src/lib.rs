// arbor-tui-widgets — built-in widget components.
//
// Each widget is a struct + impl Widget trait. Adding a new widget
// requires ZERO changes to arbor-tui-core — just impl the trait.
//
// Layout:
//   box_widget    — flex container, transparent (no visual)
//   text_widget   — styled text with word wrap and truncation
//   input_widget  — single-line text input
//   button_widget — clickable button with style variants
//   list_widget   — scrollable item list
//   table_widget  — columnar table with header
//   tabs_widget   — tabbed container
//   scroll_widget — scrollable viewport over a child

pub mod box_widget;
pub mod text_widget;
pub mod input_widget;
pub mod button_widget;
pub mod list_widget;
pub mod table_widget;
pub mod tabs_widget;
pub mod scroll_widget;
