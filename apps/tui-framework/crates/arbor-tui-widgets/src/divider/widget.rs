use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetId};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct DividerStyle {
    pub left: char,
    pub fill: char,
    pub right: char,
    pub fg: AnsiColor,
    pub bg: AnsiColor,
    pub attrs: Attrs,
}

pub struct DividerWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub style: DividerStyle,
}

impl Widget for DividerWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        SizeConstraint::fixed(self.props.width.unwrap_or(3).max(1), 1)
    }

    fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
        let width = rect.w.max(1);
        let mut screen = VirtualScreen::new(width, 1);
        let fill = Cell {
            bg: self.style.bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, width, 1), &fill);

        for col in 0..width {
            let ch = match (col, width) {
                (0, _) => self.style.left,
                (_, 2) if col == 1 => self.style.right,
                (_, w) if col + 1 == w => self.style.right,
                _ => self.style.fill,
            };
            if let Some(cell) = screen.cell_at_mut(col, 0) {
                *cell = Cell {
                    ch,
                    fg: self.style.fg,
                    bg: self.style.bg,
                    attrs: self.style.attrs,
                    phantom: false,
                };
            }
        }

        screen
    }
}
