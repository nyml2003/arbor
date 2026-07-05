use arbor_tui_adapters::crossterm_backend::CrosstermBackend;
use arbor_tui_adapters::stdin_reader::StdinReader;
use arbor_tui_application::terminal_app::{TerminalApp, TerminalAppResult};

pub fn run_crossterm_terminal_app(app: TerminalApp) -> TerminalAppResult<()> {
    let mut backend = CrosstermBackend::new();
    let input = StdinReader::new();
    app.run(&mut backend, &input)
}
