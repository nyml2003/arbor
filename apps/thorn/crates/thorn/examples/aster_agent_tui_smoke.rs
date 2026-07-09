use thorn::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentAction {
    SubmitPrompt,
    ReceiveModelChunk,
    ReceiveToolResult,
    Finish,
}

struct AsterAgentApp {
    prompts: u32,
    status: &'static str,
    transcript: Vec<&'static str>,
}

impl ThornApp for AsterAgentApp {
    type Action = AgentAction;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
        match action {
            AgentAction::SubmitPrompt => {
                self.prompts += 1;
                self.status = "thinking";
                self.transcript.push("user: summarize workspace");
                ctx.dispatch(AgentAction::ReceiveModelChunk);
            }
            AgentAction::ReceiveModelChunk => {
                self.transcript.push("assistant: reading project state");
                ctx.dispatch(AgentAction::ReceiveToolResult);
            }
            AgentAction::ReceiveToolResult => {
                self.status = "ready";
                self.transcript.push("tool: cargo check passed");
            }
            AgentAction::Finish => {
                self.status = "done";
                ctx.quit();
            }
        }
    }

    fn view(&self) -> Element<Self::Action> {
        column((
            row((text("Aster Agent"), text(format!(" [{}]", self.status)))),
            text(format!("prompts: {}", self.prompts)),
            text(self.transcript.last().copied().unwrap_or("idle")),
        ))
    }
}

struct AsterIntentMapper;

impl IntentMapper<AgentAction> for AsterIntentMapper {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<AgentAction>> {
        match intent {
            KeyIntent::RequestQuit => Some(KeyAction::App(AgentAction::Finish)),
            KeyIntent::App("submit_prompt") => Some(KeyAction::App(AgentAction::SubmitPrompt)),
            _ => None,
        }
    }
}

fn main() -> std::io::Result<()> {
    let keymap = KeyMap::new()
        .bind(KeyEvent::char('p'), KeyIntent::App("submit_prompt"))
        .bind(KeyEvent::char('q'), KeyIntent::RequestQuit);
    let mut runtime = TerminalRuntime::new(
        AsterAgentApp {
            prompts: 0,
            status: "idle",
            transcript: Vec::new(),
        },
        AsterIntentMapper,
    )
    .keymap(keymap)
    .size(64, 8);

    let mut output = Vec::new();
    runtime.run_with_io(&b"p\nq\n"[..], &mut output)?;
    let output = String::from_utf8_lossy(&output);
    let final_screen = runtime.render_text();
    print!("{output}");

    if final_screen.contains("Aster Agent [ready]")
        && final_screen.contains("prompts: 1")
        && final_screen.contains("tool: cargo check passed")
    {
        println!("\nASTER_AGENT_READY");
        Ok(())
    } else {
        eprintln!("ASTER_AGENT_NOT_READY");
        std::process::exit(1);
    }
}
