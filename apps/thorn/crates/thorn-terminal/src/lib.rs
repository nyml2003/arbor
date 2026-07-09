use std::io::{self, BufRead, Write};

use thorn_core::{
    BackendEventSource, BackendInputEvent, BackendKeyEvent, BoundedInputQueue, Cell, CellAttrs,
    Color, InputThreadDriver, IntentMapper, KeyMap, LayeredKeyMap, RuntimeInput, ScreenPatch,
    ThornApp, WideCell,
};
use thorn_runtime::AppRuntime;

pub struct TerminalRuntime<App>
where
    App: ThornApp,
{
    runtime: AppRuntime<App>,
    has_presented_frame: bool,
}

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
        InputThreadStep, KeyAction, KeyEvent, KeyIntent, PaintAttrs, PaintColor, PaintStyle, Rect,
        Screen, Size,
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
}
