# Aster Agent Integration

This is the minimum Thorn contract for using Thorn as the UI base of `aster-agent`.

## Acceptance Criteria

- Upper layers use `thorn::prelude::*` as the stable integration surface.
- Agent state lives in an app struct implementing `ThornApp`.
- Input is normalized to `RuntimeInput`, resolved by `KeyMap` to `KeyIntent`, then mapped to app actions by `IntentMapper`.
- Agent logic mutates state only in `update`; views are returned from `view` as `Element<Action>`.
- Headless tests drive the same runtime path as terminal adapters.
- Terminal adapters stay thin and depend on `thorn-runtime`, not on app-specific agent code.

## Layering

```text
aster-agent
  -> thorn::prelude
  -> thorn-headless / thorn-terminal / thorn-runtime
  -> thorn-core
```

`thorn-core` is pure model and tree transformation code. It does not know about terminal IO, agent tools, model calls, or background tasks.

`thorn-runtime` owns input handling, intent mapping, action dispatch, render scheduling, and lifecycle.

`thorn-headless` is the primary test harness for agent-like loops.

`thorn-terminal` is currently a stdio adapter. Raw mode and real input threads belong there later.

## Agent Shape

An `aster-agent` app should look like this:

```rust
use thorn::prelude::*;

struct AgentApp {
    status: &'static str,
    transcript: Vec<&'static str>,
}

enum AgentAction {
    SubmitPrompt,
    ReceiveModelChunk,
    ReceiveToolResult,
}

impl ThornApp for AgentApp {
    type Action = AgentAction;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
        match action {
            AgentAction::SubmitPrompt => ctx.dispatch(AgentAction::ReceiveModelChunk),
            AgentAction::ReceiveModelChunk => ctx.dispatch(AgentAction::ReceiveToolResult),
            AgentAction::ReceiveToolResult => self.status = "ready",
        }
    }

    fn view(&self) -> Element<Self::Action> {
        column((text("Aster Agent"), text(self.status)))
    }
}
```

## Headless Smoke

Use `TestRuntime` to drive the same state loop without terminal IO:

```rust
let keymap = KeyMap::new().bind(KeyEvent::char('p'), KeyIntent::App("submit_prompt"));
let mut runtime = TestRuntime::new(app, mapper).keymap(keymap).size(80, 24);

runtime.send_key('p');
runtime.render_frame();
runtime.assert_text("ready");
```

See `crates/thorn/examples/aster_agent_base.rs` for a runnable integration example.

## Simulated TUI Smoke

The CLI-checkable terminal smoke uses `TerminalRuntime::run_with_io` with in-memory input and output. It proves that the terminal adapter path can drive an aster-agent-like app to ready without interactive input:

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example aster_agent_tui_smoke
```

Expected success marker:

```text
ASTER_AGENT_READY
```
