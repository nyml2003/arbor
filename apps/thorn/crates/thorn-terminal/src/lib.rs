use std::io::{self, BufRead, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode as CrosstermKeyCode, KeyEvent as CrosstermKeyEvent},
    execute,
    style::{
        Attribute as CrosstermAttribute, Color as CrosstermColor, Print, ResetColor, SetAttribute,
        SetBackgroundColor, SetForegroundColor,
    },
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
    QueueableCommand,
};
use thorn_core::{
    BackendCapabilities, BackendError, BackendEventSource, BackendFeature, BackendInputEvent,
    BackendKey, BackendKeyEvent, BackendPresenter, BoundedInputQueue, Cell, CellAttrs, Color,
    Direction, InputThreadDriver, IntentMapper, KeyMap, LayeredKeyMap, PresentedFrame,
    RuntimeInput, ScreenPatch, Size, ThornApp, WideCell,
};
use thorn_runtime::AppRuntime;

pub struct TerminalRuntime<App>
where
    App: ThornApp,
{
    runtime: AppRuntime<App>,
    has_presented_frame: bool,
}

pub struct CrosstermTerminalRuntime<App>
where
    App: ThornApp,
{
    runtime: AppRuntime<App>,
    presenter: CrosstermPresenter<io::Stdout>,
}

pub struct CrosstermPresenter<Output> {
    output: Output,
    capabilities: BackendCapabilities,
    presented_frames: usize,
    has_presented_frame: bool,
}

pub struct CrosstermSession;

impl<App> TerminalRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        Self {
            runtime: AppRuntime::new(app, mapper),
            has_presented_frame: false,
        }
    }

    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.runtime = self.runtime.size(width, height);
        self
    }

    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.keymap(keymap);
        self
    }

    pub fn layered_keymap(mut self, keymap: LayeredKeyMap) -> Self {
        self.runtime = self.runtime.layered_keymap(keymap);
        self
    }

    pub fn app_keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.app_keymap(keymap);
        self
    }

    pub fn mode_keymap(mut self, mode: &'static str, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.mode_keymap(mode, keymap);
        self
    }

    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let stdout = io::stdout();
        self.run_with_io(stdin.lock(), stdout.lock())
    }

    pub fn run_with_io(&mut self, input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        let mut input_driver = InputThreadDriver::new(StdioLineEventSource::new(input));
        let mut input_queue = BoundedInputQueue::new(8);

        while self.runtime.is_running() {
            self.draw(&mut output)?;
            let _ = input_driver.step(&mut input_queue);
            self.drain_runtime_input_queue(&mut input_queue);
        }

        Ok(())
    }

    pub fn send_key(&mut self, ch: char) {
        self.runtime.send_key(ch);
    }

    pub fn handle_input(&mut self, input: RuntimeInput) {
        self.runtime.handle_input(input);
    }

    pub fn render_text(&mut self) -> String {
        self.runtime.render_frame().to_plain_text()
    }

    pub fn is_running(&self) -> bool {
        self.runtime.is_running()
    }

    fn drain_runtime_input_queue(&mut self, queue: &mut BoundedInputQueue) {
        while let Some(input) = queue.pop() {
            self.runtime.handle_input(input);
        }
    }

    fn draw(&mut self, output: &mut impl Write) -> io::Result<()> {
        let mut patch = self.runtime.render_patch();
        if !self.has_presented_frame {
            patch = self.runtime.screen().full_patch();
            patch.full = true;
            self.has_presented_frame = true;
        }

        write_screen_patch_ansi(output, &patch)?;
        write_prompt(output, patch.size.height)?;
        output.flush()
    }
}

impl<App> CrosstermTerminalRuntime<App>
where
    App: ThornApp,
{
    pub fn new(app: App, mapper: impl IntentMapper<App::Action> + 'static) -> Self {
        Self {
            runtime: AppRuntime::new(app, mapper),
            presenter: CrosstermPresenter::new(io::stdout()),
        }
    }

    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.keymap(keymap);
        self
    }

    pub fn layered_keymap(mut self, keymap: LayeredKeyMap) -> Self {
        self.runtime = self.runtime.layered_keymap(keymap);
        self
    }

    pub fn app_keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.app_keymap(keymap);
        self
    }

    pub fn mode_keymap(mut self, mode: &'static str, keymap: KeyMap) -> Self {
        self.runtime = self.runtime.mode_keymap(mode, keymap);
        self
    }

    pub fn run(&mut self) -> io::Result<()> {
        let _session = CrosstermSession::enter()?;
        let size = crossterm_terminal_size()?;
        self.runtime.resize(size);

        while self.runtime.is_running() {
            self.present_runtime_frame()?;
            if let Some(input) = read_crossterm_runtime_input()? {
                self.runtime.handle_input(input);
            }
        }

        Ok(())
    }

    pub fn present_runtime_frame(&mut self) -> io::Result<PresentedFrame> {
        let patch = if self.presenter.has_presented_frame {
            self.runtime.render_patch()
        } else {
            self.runtime.render_frame();
            self.runtime.screen().full_patch()
        };

        self.presenter
            .present(&patch)
            .map_err(crossterm_backend_error_to_io)
    }

    pub fn is_running(&self) -> bool {
        self.runtime.is_running()
    }
}

impl<Output> CrosstermPresenter<Output>
where
    Output: Write,
{
    pub fn new(output: Output) -> Self {
        Self {
            output,
            capabilities: BackendCapabilities::new(vec![
                BackendFeature::Text,
                BackendFeature::FillRect,
                BackendFeature::Border,
                BackendFeature::Cursor,
                BackendFeature::Clip,
                BackendFeature::Layer,
            ]),
            presented_frames: 0,
            has_presented_frame: false,
        }
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    pub fn output_mut(&mut self) -> &mut Output {
        &mut self.output
    }

    pub fn presented_frames(&self) -> usize {
        self.presented_frames
    }
}

impl<Output> BackendPresenter for CrosstermPresenter<Output>
where
    Output: Write,
{
    fn capabilities(&self) -> &BackendCapabilities {
        &self.capabilities
    }

    fn present(&mut self, patch: &ScreenPatch) -> Result<PresentedFrame, BackendError> {
        write_screen_patch_crossterm(&mut self.output, patch).map_err(|err| {
            BackendError::PresentationFailed {
                message: err.to_string(),
            }
        })?;
        self.output
            .flush()
            .map_err(|err| BackendError::PresentationFailed {
                message: err.to_string(),
            })?;

        self.presented_frames += 1;
        self.has_presented_frame = true;
        Ok(PresentedFrame {
            size: patch.size,
            full: patch.full,
            changed_cells: patch.cells.len(),
            output_summary: format!(
                "crossterm frame={} cells={} full={}",
                self.presented_frames,
                patch.cells.len(),
                patch.full
            ),
        })
    }
}

impl CrosstermSession {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            cursor::Hide,
            Clear(ClearType::All)
        )?;
        Ok(Self)
    }
}

impl Drop for CrosstermSession {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

struct StdioLineEventSource<Input> {
    input: Input,
    emitted_shutdown: bool,
}

impl<Input> StdioLineEventSource<Input> {
    fn new(input: Input) -> Self {
        Self {
            input,
            emitted_shutdown: false,
        }
    }
}

impl<Input> BackendEventSource for StdioLineEventSource<Input>
where
    Input: BufRead,
{
    fn read_event(&mut self) -> Option<BackendInputEvent> {
        if self.emitted_shutdown {
            return None;
        }

        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) => {
                self.emitted_shutdown = true;
                Some(BackendInputEvent::Shutdown)
            }
            Ok(_) => line
                .chars()
                .find(|ch| !ch.is_whitespace())
                .map(|ch| BackendInputEvent::Key(BackendKeyEvent::char(ch)))
                .or(Some(BackendInputEvent::Wake)),
            Err(_) => {
                self.emitted_shutdown = true;
                Some(BackendInputEvent::Shutdown)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CellSpan {
    x: u16,
    y: u16,
    style: CellStyle,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellStyle {
    foreground: Option<Color>,
    background: Option<Color>,
    attrs: CellAttrs,
}

impl CellStyle {
    fn from_cell(cell: Cell) -> Self {
        Self {
            foreground: cell.foreground,
            background: cell.background,
            attrs: cell.attrs,
        }
    }
}

pub fn write_screen_patch_ansi(output: &mut impl Write, patch: &ScreenPatch) -> io::Result<()> {
    if patch.full {
        write!(output, "\x1b[2J")?;
    }

    for span in patch_spans(patch) {
        write!(output, "\x1b[{};{}H", span.y + 1, span.x + 1)?;
        write_sgr(output, span.style)?;
        write!(output, "{}", span.text)?;
        write!(output, "\x1b[0m")?;
    }

    Ok(())
}

pub fn write_screen_patch_crossterm(
    output: &mut impl Write,
    patch: &ScreenPatch,
) -> io::Result<()> {
    output.queue(BeginSynchronizedUpdate)?;
    if patch.full {
        output.queue(Clear(ClearType::All))?;
    }

    for span in patch_spans(patch) {
        output.queue(cursor::MoveTo(span.x, span.y))?;
        queue_crossterm_style(output, span.style)?;
        output.queue(Print(span.text))?;
        output.queue(ResetColor)?;
        output.queue(SetAttribute(CrosstermAttribute::Reset))?;
    }
    output.queue(EndSynchronizedUpdate)?;

    Ok(())
}

pub fn read_crossterm_runtime_input() -> io::Result<Option<RuntimeInput>> {
    event::read().map(crossterm_event_to_runtime_input)
}

pub fn crossterm_event_to_runtime_input(event: Event) -> Option<RuntimeInput> {
    match event {
        Event::Key(event) => crossterm_key_event_to_backend_event(event)
            .map(BackendInputEvent::Key)
            .map(BackendInputEvent::into_runtime_input),
        Event::Resize(width, height) => Some(RuntimeInput::Resize(Size::new(width, height))),
        Event::FocusGained | Event::FocusLost | Event::Mouse(_) => Some(RuntimeInput::BackendWake),
    }
}

fn crossterm_key_event_to_backend_event(event: CrosstermKeyEvent) -> Option<BackendKeyEvent> {
    if !event.is_press() && !event.is_repeat() {
        return None;
    }

    let key = match event.code {
        CrosstermKeyCode::Char(ch) => BackendKey::Char(ch),
        CrosstermKeyCode::Enter => BackendKey::Enter,
        CrosstermKeyCode::Esc => BackendKey::Esc,
        CrosstermKeyCode::Backspace => BackendKey::Backspace,
        CrosstermKeyCode::Delete => BackendKey::Delete,
        CrosstermKeyCode::Left => BackendKey::Arrow(Direction::Left),
        CrosstermKeyCode::Right => BackendKey::Arrow(Direction::Right),
        CrosstermKeyCode::Up => BackendKey::Arrow(Direction::Up),
        CrosstermKeyCode::Down => BackendKey::Arrow(Direction::Down),
        CrosstermKeyCode::Home => BackendKey::Home,
        CrosstermKeyCode::End => BackendKey::End,
        CrosstermKeyCode::PageUp => BackendKey::Page(Direction::Up),
        CrosstermKeyCode::PageDown => BackendKey::Page(Direction::Down),
        CrosstermKeyCode::Tab => BackendKey::Tab,
        CrosstermKeyCode::BackTab => BackendKey::Tab,
        _ => return None,
    };

    Some(BackendKeyEvent {
        key,
        ctrl: event.modifiers.contains(event::KeyModifiers::CONTROL),
        alt: event.modifiers.contains(event::KeyModifiers::ALT),
        shift: event.modifiers.contains(event::KeyModifiers::SHIFT)
            || matches!(event.code, CrosstermKeyCode::BackTab),
        kind: thorn_core::KeyEventKind::Press,
    })
}

pub fn crossterm_terminal_size() -> io::Result<Size> {
    let (width, height) = terminal::size()?;
    Ok(Size::new(width, height))
}

fn queue_crossterm_style(output: &mut impl Write, style: CellStyle) -> io::Result<()> {
    output.queue(ResetColor)?;
    output.queue(SetAttribute(CrosstermAttribute::Reset))?;

    if let Some(foreground) = style.foreground {
        output.queue(SetForegroundColor(crossterm_color(foreground)))?;
    }
    if let Some(background) = style.background {
        output.queue(SetBackgroundColor(crossterm_color(background)))?;
    }
    if style.attrs.contains(CellAttrs::BOLD) {
        output.queue(SetAttribute(CrosstermAttribute::Bold))?;
    }
    if style.attrs.contains(CellAttrs::UNDERLINE) {
        output.queue(SetAttribute(CrosstermAttribute::Underlined))?;
    }
    if style.attrs.contains(CellAttrs::REVERSED) {
        output.queue(SetAttribute(CrosstermAttribute::Reverse))?;
    }

    Ok(())
}

fn crossterm_color(color: Color) -> CrosstermColor {
    match color {
        Color::Default => CrosstermColor::Reset,
        Color::Indexed(index) => CrosstermColor::AnsiValue(index),
        Color::Rgb(r, g, b) => CrosstermColor::Rgb { r, g, b },
    }
}

fn crossterm_backend_error_to_io(error: BackendError) -> io::Error {
    match error {
        BackendError::UnsupportedFeature(feature) => {
            io::Error::new(io::ErrorKind::Unsupported, format!("{feature:?}"))
        }
        BackendError::PresentationFailed { message } => io::Error::other(message),
    }
}

fn write_prompt(output: &mut impl Write, screen_height: u16) -> io::Result<()> {
    let prompt_row = screen_height.saturating_add(2);
    write!(
        output,
        "\x1b[{};1H\x1b[2Kpress +, -, q then Enter > ",
        prompt_row
    )
}

fn patch_spans(patch: &ScreenPatch) -> Vec<CellSpan> {
    let mut cells = patch.cells.clone();
    cells.sort_by_key(|cell| (cell.y, cell.x));

    let mut spans: Vec<CellSpan> = Vec::new();
    for cell in cells {
        if cell.cell.wide == WideCell::Continuation {
            continue;
        }

        let style = CellStyle::from_cell(cell.cell);
        if let Some(last) = spans.last_mut() {
            let expected_next_x = last.x.saturating_add(last.text.chars().count() as u16);
            if last.y == cell.y && expected_next_x == cell.x && last.style == style {
                last.text.push(cell.cell.ch);
                continue;
            }
        }

        spans.push(CellSpan {
            x: cell.x,
            y: cell.y,
            style,
            text: cell.cell.ch.to_string(),
        });
    }

    spans
}

fn write_sgr(output: &mut impl Write, style: CellStyle) -> io::Result<()> {
    let mut codes = vec!["0".to_string()];

    if style.attrs.contains(CellAttrs::BOLD) {
        codes.push("1".to_string());
    }
    if style.attrs.contains(CellAttrs::UNDERLINE) {
        codes.push("4".to_string());
    }
    if style.attrs.contains(CellAttrs::REVERSED) {
        codes.push("7".to_string());
    }

    if let Some(color) = style.foreground {
        extend_color_codes(&mut codes, color, true);
    }
    if let Some(color) = style.background {
        extend_color_codes(&mut codes, color, false);
    }

    write!(output, "\x1b[{}m", codes.join(";"))
}

fn extend_color_codes(codes: &mut Vec<String>, color: Color, foreground: bool) {
    match color {
        Color::Default => codes.push(if foreground { "39" } else { "49" }.to_string()),
        Color::Indexed(index) => {
            codes.push(if foreground { "38" } else { "48" }.to_string());
            codes.push("5".to_string());
            codes.push(index.to_string());
        }
        Color::Rgb(r, g, b) => {
            codes.push(if foreground { "38" } else { "48" }.to_string());
            codes.push("2".to_string());
            codes.push(r.to_string());
            codes.push(g.to_string());
            codes.push(b.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use thorn_core::{
        column, text, AppContext, BackendInputEvent, Cell, CellPatch, DirtyRegion, Element,
        InputThreadStep, Key, KeyAction, KeyEvent, KeyIntent, KeyModifiers, PaintAttrs, PaintColor,
        PaintStyle, Rect, Screen, Size,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum CounterAction {
        Increment,
        Decrement,
    }

    struct CounterApp {
        count: i32,
    }

    impl ThornApp for CounterApp {
        type Action = CounterAction;

        fn update(&mut self, action: Self::Action, _ctx: &mut AppContext<Self::Action>) {
            match action {
                CounterAction::Increment => self.count += 1,
                CounterAction::Decrement => self.count -= 1,
            }
        }

        fn view(&self) -> Element<Self::Action> {
            column((
                text("Counter"),
                text(format!("count: {}", self.count)),
                text("+/- change, q quit"),
            ))
        }
    }

    struct CounterIntentMapper;

    impl IntentMapper<CounterAction> for CounterIntentMapper {
        fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<CounterAction>> {
            match intent {
                KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
                KeyIntent::App("increment") => Some(KeyAction::App(CounterAction::Increment)),
                KeyIntent::App("decrement") => Some(KeyAction::App(CounterAction::Decrement)),
                _ => None,
            }
        }
    }

    fn counter_runtime() -> TerminalRuntime<CounterApp> {
        TerminalRuntime::new(CounterApp { count: 0 }, CounterIntentMapper).size(40, 8)
    }

    fn ansi_output_for_patch(patch: &ScreenPatch) -> String {
        let mut output = Vec::new();
        write_screen_patch_ansi(&mut output, patch).unwrap();
        String::from_utf8(output).unwrap()
    }

    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.match_indices(needle).count()
    }

    #[test]
    fn terminal_input_source_normalizes_lines_to_backend_events() {
        let mut source = StdioLineEventSource::new(&b"+\n\nq\n"[..]);

        assert_eq!(
            source.read_event(),
            Some(BackendInputEvent::Key(BackendKeyEvent::char('+')))
        );
        assert_eq!(source.read_event(), Some(BackendInputEvent::Wake));
        assert_eq!(
            source.read_event(),
            Some(BackendInputEvent::Key(BackendKeyEvent::char('q')))
        );
        assert_eq!(source.read_event(), Some(BackendInputEvent::Shutdown));
        assert_eq!(source.read_event(), None);
    }

    #[test]
    fn run_with_io_uses_backend_event_queue_before_runtime_input() {
        let mut runtime = counter_runtime();
        let mut driver = InputThreadDriver::new(StdioLineEventSource::new(&b"+\n"[..]));
        let mut queue = BoundedInputQueue::new(4);

        runtime.render_text();

        assert_eq!(driver.step(&mut queue), InputThreadStep::Queued);
        assert!(runtime.render_text().contains("count: 0"));

        runtime.drain_runtime_input_queue(&mut queue);

        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn render_text_shows_initial_counter() {
        let mut runtime = counter_runtime();

        assert!(runtime.render_text().contains("count: 0"));
    }

    #[test]
    fn plus_key_updates_render_text() {
        let mut runtime = counter_runtime();

        runtime.send_key('+');

        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn run_with_io_exits_on_q() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.run_with_io(&b"q\n"[..], &mut output).unwrap();

        assert!(!runtime.is_running());
        assert!(String::from_utf8(output).unwrap().contains("Counter"));
    }

    #[test]
    fn run_with_io_renders_after_increment_before_quit() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.run_with_io(&b"+\nq\n"[..], &mut output).unwrap();

        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn custom_keymap_smoke_updates_terminal_output() {
        let mut runtime = TerminalRuntime::new(CounterApp { count: 0 }, CounterIntentMapper)
            .keymap(KeyMap::new().bind(KeyEvent::char('n'), KeyIntent::App("increment")))
            .size(40, 8);
        let mut output = Vec::new();

        runtime.run_with_io(&b"n\nq\n"[..], &mut output).unwrap();

        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn terminal_run_with_io_drains_runtime_input_queue() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.run_with_io(&b"+\nq\n"[..], &mut output).unwrap();

        assert!(!runtime.is_running());
        assert!(runtime.render_text().contains("count: 1"));
    }

    #[test]
    fn full_patch_lowers_to_ansi_cursor_moves_text_and_resets() {
        let mut screen = Screen::new(Size::new(2, 1));
        screen.cells[0] = Cell::new('A').with_style(PaintStyle {
            foreground: Some(PaintColor::Indexed(2)),
            attrs: PaintAttrs::BOLD,
            ..PaintStyle::default()
        });
        screen.cells[1] = Cell::new('B').with_style(PaintStyle {
            foreground: Some(PaintColor::Indexed(2)),
            attrs: PaintAttrs::BOLD,
            ..PaintStyle::default()
        });

        let ansi = ansi_output_for_patch(&screen.full_patch());

        assert!(ansi.starts_with("\x1b[2J"));
        assert!(ansi.contains("\x1b[1;1H"));
        assert!(ansi.contains("\x1b[0;1;38;5;2mAB\x1b[0m"));
    }

    #[test]
    fn adjacent_same_style_dirty_cells_merge_into_one_span() {
        let patch = ScreenPatch {
            size: Size::new(3, 1),
            full: false,
            regions: vec![DirtyRegion {
                rect: Rect::new(0, 0, 2, 1),
            }],
            cells: vec![
                CellPatch {
                    x: 0,
                    y: 0,
                    cell: Cell::new('A'),
                },
                CellPatch {
                    x: 1,
                    y: 0,
                    cell: Cell::new('B'),
                },
            ],
        };

        let ansi = ansi_output_for_patch(&patch);

        assert_eq!(count_occurrences(&ansi, "\x1b[1;1H"), 1);
        assert!(ansi.contains("\x1b[0mAB\x1b[0m"));
    }

    #[test]
    fn non_adjacent_or_different_style_cells_split_spans() {
        let patch = ScreenPatch {
            size: Size::new(4, 1),
            full: false,
            regions: vec![
                DirtyRegion {
                    rect: Rect::new(0, 0, 1, 1),
                },
                DirtyRegion {
                    rect: Rect::new(2, 0, 2, 1),
                },
            ],
            cells: vec![
                CellPatch {
                    x: 0,
                    y: 0,
                    cell: Cell::new('A'),
                },
                CellPatch {
                    x: 2,
                    y: 0,
                    cell: Cell::new('B'),
                },
                CellPatch {
                    x: 3,
                    y: 0,
                    cell: Cell::new('C').with_style(PaintStyle {
                        foreground: Some(PaintColor::Indexed(4)),
                        background: Some(PaintColor::Indexed(7)),
                        attrs: PaintAttrs::UNDERLINE,
                    }),
                },
            ],
        };

        let ansi = ansi_output_for_patch(&patch);

        assert!(ansi.contains("\x1b[1;1H\x1b[0mA\x1b[0m"));
        assert!(ansi.contains("\x1b[1;3H\x1b[0mB\x1b[0m"));
        assert!(ansi.contains("\x1b[1;4H\x1b[0;4;38;5;4;48;5;7mC\x1b[0m"));
    }

    #[test]
    fn incremental_draw_only_writes_dirty_cells_after_first_frame() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.draw(&mut output).unwrap();
        let first = String::from_utf8(output.clone()).unwrap();
        assert!(first.contains("\x1b[2J"));
        assert!(first.contains("Counter"));

        runtime.send_key('+');
        runtime.draw(&mut output).unwrap();
        let second = String::from_utf8(output).unwrap();
        let delta = &second[first.len()..];

        assert!(!delta.contains("\x1b[2J"));
        assert!(!delta.contains("Counter"));
        assert!(delta.contains("\x1b[2;8H"));
        assert!(delta.contains("\x1b[0m1\x1b[0m"));
    }

    #[test]
    fn resize_draw_forces_full_clear_and_repaint() {
        let mut runtime = counter_runtime();
        let mut output = Vec::new();

        runtime.draw(&mut output).unwrap();
        let first = String::from_utf8(output.clone()).unwrap();

        runtime.handle_input(RuntimeInput::Resize(Size::new(20, 4)));
        runtime.draw(&mut output).unwrap();
        let second = String::from_utf8(output).unwrap();
        let delta = &second[first.len()..];

        assert!(delta.contains("\x1b[2J"));
        assert!(delta.contains("\x1b[1;1H"));
        assert!(delta.contains("Counter"));
    }

    #[test]
    fn crossterm_writer_lowers_patch_with_cursor_style_and_text_commands() {
        let mut screen = Screen::new(Size::new(2, 1));
        screen.cells[0] = Cell::new('A').with_style(PaintStyle {
            foreground: Some(PaintColor::Rgb(1, 2, 3)),
            background: Some(PaintColor::Indexed(4)),
            attrs: PaintAttrs::BOLD,
        });
        let mut output = Vec::new();

        write_screen_patch_crossterm(&mut output, &screen.full_patch()).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("\x1b[?2026h"));
        assert!(output.contains("\x1b[2J"));
        assert!(output.contains("\x1b[1;1H"));
        assert!(output.contains("\x1b[38;2;1;2;3m"));
        assert!(output.contains("\x1b[48;5;4m"));
        assert!(output.contains('A'));
        assert!(output.ends_with("\x1b[?2026l"));
    }

    #[test]
    fn crossterm_writer_keeps_incremental_patch_without_clear() {
        let mut previous = Screen::new(Size::new(2, 1));
        previous.write_text(0, 0, "AB");
        let mut next = Screen::new(Size::new(2, 1));
        next.write_text(0, 0, "AC");
        let patch = previous.diff(&next);
        let mut output = Vec::new();

        write_screen_patch_crossterm(&mut output, &patch).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(!patch.full);
        assert_eq!(patch.cells.len(), 1);
        assert!(output.starts_with("\x1b[?2026h"));
        assert!(!output.contains("\x1b[2J"));
        assert!(output.contains("\x1b[1;2H"));
        assert!(output.contains('C'));
        assert!(output.ends_with("\x1b[?2026l"));
    }

    #[test]
    fn crossterm_key_event_normalizes_to_runtime_input() {
        let input = crossterm_event_to_runtime_input(Event::Key(CrosstermKeyEvent::new(
            CrosstermKeyCode::Char('c'),
            event::KeyModifiers::CONTROL,
        )));

        assert_eq!(
            input,
            Some(RuntimeInput::Key(KeyEvent {
                key: Key::Char('c'),
                modifiers: KeyModifiers::CTRL,
                kind: thorn_core::KeyEventKind::Press,
            }))
        );
    }

    #[test]
    fn crossterm_resize_event_normalizes_to_runtime_input() {
        assert_eq!(
            crossterm_event_to_runtime_input(Event::Resize(120, 40)),
            Some(RuntimeInput::Resize(Size::new(120, 40)))
        );
    }

    #[test]
    fn crossterm_presenter_reports_presented_frame() {
        let screen = render_screen("ok", Size::new(2, 1));
        let patch = screen.full_patch();
        let mut presenter = CrosstermPresenter::new(Vec::new());

        let frame = presenter.present(&patch).unwrap();

        assert_eq!(presenter.presented_frames(), 1);
        assert_eq!(frame.size, Size::new(2, 1));
        assert!(frame.full);
        assert_eq!(frame.changed_cells, 2);
        assert_eq!(frame.output_summary, "crossterm frame=1 cells=2 full=true");
        assert!(!presenter.output().is_empty());
    }

    fn render_screen(text: &str, size: Size) -> Screen {
        let mut screen = Screen::new(size);
        screen.write_text(0, 0, text);
        screen
    }
}
