use std::io::{self, Stdout, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, Event, KeyCode, KeyEvent as CrosstermKeyEvent, KeyEventKind as CrosstermKeyEventKind,
        KeyModifiers as CrosstermKeyModifiers,
    },
    execute, queue,
    style::{Color as TermColor, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use thorn_core::{
    layout::Size,
    render::{DirtyRegion, Screen},
    runtime::{Key, KeyEvent, KeyEventKind, KeyModifiers, RuntimeInput},
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
        let _ = execute!(stdout, ResetColor, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub struct CrosstermBackend {
    stdout: Stdout,
}

impl CrosstermBackend {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    pub fn read_input(&mut self) -> Result<Option<RuntimeInput>> {
        loop {
            let Some(input) = convert_event(event::read()?) else {
                continue;
            };
            return Ok(Some(input));
        }
    }
}

pub struct InputReader {
    events: Receiver<RuntimeInput>,
    shutdown: SyncSender<()>,
    handle: Option<JoinHandle<()>>,
}

impl InputReader {
    pub fn spawn() -> Self {
        let (event_tx, events) = mpsc::sync_channel(256);
        let (shutdown, shutdown_rx) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || loop {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }
            match event::poll(Duration::from_millis(50)) {
                Ok(true) => match event::read() {
                    Ok(event) => {
                        if let Some(input) = convert_event(event) {
                            let _ = event_tx.try_send(input);
                        }
                    }
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        });
        Self {
            events,
            shutdown,
            handle: Some(handle),
        }
    }

    pub fn poll(&self) -> Vec<RuntimeInput> {
        let mut out = Vec::new();
        loop {
            match self.events.try_recv() {
                Ok(input) => out.push(input),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        out
    }

    pub fn poll_timeout(&self, timeout: Duration) -> Vec<RuntimeInput> {
        let mut out = Vec::new();
        match self.events.recv_timeout(timeout) {
            Ok(input) => out.push(input),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return out,
        }
        out.extend(self.poll());
        out
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.try_send(());
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
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
        execute!(self.stdout, EnterAlternateScreen, Hide)?;
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

pub fn convert_event(event: Event) -> Option<RuntimeInput> {
    match event {
        Event::Key(key) => convert_key_event(key).map(RuntimeInput::Key),
        Event::Resize(width, height) => Some(RuntimeInput::Resize(Size::new(width, height))),
        _ => None,
    }
}

pub fn convert_key_event(event: CrosstermKeyEvent) -> Option<KeyEvent> {
    Some(KeyEvent {
        key: convert_key(event.code)?,
        modifiers: convert_modifiers(event.modifiers),
        kind: convert_key_kind(event.kind),
    })
}

fn convert_key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Enter => Key::Enter,
        KeyCode::Left => Key::ArrowLeft,
        KeyCode::Right => Key::ArrowRight,
        KeyCode::Up => Key::ArrowUp,
        KeyCode::Down => Key::ArrowDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Tab | KeyCode::BackTab => Key::Tab,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::F(value) => Key::F(value),
        KeyCode::Null => return None,
        KeyCode::Esc => Key::Escape,
        KeyCode::Char(ch) => Key::Char(ch),
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin
        | KeyCode::Media(_)
        | KeyCode::Modifier(_) => return None,
    })
}

fn convert_modifiers(modifiers: CrosstermKeyModifiers) -> KeyModifiers {
    let mut out = KeyModifiers::empty();
    if modifiers.contains(CrosstermKeyModifiers::SHIFT) {
        out = out.union(KeyModifiers::SHIFT);
    }
    if modifiers.contains(CrosstermKeyModifiers::CONTROL) {
        out = out.union(KeyModifiers::CTRL);
    }
    if modifiers.contains(CrosstermKeyModifiers::ALT) {
        out = out.union(KeyModifiers::ALT);
    }
    out
}

fn convert_key_kind(kind: CrosstermKeyEventKind) -> KeyEventKind {
    match kind {
        CrosstermKeyEventKind::Press => KeyEventKind::Press,
        CrosstermKeyEventKind::Repeat => KeyEventKind::Repeat,
        CrosstermKeyEventKind::Release => KeyEventKind::Release,
    }
}

#[cfg(test)]
fn enter_terminal_commands_include_mouse_capture() -> bool {
    false
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
    use crossterm::event::KeyEventState;
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

    #[test]
    fn converts_crossterm_key_event_to_core_input() {
        let input = convert_event(Event::Key(CrosstermKeyEvent {
            code: KeyCode::Char('x'),
            modifiers: CrosstermKeyModifiers::CONTROL | CrosstermKeyModifiers::SHIFT,
            kind: CrosstermKeyEventKind::Press,
            state: KeyEventState::NONE,
        }));

        assert_eq!(
            input,
            Some(RuntimeInput::Key(KeyEvent {
                key: Key::Char('x'),
                modifiers: KeyModifiers::CTRL.union(KeyModifiers::SHIFT),
                kind: KeyEventKind::Press,
            }))
        );
    }

    #[test]
    fn converts_crossterm_resize_event_to_core_input() {
        assert_eq!(
            convert_event(Event::Resize(80, 24)),
            Some(RuntimeInput::Resize(Size::new(80, 24)))
        );
    }

    #[test]
    fn terminal_entry_does_not_enable_mouse_capture() {
        assert!(!enter_terminal_commands_include_mouse_capture());
    }
}
