use std::time::Duration;

use thorn::prelude::*;

const COMMANDS: [&str; 3] = ["/clear", "/model", "/theme"];

#[derive(Clone)]
enum Action {
    DraftChanged(String),
    SubmitPrompt(String),
    TogglePalette,
    PaletteQuery(String),
    PaletteMove(i32),
    PaletteSubmit(String),
    Tick,
}

struct State {
    draft: String,
    messages: Vec<TranscriptMessage>,
    palette_open: bool,
    palette_query: String,
    palette_selected: usize,
    loading_phase: usize,
}

fn main() -> thorn::Result<()> {
    ThornApp::new(State {
        draft: String::new(),
        messages: vec![TranscriptMessage::new(
            "Aster",
            Token::Primary,
            "Thorn shell is ready.",
        )],
        palette_open: false,
        palette_query: String::new(),
        palette_selected: 0,
        loading_phase: 0,
    })
    .theme(Theme::dark())
    .poll_timeout(Duration::from_millis(50))
    .update(|state, action, _ctx| match action {
        Action::DraftChanged(draft) => state.draft = draft,
        Action::SubmitPrompt(prompt) => {
            let prompt = prompt.trim().to_string();
            if prompt.is_empty() {
                return;
            }
            state
                .messages
                .push(TranscriptMessage::new("You", Token::Accent, prompt.clone()));
            state.messages.push(TranscriptMessage::new(
                "Aster",
                Token::Primary,
                format!("echo: {prompt}"),
            ));
            state.draft.clear();
        }
        Action::TogglePalette => {
            state.palette_open = !state.palette_open;
            state.palette_query.clear();
            state.palette_selected = 0;
        }
        Action::PaletteQuery(query) => {
            state.palette_query = query;
            state.palette_selected = 0;
        }
        Action::PaletteMove(delta) => {
            state.palette_selected = if delta < 0 {
                state.palette_selected.saturating_sub(1)
            } else {
                state
                    .palette_selected
                    .saturating_add(1)
                    .min(COMMANDS.len().saturating_sub(1))
            };
        }
        Action::PaletteSubmit(command) => {
            state.draft = format!("{command} ");
            state.palette_open = false;
        }
        Action::Tick => state.loading_phase = state.loading_phase.wrapping_add(1),
    })
    .view(|_, state| {
        let transcript = panel(
            transcript()
                .messages(state.messages.clone())
                .empty_text("No messages yet")
                .build(),
        )
        .title(" Transcript ")
        .height(14);

        let prompt = input()
            .value(state.draft.clone())
            .placeholder("Message Aster")
            .loading_phase(state.loading_phase)
            .on_change(Action::DraftChanged)
            .on_submit(Action::SubmitPrompt)
            .build();

        if state.palette_open {
            col((
                fuzzy_panel(COMMANDS)
                    .title(" Commands ")
                    .placeholder("Filter commands")
                    .empty_text("No command matches")
                    .query(state.palette_query.clone())
                    .selected_index(state.palette_selected)
                    .on_move_selection(Action::PaletteMove)
                    .on_query_change(Action::PaletteQuery)
                    .on_submit(|selection| Action::PaletteSubmit(selection.item))
                    .build()
                    .height(7),
                transcript,
                prompt,
                text("Tab toggles commands. Esc or Ctrl-Q quits."),
            ))
        } else {
            col((
                transcript,
                prompt,
                text("Tab toggles commands. Esc or Ctrl-Q quits."),
            ))
        }
    })
    .keymap(KeyMap::new().bind(Key::Tab, Action::TogglePalette))
    .before_events(|_, ctx, _, inputs| {
        if inputs
            .iter()
            .any(|input| matches!(input, RuntimeInput::Tick))
        {
            ctx.dispatch(Action::Tick);
        }
    })
    .run()
}
