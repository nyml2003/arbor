// TextWidget — styled text display with word wrapping and truncation.

use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_domain::component::PropsRevisionBuilder;
use arbor_tui_domain::identity::DirtyKind;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::signal::{ReadSignal, SignalDep};
use arbor_tui_domain::text::{self, TruncateStrategy, WrapStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetId};
use arbor_tui_domain::PropsRevision;

pub struct TextWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub text: ReadSignal<String>,
    pub style: ReadSignal<TextStyle>,
    pub wrap: WrapStrategy,
    pub truncate: TruncateStrategy,
}

#[derive(Clone, PartialEq)]
pub struct TextStyle {
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}

impl Default for TextStyle {
    /// Matches dark theme: soft white on black.
    fn default() -> Self {
        Self {
            fg: AnsiColor {
                palette: arbor_tui_domain::cell::PaletteColor(252),
                true_color: None,
            },
            bg: AnsiColor {
                palette: arbor_tui_domain::cell::PaletteColor(0),
                true_color: None,
            },
            attrs: Attrs::default(),
        }
    }
}

impl Widget for TextWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn props_revision(&self) -> PropsRevision {
        let mut revision = PropsRevisionBuilder::new();
        write_layout_revision(&mut revision, &self.props);
        write_wrap_strategy(&mut revision, self.wrap);
        write_truncate_strategy(&mut revision, self.truncate);
        revision.finish()
    }

    fn signal_deps(&self) -> Vec<SignalDep> {
        vec![
            self.text.dep(DirtyKind::Layout),
            self.style.dep(DirtyKind::Render),
        ]
    }

    fn on_mount(&mut self) {
        self.text
            .subscribe_with_dirty_kind(self.id, DirtyKind::Layout);
        self.style
            .subscribe_with_dirty_kind(self.id, DirtyKind::Render);
    }

    fn on_unmount(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        let text_content = self.text.get();
        let expanded = text::expand_tabs(&text_content);
        let text_w = text::measure_width(&expanded);

        match self.wrap {
            WrapStrategy::None => {
                let w = self.props.width.unwrap_or(text_w).max(1);
                let h = (expanded.lines().count() as u16).max(1);
                SizeConstraint::fixed(w, h)
            }
            _ => {
                // Wrapping width: use available if not fixed-width
                let max_w = self.props.width.unwrap_or(text_w.max(1));
                let lines = text::wrap_lines(&expanded, max_w.max(1), self.wrap);
                SizeConstraint {
                    min_w: 1,
                    min_h: 1,
                    max_w: arbor_tui_domain::layout::AxisConstraint::Fixed(max_w.max(1)),
                    max_h: arbor_tui_domain::layout::AxisConstraint::Fixed(
                        (lines.len() as u16).max(1),
                    ),
                }
            }
        }
    }

    fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(1), rect.h.max(1));
        let style = self.style.get();

        // 先用组件背景色填充整个区域，避免 Cell::default() (黑底) 在
        // blit 时覆盖父组件的背景。
        let fill = Cell {
            bg: style.bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w.max(1), rect.h.max(1)), &fill);

        let text_content = self.text.get();
        let expanded = text::expand_tabs(&text_content);

        match self.wrap {
            WrapStrategy::None => {
                for (i, line) in expanded.lines().enumerate() {
                    if i as u16 >= rect.h {
                        break;
                    }
                    let display = text::truncate(line, rect.w, self.truncate);
                    screen.write_str(0, i as u16, &display, style.fg, style.bg, style.attrs);
                }
            }
            _ => {
                let lines = text::wrap_lines(&expanded, rect.w, self.wrap);
                for (i, line) in lines.iter().enumerate() {
                    if i as u16 >= rect.h {
                        break;
                    }
                    let display = text::truncate(line, rect.w, self.truncate);
                    screen.write_str(0, i as u16, &display, style.fg, style.bg, style.attrs);
                }
            }
        }
        screen
    }
}

impl Drop for TextWidget {
    fn drop(&mut self) {
        self.text.unsubscribe(self.id);
        self.style.unsubscribe(self.id);
    }
}

fn write_layout_revision(builder: &mut PropsRevisionBuilder, props: &LayoutProps) {
    builder
        .field_tag(1)
        .write_f32(props.flex)
        .field_tag(2)
        .write_option_u16(props.width)
        .field_tag(3)
        .write_option_u16(props.height)
        .field_tag(4)
        .write_u16(props.padding.top)
        .write_u16(props.padding.right)
        .write_u16(props.padding.bottom)
        .write_u16(props.padding.left);
}

fn write_wrap_strategy(builder: &mut PropsRevisionBuilder, wrap: WrapStrategy) {
    let value = match wrap {
        WrapStrategy::None => 0,
        WrapStrategy::Word => 1,
        WrapStrategy::Char => 2,
    };
    builder.field_tag(5).write_u8(value);
}

fn write_truncate_strategy(builder: &mut PropsRevisionBuilder, truncate: TruncateStrategy) {
    let value = match truncate {
        TruncateStrategy::End => 0,
        TruncateStrategy::Middle => 1,
        TruncateStrategy::None => 2,
    };
    builder.field_tag(6).write_u8(value);
}
