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
            KeyIntent::App(_) => None,
        }
    }
}

fn main() {
    let keymap = KeyMap::new()
        .bind(KeyEvent::char('p'), KeyIntent::App("submit_prompt"))
        .bind(KeyEvent::char('q'), KeyIntent::RequestQuit);
    let mut runtime = TestRuntime::new(
        AsterAgentApp {
            prompts: 0,
            status: "idle",
            transcript: Vec::new(),
        },
        AsterIntentMapper,
    )
    .keymap(keymap)
    .size(64, 8);

    runtime.render_frame();
    println!("{}", runtime.snapshot().to_plain_text());

    runtime.send_key('p');
    runtime.render_frame();
    println!("{}", runtime.snapshot().to_plain_text());

    runtime.send_key('q');
    assert!(!runtime.is_running());
}
