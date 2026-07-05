use arbor_tui_backend::simulated_backend::SimulatedBackend;
use arbor_tui_primitives::cell::{AnsiColor, Cell, PaletteColor};
use arbor_tui_primitives::input::{Key, KeyEvent, KeyEventKind, Modifiers};
use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::focus::mount_tree;
use arbor_tui_widget::widget::{WidgetId, WidgetNode};

use crate::app::{App, RenderResult};
use crate::runtime::{runtime_step, RuntimeInput, RuntimeStepResult};

pub struct TuiTestDriver {
    app: App,
    root: WidgetNode,
    backend: SimulatedBackend,
    theme: Theme,
    mounted: bool,
    first_frame: bool,
    last_step: RuntimeStepResult,
    last_render: Option<RenderResult>,
}

impl TuiTestDriver {
    pub fn new(root: WidgetNode, cols: u16, rows: u16, theme: Theme) -> Self {
        Self {
            app: App::new(cols, rows),
            root,
            backend: SimulatedBackend::new(cols, rows),
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
        self.tick([KeyEvent {
            key,
            modifiers: Modifiers::default(),
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
        find_text_in_screen(self.screen(), needle)
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

pub fn find_text_in_screen(screen: &VirtualScreen, needle: &str) -> Vec<(u16, u16)> {
    let mut positions = Vec::new();
    let needle_chars: Vec<char> = needle.chars().collect();
    if needle_chars.is_empty() {
        return positions;
    }

    for row in 0..screen.rows() {
        let mut col = 0u16;
        while col < screen.cols() {
            let mut matched = true;
            for (i, ch) in needle_chars.iter().enumerate() {
                let c = col + i as u16;
                if c >= screen.cols() || screen.cell_at(c, row).ch != *ch {
                    matched = false;
                    break;
                }
            }
            if matched {
                positions.push((col, row));
                col += needle_chars.len() as u16;
            } else {
                col += 1;
            }
        }
    }

    positions
}
