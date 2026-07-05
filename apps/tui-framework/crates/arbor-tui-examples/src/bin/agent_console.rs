use arbor_tui::prelude::*;

#[derive(Default)]
struct AgentState {
    messages: Vec<String>,
    running: bool,
    loading_phase: usize,
}

enum Action {
    SubmitPrompt(String),
    StreamDone,
}

fn main() -> Result<()> {
    ArborApp::new(AgentState::default())
        .theme(Theme::dark())
        .update(update)
        .view(view)
        .run()
}

fn update(state: &mut AgentState, action: Action, ctx: &mut AppContext<Action>) {
    match action {
        Action::SubmitPrompt(text) if text.trim() == "/done" => {
            ctx.dispatch(Action::StreamDone);
        }
        Action::SubmitPrompt(text) => {
            state.messages.push(format!("You: {text}"));
            state.messages.push("Agent: running...".to_string());
            state.running = true;
            state.loading_phase = state.loading_phase.wrapping_add(1);
        }
        Action::StreamDone => {
            state.running = false;
            state.messages.push("Agent: done".to_string());
        }
    }
}

fn view(state: &AgentState, ui: &Ui<Action>) -> Node<Action> {
    let transcript = if state.messages.is_empty() {
        "No messages yet".to_string()
    } else {
        state.messages.join("\n")
    };

    ui.page()
        .title("Arbor Agent Console")
        .header(ui.status_line(if state.running {
            "Status: Running"
        } else {
            "Status: Idle"
        }))
        .body(
            ui.row()
                .fill()
                .child(
                    ui.panel(ui.text("Tasks\n> build ui\n  run tests\n  review"))
                        .title(" Tasks ")
                        .fill()
                        .build(),
                )
                .child(
                    ui.panel(ui.text(transcript))
                        .title(" Transcript ")
                        .fill()
                        .build(),
                )
                .child(
                    ui.panel(ui.text("Files\nCargo.toml\nui.rs\nstate.rs"))
                        .title(" Context ")
                        .fill()
                        .build(),
                )
                .build(),
        )
        .footer(
            ui.prompt("ask agent / type /done")
                .loading(state.running)
                .loading_phase(state.loading_phase)
                .on_submit(Action::SubmitPrompt)
                .build(),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_agent_console_with_facade_api() {
        let mut app = TestApp::new(AgentState::default(), update, view).theme(Theme::light());

        app.render(80, 16)
            .assert_text("Arbor Agent Console")
            .assert_text("Tasks")
            .assert_text("Transcript")
            .assert_text("Context")
            .assert_no_default_bg();

        app.dispatch(Action::SubmitPrompt("fix layout".to_string()));
        app.render(80, 16)
            .assert_text("Status: Running")
            .assert_text("You: fix layout")
            .assert_text("Agent: running")
            .assert_no_default_bg();
    }
}
