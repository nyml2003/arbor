// Widget trait and WidgetNode enum.
// All built-in components are variants of WidgetNode.

use crate::layout::{LayoutProps, Rect, Size, SizeConstraint};
use crate::screen::VirtualScreen;
use crate::input::{Key, KeyEvent, KeyHandleResult};
use crate::signal::ReadSignal;
use crate::theme::Theme;

/// Unique widget identifier — auto-assigned by the App on creation.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WidgetId(pub u64);

/// The Widget trait — all components implement this.
///
/// Every method has a default implementation, so user-defined widgets
/// only need to override the methods they use.
pub trait Widget {
    fn id(&self) -> WidgetId;
    fn layout_props(&self) -> &LayoutProps;
    fn children(&self) -> &[WidgetNode] { &[] }

    /// Pass 1 of layout: report size constraints.
    fn measure(&self, _available: Size) -> SizeConstraint { SizeConstraint::unbounded() }

    /// Render this widget into a VirtualScreen for the given content rect.
    fn render(&self, _rect: Rect, _theme: &Theme) -> VirtualScreen {
        VirtualScreen::new(_rect.w, _rect.h)
    }

    // Focus / input
    fn focusable(&self) -> bool { false }
    fn tab_index(&self) -> u16 { 0 }
    fn on_key(&mut self, _event: &KeyEvent) -> KeyHandleResult { KeyHandleResult::Bubble }

    // Lifecycle
    fn on_mount(&mut self) {}
    fn on_unmount(&mut self) {}
}

/// Homogeneous widget tree node.
///
/// Uses an enum rather than trait objects — the set of built-in components
/// is fixed (9 types), so compile-time monomorphization wins over vtable dispatch.
/// Generic List/Table use `dyn Any` type erasure.
pub enum WidgetNode {
    Box(BoxWidget),
    Text(TextWidget),
    Input(InputWidget),
    Button(ButtonWidget),
    List(ListWidget),
    Table(TableWidget),
    Tabs(TabsWidget),
    ScrollView(Box<ScrollViewWidget>),
}

// ── Built-in widget structs ──────────────────────────────────────

pub struct BoxWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub children: Vec<WidgetNode>,
}

pub struct TextWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub text: ReadSignal<String>, // 只读文本内容 — 通过 signal.read_only() 获取
    pub style: TextStyle,
    pub wrap: crate::text::WrapStrategy,
    pub truncate: crate::text::TruncateStrategy,
}

#[derive(Clone)]
pub struct TextStyle {
    pub fg: crate::cell::AnsiColor,
    pub bg: crate::cell::AnsiColor,
    pub attrs: crate::cell::Attrs,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            fg: crate::cell::AnsiColor::default(),
            bg: crate::cell::AnsiColor { palette: crate::cell::PaletteColor(0), true_color: None },
            attrs: crate::cell::Attrs::default(),
        }
    }
}

pub struct InputWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub buffer: String,          // internal editing buffer
    pub cursor: usize,           // cursor position within buffer
    pub placeholder: String,
    pub password: bool,
    /// Called on each keystroke with the new buffer content.
    pub on_change: Option<Box<dyn Fn(String)>>,
    /// Called on Enter with the current buffer content.
    pub on_submit: Option<Box<dyn Fn(String)>>,
}

pub struct ButtonWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub label: String,
    pub style: ButtonStyle,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ButtonStyle { Primary, Secondary, Danger, Default }

pub struct ListWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    /// Pre-rendered items as strings. The signal system pushes updated strings.
    pub items: Vec<String>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
}

pub struct TableWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub columns: Vec<ColumnDef>,
    /// Pre-rendered cell strings, row-major: rows[0][col0], rows[0][col1], ...
    pub cells: Vec<Vec<String>>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
}

pub struct ColumnDef {
    pub header: String,
    pub width: ColumnWidth,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ColumnWidth {
    Fixed(u16),
    Flex(f32),
}

pub struct TabsWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub tabs: Vec<TabDef>,
    pub active: usize,
}

pub struct TabDef {
    pub label: String,
    pub content: WidgetNode,
}

pub struct ScrollViewWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub child: Box<WidgetNode>,
    pub scroll_x: u16,
    pub scroll_y: u16,
}

// ── Widget trait impls for built-in types ────────────────────────

macro_rules! impl_widget_for {
    ($ty:ty, $id:ident) => {
        impl Widget for $ty {
            fn id(&self) -> WidgetId { self.id }
            fn layout_props(&self) -> &LayoutProps { &self.props }
        }
    };
}

impl_widget_for!(TextWidget, id);
impl_widget_for!(ButtonWidget, id);
impl_widget_for!(TabsWidget, id);
impl_widget_for!(ListWidget, id);
impl_widget_for!(TableWidget, id);

impl Widget for BoxWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn children(&self) -> &[WidgetNode] { &self.children }
}

impl Widget for InputWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn focusable(&self) -> bool { true }

    fn on_key(&mut self, event: &KeyEvent) -> KeyHandleResult {
        match &event.key {
            Key::Enter => {
                if let Some(ref cb) = self.on_submit {
                    cb(self.buffer.clone());
                }
                KeyHandleResult::Handled
            }
            Key::Backspace => {
                if self.cursor > 0 {
                    // Remove the char before cursor
                    let idx = self.buffer.char_indices()
                        .nth(self.cursor - 1)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.buffer.remove(idx);
                    self.cursor -= 1;
                    if let Some(ref cb) = self.on_change {
                        cb(self.buffer.clone());
                    }
                }
                KeyHandleResult::Handled
            }
            Key::Char(c) if !event.modifiers.ctrl && !event.modifiers.alt => {
                self.buffer.insert(self.buffer.char_indices()
                    .nth(self.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.buffer.len()), *c);
                self.cursor += 1;
                if let Some(ref cb) = self.on_change {
                    cb(self.buffer.clone());
                }
                KeyHandleResult::Handled
            }
            Key::ArrowLeft => {
                if self.cursor > 0 { self.cursor -= 1; }
                KeyHandleResult::Handled
            }
            Key::ArrowRight => {
                if self.cursor < self.buffer.chars().count() { self.cursor += 1; }
                KeyHandleResult::Handled
            }
            Key::Home => {
                self.cursor = 0;
                KeyHandleResult::Handled
            }
            Key::End => {
                self.cursor = self.buffer.chars().count();
                KeyHandleResult::Handled
            }
            Key::Delete => {
                let len = self.buffer.chars().count();
                if self.cursor < len {
                    let idx = self.buffer.char_indices()
                        .nth(self.cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.buffer.len());
                    self.buffer.remove(idx);
                    if let Some(ref cb) = self.on_change {
                        cb(self.buffer.clone());
                    }
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Bubble,
        }
    }
}

impl Widget for ScrollViewWidget {
    fn id(&self) -> WidgetId { self.id }
    fn layout_props(&self) -> &LayoutProps { &self.props }
    fn children(&self) -> &[WidgetNode] { std::slice::from_ref(&*self.child) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Key, KeyEvent};

    fn make_input() -> InputWidget {
        InputWidget {
            id: WidgetId(1),
            props: LayoutProps::default(),
            buffer: String::new(),
            cursor: 0,
            placeholder: "placeholder".to_string(),
            password: false,
            on_change: None,
            on_submit: None,
        }
    }

    #[test]
    fn input_char_appends_to_buffer() {
        let mut w = make_input();
        let result = w.on_key(&KeyEvent::char('a'));
        assert_eq!(result, KeyHandleResult::Handled);
        assert_eq!(w.buffer, "a");
        assert_eq!(w.cursor, 1);
    }

    #[test]
    fn input_backspace_removes_char() {
        let mut w = make_input();
        w.buffer = "abc".to_string();
        w.cursor = 3;
        let result = w.on_key(&KeyEvent { key: Key::Backspace, modifiers: Default::default() });
        assert_eq!(result, KeyHandleResult::Handled);
        assert_eq!(w.buffer, "ab");
        assert_eq!(w.cursor, 2);
    }

    #[test]
    fn input_enter_calls_on_submit() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let submitted = Rc::new(RefCell::new(String::new()));
        let sub_clone = submitted.clone();
        let mut w = make_input();
        w.buffer = "hello".to_string();
        w.on_submit = Some(Box::new(move |s| *sub_clone.borrow_mut() = s));

        let result = w.on_key(&KeyEvent { key: Key::Enter, modifiers: Default::default() });
        assert_eq!(result, KeyHandleResult::Handled);
        assert_eq!(*submitted.borrow(), "hello");
    }

    #[test]
    fn input_on_change_fires_on_keystroke() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let changes = Rc::new(RefCell::new(Vec::new()));
        let ch_clone = changes.clone();
        let mut w = make_input();
        w.on_change = Some(Box::new(move |s| ch_clone.borrow_mut().push(s)));

        w.on_key(&KeyEvent::char('a'));
        w.on_key(&KeyEvent::char('b'));
        assert_eq!(*changes.borrow(), vec!["a".to_string(), "ab".to_string()]);
    }

    #[test]
    fn input_arrow_left_moves_cursor() {
        let mut w = make_input();
        w.buffer = "abc".to_string();
        w.cursor = 3;
        w.on_key(&KeyEvent { key: Key::ArrowLeft, modifiers: Default::default() });
        assert_eq!(w.cursor, 2);
    }

    #[test]
    fn input_home_moves_to_start() {
        let mut w = make_input();
        w.buffer = "abc".to_string();
        w.cursor = 3;
        w.on_key(&KeyEvent { key: Key::Home, modifiers: Default::default() });
        assert_eq!(w.cursor, 0);
    }

    #[test]
    fn input_delete_removes_char_at_cursor() {
        let mut w = make_input();
        w.buffer = "abc".to_string();
        w.cursor = 1; // before 'b'
        w.on_key(&KeyEvent { key: Key::Delete, modifiers: Default::default() });
        assert_eq!(w.buffer, "ac");
    }

    #[test]
    fn input_escape_bubbles() {
        let mut w = make_input();
        let result = w.on_key(&KeyEvent { key: Key::Escape, modifiers: Default::default() });
        assert_eq!(result, KeyHandleResult::Bubble);
    }
}
