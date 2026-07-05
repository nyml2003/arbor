use std::iter::Peekable;

use arbor_tui_adapters::crossterm_backend::encode_regions_for_testing;
use arbor_tui_application::app::{App, RenderResult};
use arbor_tui_application::runtime::{runtime_step, RuntimeInput, RuntimeStepResult};
use arbor_tui_domain::backend::{BackendError, BackendResult, TerminalBackend, TerminalGuard};
use arbor_tui_domain::cell::{AnsiColor, Attrs, Cell, PaletteColor};
use arbor_tui_domain::diff::DirtyRegion;
use arbor_tui_domain::focus::mount_tree;
use arbor_tui_domain::input::{Key, KeyEvent, KeyEventKind, Modifiers};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::text::measure_width;
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{WidgetId, WidgetNode};

pub struct AnsiTerminal {
    screen: VirtualScreen,
    cursor_col: u16,
    cursor_row: u16,
    fg: AnsiColor,
    bg: AnsiColor,
    attrs: Attrs,
}

impl AnsiTerminal {
    pub fn new(cols: u16, rows: u16) -> Self {
        let defaults = Cell::default();
        Self {
            screen: VirtualScreen::new(cols, rows),
            cursor_col: 0,
            cursor_row: 0,
            fg: defaults.fg,
            bg: defaults.bg,
            attrs: defaults.attrs,
        }
    }

    pub fn screen(&self) -> &VirtualScreen {
        &self.screen
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.screen = VirtualScreen::new(cols, rows);
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
    }

    pub fn replay(&mut self, bytes: &[u8]) -> Result<(), String> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| format!("ANSI output was not valid UTF-8: {e}"))?;
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '\x1b' => {
                    if matches!(chars.peek(), Some(&'[')) {
                        chars.next();
                        self.consume_csi(&mut chars)?;
                    }
                }
                '\r' => {
                    self.cursor_col = 0;
                }
                '\n' => {
                    self.cursor_row = self.cursor_row.saturating_add(1);
                    self.cursor_col = 0;
                }
                '\u{8}' => {
                    self.cursor_col = self.cursor_col.saturating_sub(1);
                }
                _ if !ch.is_control() => self.put_char(ch),
                _ => {}
            }
        }

        Ok(())
    }

    fn consume_csi<I>(&mut self, chars: &mut Peekable<I>) -> Result<(), String>
    where
        I: Iterator<Item = char>,
    {
        let mut body = String::new();

        for ch in chars.by_ref() {
            if is_csi_final(ch) {
                self.handle_csi(&body, ch);
                return Ok(());
            }
            body.push(ch);
        }

        Err("unterminated CSI sequence".to_string())
    }

    fn handle_csi(&mut self, body: &str, final_ch: char) {
        if body.starts_with('?') {
            return;
        }

        match final_ch {
            'H' | 'f' => self.move_cursor(body),
            'J' => self.clear_display(body),
            'K' => self.clear_line(body),
            'm' => self.apply_sgr(body),
            _ => {}
        }
    }

    fn move_cursor(&mut self, body: &str) {
        let params = parse_params(body);
        let row = params.first().copied().filter(|v| *v > 0).unwrap_or(1);
        let col = params.get(1).copied().filter(|v| *v > 0).unwrap_or(1);
        self.cursor_row = row
            .saturating_sub(1)
            .min(self.screen.rows().saturating_sub(1));
        self.cursor_col = col
            .saturating_sub(1)
            .min(self.screen.cols().saturating_sub(1));
    }

    fn clear_display(&mut self, body: &str) {
        let mode = parse_params(body).first().copied().unwrap_or(0);
        match mode {
            2 | 3 => self.fill_rect(0, 0, self.screen.cols(), self.screen.rows()),
            0 => self.fill_rect(
                self.cursor_col,
                self.cursor_row,
                self.screen.cols().saturating_sub(self.cursor_col),
                self.screen.rows().saturating_sub(self.cursor_row),
            ),
            _ => {}
        }
    }

    fn clear_line(&mut self, body: &str) {
        let mode = parse_params(body).first().copied().unwrap_or(0);
        match mode {
            0 => self.fill_rect(
                self.cursor_col,
                self.cursor_row,
                self.screen.cols().saturating_sub(self.cursor_col),
                1,
            ),
            1 => self.fill_rect(0, self.cursor_row, self.cursor_col.saturating_add(1), 1),
            2 => self.fill_rect(0, self.cursor_row, self.screen.cols(), 1),
            _ => {}
        }
    }

    fn apply_sgr(&mut self, body: &str) {
        let params = parse_params(body);
        let mut index = 0;

        while index < params.len() {
            match params[index] {
                0 => self.reset_style(),
                1 => self.attrs.bold = true,
                2 => self.attrs.dim = true,
                3 => self.attrs.italic = true,
                4 => self.attrs.underline = true,
                7 => self.attrs.reverse = true,
                22 => {
                    self.attrs.bold = false;
                    self.attrs.dim = false;
                }
                23 => self.attrs.italic = false,
                24 => self.attrs.underline = false,
                27 => self.attrs.reverse = false,
                30..=37 => self.fg = AnsiColor::from_palette((params[index] - 30) as u8),
                40..=47 => self.bg = AnsiColor::from_palette((params[index] - 40) as u8),
                90..=97 => self.fg = AnsiColor::from_palette((params[index] - 90 + 8) as u8),
                100..=107 => self.bg = AnsiColor::from_palette((params[index] - 100 + 8) as u8),
                38 => {
                    index += self.apply_extended_color(&params[index..], true);
                }
                48 => {
                    index += self.apply_extended_color(&params[index..], false);
                }
                39 => self.fg = Cell::default().fg,
                49 => self.bg = Cell::default().bg,
                _ => {}
            }
            index += 1;
        }
    }

    fn apply_extended_color(&mut self, params: &[u16], foreground: bool) -> usize {
        if params.len() >= 3 && params[1] == 5 {
            let color = AnsiColor::from_palette(params[2].min(255) as u8);
            if foreground {
                self.fg = color;
            } else {
                self.bg = color;
            }
            return 2;
        }

        if params.len() >= 5 && params[1] == 2 {
            let color = AnsiColor::from_rgb(params[2] as u8, params[3] as u8, params[4] as u8);
            if foreground {
                self.fg = color;
            } else {
                self.bg = color;
            }
            return 4;
        }

        0
    }

    fn reset_style(&mut self) {
        let defaults = Cell::default();
        self.fg = defaults.fg;
        self.bg = defaults.bg;
        self.attrs = defaults.attrs;
    }

    fn put_char(&mut self, ch: char) {
        if self.cursor_row >= self.screen.rows() {
            return;
        }
        if self.cursor_col >= self.screen.cols() {
            self.cursor_col = 0;
            self.cursor_row = self.cursor_row.saturating_add(1);
            if self.cursor_row >= self.screen.rows() {
                return;
            }
        }

        let width = measure_width(&ch.to_string()).max(1);
        if self.cursor_col.saturating_add(width) > self.screen.cols() {
            return;
        }

        if let Some(cell) = self.screen.cell_at_mut(self.cursor_col, self.cursor_row) {
            *cell = Cell {
                ch,
                fg: self.fg,
                bg: self.bg,
                attrs: self.attrs,
                phantom: false,
            };
        }

        let phantom = Cell {
            phantom: true,
            ..self.blank_cell()
        };
        for offset in 1..width {
            if let Some(cell) = self
                .screen
                .cell_at_mut(self.cursor_col + offset, self.cursor_row)
            {
                *cell = phantom.clone();
            }
        }

        self.cursor_col = self.cursor_col.saturating_add(width);
    }

    fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16) {
        let cell = self.blank_cell();
        for row in y..y.saturating_add(h).min(self.screen.rows()) {
            for col in x..x.saturating_add(w).min(self.screen.cols()) {
                if let Some(target) = self.screen.cell_at_mut(col, row) {
                    *target = cell.clone();
                }
            }
        }
    }

    fn blank_cell(&self) -> Cell {
        Cell {
            ch: ' ',
            fg: self.fg,
            bg: self.bg,
            attrs: self.attrs,
            phantom: false,
        }
    }
}

pub struct AnsiReplayBackend {
    terminal: AnsiTerminal,
    output: Vec<u8>,
    no_color: bool,
}

impl AnsiReplayBackend {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            terminal: AnsiTerminal::new(cols, rows),
            output: Vec::new(),
            no_color: false,
        }
    }

    pub fn with_no_color(cols: u16, rows: u16) -> Self {
        Self {
            no_color: true,
            ..Self::new(cols, rows)
        }
    }

    pub fn screen(&self) -> &VirtualScreen {
        self.terminal.screen()
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.terminal.resize(cols, rows);
    }

    pub fn output(&self) -> &[u8] {
        &self.output
    }

    pub fn output_len(&self) -> usize {
        self.output.len()
    }

    pub fn output_contains(&self, needle: &str) -> bool {
        String::from_utf8_lossy(&self.output).contains(needle)
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }
}

struct AnsiReplayGuard;

impl TerminalGuard for AnsiReplayGuard {
    fn restore(&mut self) {}
}

impl TerminalBackend for AnsiReplayBackend {
    fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>> {
        Ok(Box::new(AnsiReplayGuard))
    }

    fn size(&self) -> BackendResult<(u16, u16)> {
        Ok((self.screen().cols(), self.screen().rows()))
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()> {
        let bytes = encode_regions_for_testing(regions, screen, self.no_color)?;
        self.terminal.replay(&bytes).map_err(BackendError::new)?;
        self.output.extend(bytes);
        Ok(())
    }

    fn hide_cursor(&mut self) -> BackendResult<()> {
        self.output.extend(b"\x1b[?25l");
        Ok(())
    }

    fn show_cursor(&mut self) -> BackendResult<()> {
        self.output.extend(b"\x1b[?25h");
        Ok(())
    }

    fn enter_alternate_screen(&mut self) -> BackendResult<()> {
        self.output.extend(b"\x1b[?1049h");
        Ok(())
    }

    fn exit_alternate_screen(&mut self) -> BackendResult<()> {
        self.output.extend(b"\x1b[?1049l");
        Ok(())
    }

    fn clear(&mut self) -> BackendResult<()> {
        let bytes = b"\x1b[2J";
        self.terminal.replay(bytes).map_err(BackendError::new)?;
        self.output.extend(bytes);
        Ok(())
    }

    fn flush(&mut self) -> BackendResult<()> {
        Ok(())
    }
}

pub struct AnsiTuiTestDriver {
    app: App,
    root: WidgetNode,
    backend: AnsiReplayBackend,
    theme: Theme,
    mounted: bool,
    first_frame: bool,
    last_step: RuntimeStepResult,
    last_render: Option<RenderResult>,
}

impl AnsiTuiTestDriver {
    pub fn new(root: WidgetNode, cols: u16, rows: u16, theme: Theme) -> Self {
        Self {
            app: App::new(cols, rows),
            root,
            backend: AnsiReplayBackend::new(cols, rows),
            theme,
            mounted: false,
            first_frame: true,
            last_step: RuntimeStepResult::default(),
            last_render: None,
        }
    }

    fn mount(&mut self) {
        if !self.mounted {
            self.app.run();
            mount_tree(&mut self.root);
            self.mounted = true;
        }
    }

    pub fn tick(
        &mut self,
        events: impl IntoIterator<Item = KeyEvent>,
    ) -> anyhow::Result<RuntimeStepResult> {
        self.mount();
        let events = events.into_iter().collect();
        let input = if self.first_frame {
            RuntimeInput::first_frame_with_events(events)
        } else {
            RuntimeInput::new(events)
        };
        let step = runtime_step(&mut self.app, &mut self.root, &self.backend, input)?;
        if step.should_clear {
            self.backend.clear()?;
        }
        self.last_render = if step.should_render {
            Some(
                self.app
                    .render_widget_tree(&self.root, &self.theme, &mut self.backend)?,
            )
        } else {
            None
        };
        self.first_frame = false;
        self.last_step = step;
        Ok(step)
    }

    pub fn render_initial(&mut self) -> anyhow::Result<()> {
        self.tick([])?;
        Ok(())
    }

    pub fn focus_next(&mut self) -> anyhow::Result<()> {
        self.tick([KeyEvent {
            key: Key::Tab,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }])?;
        Ok(())
    }

    pub fn send_chars(&mut self, text: &str) -> anyhow::Result<()> {
        self.tick(text.chars().map(KeyEvent::char))?;
        Ok(())
    }

    pub fn send_key(&mut self, key: Key) -> anyhow::Result<()> {
        self.send_modified_key(key, Modifiers::default())
    }

    pub fn send_modified_key(&mut self, key: Key, modifiers: Modifiers) -> anyhow::Result<()> {
        self.tick([KeyEvent {
            key,
            modifiers,
            kind: KeyEventKind::Press,
        }])?;
        Ok(())
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> {
        self.backend.resize(cols, rows);
        let step = runtime_step(
            &mut self.app,
            &mut self.root,
            &self.backend,
            RuntimeInput::resize(cols, rows),
        )?;
        if step.should_clear {
            self.backend.clear()?;
        }
        self.last_render = if step.should_render {
            Some(
                self.app
                    .render_widget_tree(&self.root, &self.theme, &mut self.backend)?,
            )
        } else {
            None
        };
        self.first_frame = false;
        self.last_step = step;
        Ok(())
    }

    pub fn screen(&self) -> &VirtualScreen {
        self.backend.screen()
    }

    pub fn cell_at(&self, col: u16, row: u16) -> Cell {
        self.screen().cell_at(col, row)
    }

    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.app.focused_widget()
    }

    pub fn is_running(&self) -> bool {
        self.app.is_running()
    }

    pub fn last_step(&self) -> RuntimeStepResult {
        self.last_step
    }

    pub fn last_render(&self) -> Option<RenderResult> {
        self.last_render
    }

    pub fn output(&self) -> &[u8] {
        self.backend.output()
    }

    pub fn output_len(&self) -> usize {
        self.backend.output_len()
    }

    pub fn output_contains(&self, needle: &str) -> bool {
        self.backend.output_contains(needle)
    }

    pub fn clear_output(&mut self) {
        self.backend.clear_output();
    }

    pub fn find_text(&self, needle: &str) -> Vec<(u16, u16)> {
        crate::e2e::find_text_in_screen(self.screen(), needle)
    }

    pub fn row_text(&self, row: u16) -> String {
        (0..self.screen().cols())
            .map(|col| self.screen().cell_at(col, row).ch)
            .collect()
    }

    pub fn visible_text(&self) -> String {
        let mut text = String::new();
        for row in 0..self.screen().rows() {
            text.push_str(&self.row_text(row));
            if row + 1 < self.screen().rows() {
                text.push('\n');
            }
        }
        text
    }

    pub fn assert_no_default_black_on_visible_text(&self) -> Result<(), Vec<(u16, u16, char)>> {
        let black = PaletteColor(0);
        let mut offenders = Vec::new();
        for (col, row, cell) in self.screen().iter_cells() {
            if cell.ch != ' ' && cell.bg.palette == black {
                offenders.push((col, row, cell.ch));
            }
        }
        if offenders.is_empty() {
            Ok(())
        } else {
            Err(offenders)
        }
    }

    pub fn count_bg(&self, bg: AnsiColor) -> usize {
        self.screen()
            .iter_cells()
            .filter(|(_, _, cell)| cell.bg == bg)
            .count()
    }
}

fn parse_params(body: &str) -> Vec<u16> {
    if body.is_empty() {
        return vec![0];
    }

    body.split(';')
        .map(|part| part.parse::<u16>().unwrap_or(0))
        .collect()
}

fn is_csi_final(ch: char) -> bool {
    ('\u{40}'..='\u{7e}').contains(&ch)
}
