use std::io::{self, Stdout, Write};

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue,
    style::{Color as TermColor, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use thorn_core::{
    render::{DirtyRegion, Screen},
    theme::Color,
};

pub type Result<T> = std::result::Result<T, TerminalError>;

#[derive(Debug)]
pub enum TerminalError {
    BackendUnavailable,
    Io(io::Error),
}

impl From<io::Error> for TerminalError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub trait TerminalBackend {
    fn size(&self) -> Result<(u16, u16)>;
    fn enter(&mut self) -> Result<TerminalGuard>;
    fn emit(&mut self, regions: &[DirtyRegion], screen: &Screen) -> Result<()>;
    fn flush(&mut self) -> Result<()>;
}

pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    fn inactive() -> Self {
        Self { active: false }
    }

    fn active() -> Self {
        Self { active: true }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if !self.active {
            return;
        }

        let mut stdout = io::stdout();
        let _ = execute!(
            stdout,
            ResetColor,
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}

pub struct CrosstermBackend {
    stdout: Stdout,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TerminalEvent {
    Enter,
    Quit,
    Resize,
    Other,
}

impl CrosstermBackend {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    pub fn read_event(&mut self) -> Result<TerminalEvent> {
        loop {
            match event::read()? {
                Event::Key(key) => {
                    return Ok(match key.code {
                        KeyCode::Enter => TerminalEvent::Enter,
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            TerminalEvent::Quit
                        }
                        _ => TerminalEvent::Other,
                    });
                }
                Event::Resize(_, _) => return Ok(TerminalEvent::Resize),
                _ => return Ok(TerminalEvent::Other),
            }
        }
    }
}

impl Default for CrosstermBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalBackend for CrosstermBackend {
    fn size(&self) -> Result<(u16, u16)> {
        Ok(terminal::size()?)
    }

    fn enter(&mut self) -> Result<TerminalGuard> {
        enable_raw_mode()?;
        execute!(self.stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
        Ok(TerminalGuard::active())
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &Screen) -> Result<()> {
        for region in regions {
            let y_end = region
                .rect
                .y
                .saturating_add(region.rect.h)
                .min(screen.height());
            let x_end = region
                .rect
                .x
                .saturating_add(region.rect.w)
                .min(screen.width());
            for y in region.rect.y..y_end {
                for x in region.rect.x..x_end {
                    let cell = screen.get(x, y);
                    if cell.wide_continuation {
                        continue;
                    }
                    queue!(
                        self.stdout,
                        MoveTo(x, y),
                        SetForegroundColor(to_term_color(cell.fg)),
                        SetBackgroundColor(to_term_color(cell.bg)),
                        Print(cell.ch)
                    )?;
                }
            }
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.stdout.flush()?;
        Ok(())
    }
}

#[derive(Default)]
pub struct MemoryBackend {
    screen: Option<Screen>,
    emitted_regions: Vec<DirtyRegion>,
    flushes: usize,
}

impl MemoryBackend {
    pub fn screen(&self) -> Option<&Screen> {
        self.screen.as_ref()
    }

    pub fn emitted_regions(&self) -> &[DirtyRegion] {
        &self.emitted_regions
    }

    pub fn flushes(&self) -> usize {
        self.flushes
    }
}

impl TerminalBackend for MemoryBackend {
    fn size(&self) -> Result<(u16, u16)> {
        Ok(self
            .screen
            .as_ref()
            .map(|screen| (screen.width(), screen.height()))
            .unwrap_or((0, 0)))
    }

    fn enter(&mut self) -> Result<TerminalGuard> {
        Ok(TerminalGuard::inactive())
    }

    fn emit(&mut self, regions: &[DirtyRegion], screen: &Screen) -> Result<()> {
        self.emitted_regions.extend_from_slice(regions);
        self.screen = Some(screen.clone());
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.flushes += 1;
        Ok(())
    }
}

fn to_term_color(color: Color) -> TermColor {
    match color {
        Color::Palette(value) => TermColor::AnsiValue(value),
        Color::Rgb {
            r,
            g,
            b,
            fallback: _,
        } => TermColor::Rgb { r, g, b },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_core::layout::Rect;

    #[test]
    fn memory_backend_records_emit_and_flush() {
        let mut backend = MemoryBackend::default();
        let screen = Screen::new(2, 1);
        let regions = vec![DirtyRegion {
            rect: Rect::new(0, 0, 2, 1),
        }];

        let _guard = backend.enter().unwrap();
        backend.emit(&regions, &screen).unwrap();
        backend.flush().unwrap();

        assert_eq!(backend.screen(), Some(&screen));
        assert_eq!(backend.emitted_regions(), regions);
        assert_eq!(backend.flushes(), 1);
    }

    #[test]
    fn palette_colors_map_to_ansi_values() {
        assert_eq!(to_term_color(Color::Palette(42)), TermColor::AnsiValue(42));
    }
}
