// ButtonWidget — clickable button with style variants.

use arbor_tui_domain::cell::{Attrs, Cell};
use arbor_tui_domain::component::PropsRevisionBuilder;
use arbor_tui_domain::identity::DirtyKind;
use arbor_tui_domain::input::KeyHandleResult;
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::signal::{ReadSignal, SignalDep};
use arbor_tui_domain::text::{self, TruncateStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetAction, WidgetId};
use arbor_tui_domain::PropsRevision;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Danger,
    Default,
}

pub struct ButtonWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub label: ReadSignal<String>,
    pub style: ButtonStyle,
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Widget for ButtonWidget {
    fn id(&self) -> WidgetId {
        self.id
    }
    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }
    fn focusable(&self) -> bool {
        true
    }

    fn props_revision(&self) -> PropsRevision {
        let mut revision = PropsRevisionBuilder::new();
        revision
            .field_tag(1)
            .write_u8(match self.style {
                ButtonStyle::Primary => 1,
                ButtonStyle::Secondary => 2,
                ButtonStyle::Danger => 3,
                ButtonStyle::Default => 4,
            })
            .field_tag(2)
            .write_option_u16(self.props.width)
            .field_tag(3)
            .write_option_u16(self.props.height)
            .field_tag(4)
            .write_u16(self.props.padding.top)
            .write_u16(self.props.padding.right)
            .write_u16(self.props.padding.bottom)
            .write_u16(self.props.padding.left)
            .finish()
    }

    fn signal_deps(&self) -> Vec<SignalDep> {
        vec![self.label.dep(DirtyKind::Layout)]
    }

    fn on_mount(&mut self) {
        self.label
            .subscribe_with_dirty_kind(self.id, DirtyKind::Layout);
    }
    fn on_unmount(&mut self) {
        self.label.unsubscribe(self.id);
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        if let (Some(w_val), Some(h_val)) = (self.props.width, self.props.height) {
            SizeConstraint::fixed(w_val, h_val)
        } else {
            let label = self.label.get();
            let label_w = text::measure_width(&label) + 4; // 2 padding each side
            let bw = self.props.width.unwrap_or(label_w).max(1);
            let bh = self.props.height.unwrap_or(1);
            SizeConstraint::fixed(bw, bh)
        }
    }

    fn render(&self, rect: Rect, theme: &Theme) -> VirtualScreen {
        let mut screen = VirtualScreen::new(rect.w.max(3), 1);
        let (fg, bg) = match self.style {
            ButtonStyle::Primary => (theme.surface(), theme.primary()),
            ButtonStyle::Danger => (theme.surface(), theme.danger()),
            ButtonStyle::Secondary => (theme.text(), theme.surface_alt()),
            ButtonStyle::Default => (theme.text(), theme.surface_alt()),
        };
        let attrs = Attrs {
            bold: true,
            ..Default::default()
        };

        let display = format!(" {} ", self.label.get());
        let truncated = text::truncate(&display, rect.w, TruncateStrategy::End);
        let label_w = text::measure_width(&truncated);
        let offset = rect.w.saturating_sub(label_w) / 2;

        let bg_cell = Cell {
            bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, rect.w, 1), &bg_cell);
        screen.write_str(offset, 0, &truncated, fg, bg, attrs);
        screen
    }

    fn perform(&mut self, action: &WidgetAction) -> KeyHandleResult {
        match action {
            WidgetAction::Activate => {
                if let Some(ref cb) = self.on_click {
                    cb();
                }
                KeyHandleResult::Handled
            }
            _ => KeyHandleResult::Bubble,
        }
    }
}

impl Drop for ButtonWidget {
    fn drop(&mut self) {
        self.label.unsubscribe(self.id);
    }
}
