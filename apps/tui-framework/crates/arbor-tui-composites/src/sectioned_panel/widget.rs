use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell};
use arbor_tui_domain::layout::{LayoutProps, Rect, Size, SizeConstraint};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::text::{self, TruncateStrategy};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{Widget, WidgetId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionedPanelSection {
    title: Option<String>,
    lines: Vec<String>,
}

impl SectionedPanelSection {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            lines: Vec::new(),
        }
    }

    pub fn untitled() -> Self {
        Self {
            title: None,
            lines: Vec::new(),
        }
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn lines(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.lines.extend(lines.into_iter().map(Into::into));
        self
    }

    fn rendered_title(&self) -> Option<String> {
        self.title.as_ref().map(|title| format!("【{title}】"))
    }

    fn line_count(&self) -> u16 {
        let title = u16::from(self.title.is_some());
        title + self.lines.len().min(u16::MAX as usize) as u16
    }

    fn max_content_width(&self) -> u16 {
        let title_width = self
            .rendered_title()
            .map(|title| text::measure_width(&title))
            .unwrap_or(0);
        self.lines
            .iter()
            .map(|line| text::measure_width(line))
            .fold(title_width, u16::max)
    }
}

pub(crate) struct SectionedPanelStyle {
    pub border_fg: AnsiColor,
    pub bg: AnsiColor,
    pub title_fg: AnsiColor,
    pub text_fg: AnsiColor,
}

pub(crate) struct SectionedPanelWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub sections: Vec<SectionedPanelSection>,
    pub style: SectionedPanelStyle,
}

impl Widget for SectionedPanelWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn measure(&self, _available: Size) -> SizeConstraint {
        let content_w = self
            .sections
            .iter()
            .map(SectionedPanelSection::max_content_width)
            .max()
            .unwrap_or(0);
        let intrinsic_w = content_w.saturating_add(4).max(4);
        let content_h = self
            .sections
            .iter()
            .map(SectionedPanelSection::line_count)
            .fold(0u16, u16::saturating_add);
        let separator_h = self.sections.len().saturating_sub(1).min(u16::MAX as usize) as u16;
        let intrinsic_h = content_h
            .saturating_add(separator_h)
            .saturating_add(2)
            .max(2);

        SizeConstraint::fixed(
            self.props.width.unwrap_or(intrinsic_w).max(4),
            self.props.height.unwrap_or(intrinsic_h).max(2),
        )
    }

    fn render(&self, rect: Rect, _theme: &Theme) -> VirtualScreen {
        let width = rect.w.max(4);
        let height = rect.h.max(2);
        let mut screen = VirtualScreen::new(width, height);
        let fill = Cell {
            bg: self.style.bg,
            ..Default::default()
        };
        screen.fill_rect(Rect::new(0, 0, width, height), &fill);

        self.draw_top(&mut screen, 0, width);

        let mut row = 1;
        for (index, section) in self.sections.iter().enumerate() {
            if row >= height.saturating_sub(1) {
                break;
            }
            if let Some(title) = section.rendered_title() {
                self.draw_content_line(&mut screen, row, width, &title, self.style.title_fg);
                row += 1;
            }
            for line in &section.lines {
                if row >= height.saturating_sub(1) {
                    break;
                }
                self.draw_content_line(&mut screen, row, width, line, self.style.text_fg);
                row += 1;
            }
            if index + 1 < self.sections.len() && row < height.saturating_sub(1) {
                self.draw_bridge(&mut screen, row, width);
                row += 1;
            }
        }

        self.draw_bottom(&mut screen, height - 1, width);
        screen
    }
}

impl SectionedPanelWidget {
    fn border_cell(&self, ch: char) -> Cell {
        Cell {
            ch,
            fg: self.style.border_fg,
            bg: self.style.bg,
            attrs: Attrs::default(),
            phantom: false,
        }
    }

    fn set_border(&self, screen: &mut VirtualScreen, col: u16, row: u16, ch: char) {
        if let Some(cell) = screen.cell_at_mut(col, row) {
            *cell = self.border_cell(ch);
        }
    }

    fn draw_top(&self, screen: &mut VirtualScreen, row: u16, width: u16) {
        self.set_border(screen, 0, row, '╭');
        for col in 1..width.saturating_sub(1) {
            self.set_border(screen, col, row, '─');
        }
        self.set_border(screen, width - 1, row, '╮');
    }

    fn draw_bridge(&self, screen: &mut VirtualScreen, row: u16, width: u16) {
        self.set_border(screen, 0, row, '╰');
        for col in 1..width.saturating_sub(2) {
            self.set_border(screen, col, row, '─');
        }
        self.set_border(screen, width - 2, row, '╭');
        self.set_border(screen, width - 1, row, '╯');
    }

    fn draw_bottom(&self, screen: &mut VirtualScreen, row: u16, width: u16) {
        self.set_border(screen, 0, row, '╰');
        for col in 1..width.saturating_sub(1) {
            self.set_border(screen, col, row, '─');
        }
        self.set_border(screen, width - 1, row, '╯');
    }

    fn draw_content_line(
        &self,
        screen: &mut VirtualScreen,
        row: u16,
        width: u16,
        text: &str,
        fg: AnsiColor,
    ) {
        self.set_border(screen, 0, row, '│');
        self.set_border(screen, width - 1, row, '│');

        let content_w = width.saturating_sub(4);
        if content_w == 0 {
            return;
        }

        let display = text::truncate(text, content_w, TruncateStrategy::End);
        screen.write_str(2, row, &display, fg, self.style.bg, Attrs::default());
    }
}
