mod view;

use std::{error::Error, io, time::Duration, time::Instant};

use punctum_crossterm::{TerminalPresenter, TerminalSession, event, normalize_key_event};
use punctum_tetris::{PieceKind, TetrisCommand, TetrisState, command_for_key, transition};

use view::{should_quit, terminal_surface};

const TICK_INTERVAL: Duration = Duration::from_millis(450);

fn main() -> Result<(), Box<dyn Error>> {
    let _session = TerminalSession::enter()?;
    let mut presenter = TerminalPresenter::new(io::stdout(), 1)?;
    let mut state = TetrisState::new(PieceKind::ALL.to_vec())?;
    let mut next_tick = Instant::now() + TICK_INTERVAL;

    loop {
        presenter.present(&terminal_surface(&state))?;

        let timeout = next_tick.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            match event::read()? {
                event::Event::Key(raw) => {
                    let key = normalize_key_event(raw);
                    if should_quit(&key) {
                        break;
                    }
                    if let Some(command) = command_for_key(&key) {
                        state = transition(&state, command);
                    }
                }
                event::Event::Resize(_, _) => presenter.invalidate(),
                _ => {}
            }
        }

        let now = Instant::now();
        while now >= next_tick {
            state = transition(&state, TetrisCommand::Tick);
            next_tick += TICK_INTERVAL;
        }
    }

    Ok(())
}
