# THEP Compliance Matrix

This matrix records the CLI-checkable Thorn baseline for using Thorn as the UI/runtime base of `aster-agent`.

`docs/THEPs` is the source of truth. This file is implementation evidence only.

## Acceptance Target

Thorn is aster-agent-ready when a non-interactive CLI command can prove:

- App state is owned by a `ThornApp` struct.
- Backend input is normalized to `RuntimeInput`.
- `KeyMap` resolves physical input to `KeyIntent`.
- `IntentMapper` resolves intents to `KeyAction`.
- App actions update state through `App::update`.
- `App::view` produces an `Element` tree.
- The tree lowers through Host, Layout, Paint, Cell Grid, and backend output.
- Headless and simulated TUI paths can drive an aster-agent-like app to ready.

## Matrix

| THEP | Required Area | Implementation Evidence | Machine Check |
| --- | --- | --- | --- |
| THEP-0001 | Backend-independent UI runtime, headless backend, terminal adapter boundary | `thorn-core` has no terminal dependency; `thorn-headless` and `thorn-terminal` depend through `thorn-runtime`; `aster_agent_tui_smoke` uses terminal adapter without core backend leakage | `cargo test --workspace`; `cargo run -p thorn --example aster_agent_tui_smoke` |
| THEP-0002 | Component/Element/Host/Layout/Paint/Backend layering | `thorn-core/src/{element,host,layout,paint,screen}.rs`; module tests compare each stage independently | `cargo test -p thorn-core` |
| THEP-0003 | Component returns Element; structural Row/Column sugar lowers to host semantics | `Element`, `row`, `column`, `text`, `view`; host tests cover same-axis flattening and boundary preservation | `cargo test -p thorn-core host::tests` |
| THEP-0004 | Host Tree is backend-independent and preserves identity | `HostNode`, `HostNodeId`, `HostKind`; host tests cover identity and backend-free lowering | `cargo test -p thorn-core host::tests` |
| THEP-0005 | App/state/action runtime, action queue, request-render, quit | `AppRuntime`, `AppContext`, `TestRuntime`; runtime tests cover action update, render scheduling, quit, custom keymaps | `cargo test -p thorn-runtime` |
| THEP-0006 | Explicit tree transformation pipeline and legal flattening | `render_to_screen` composes lower/layout/paint/screen; host/layout/paint/screen tests observe intermediate IRs | `cargo test -p thorn-core` |
| THEP-0007 | Deterministic row/column layout in TUI cell units | `layout_tree`, `Size`, `Rect`, `LayoutNode`; tests cover row/column placement and resize determinism through runtime | `cargo test -p thorn-core layout::tests`; `cargo test -p thorn-runtime resize_replaces_screen_and_requests_render` |
| THEP-0008 | Paint primitives, cell grid, dirty patch, backend adapter | `PaintPrimitive::TextRun`, `Screen`, `ScreenPatch`, `BackendCapabilities`, `BackendPresenter`, `PresentedFrame`; terminal adapter renders through runtime; tests cover cell diff, unsupported capability errors, and terminal output | `cargo test -p thorn-core backend::tests`; `cargo test -p thorn-core screen::tests`; `cargo test -p thorn-terminal` |
| THEP-0009 | Tree/render optimization observability baseline | Dirty cell patches and `FrameStats` provide the first machine-checkable observability baseline; caches remain future work | `cargo test -p thorn-core screen::tests::screen_diff_reports_changed_cells`; `cargo test -p thorn-runtime render_frame_records_frame_stats` |
| THEP-0010 | Roadmap-compatible crate split and staged implementation | Workspace contains `thorn-core`, `thorn-runtime`, `thorn-headless`, `thorn-terminal`, `thorn`; examples are CLI-runnable | `cargo check --workspace --examples` |
| THEP-0011 | RuntimeInput, KeyIntent, KeyAction, KeyMap, reserved quit, bounded input queue | `RuntimeInput::BackendWake`, `KeyMapLayer`, `KeyMapResult`, `BoundedInputQueue`, `ControlKeyAction`; tests cover duplicate binding, pass/handle, queue full, shutdown, reserved Ctrl-C | `cargo test -p thorn-core input::tests`; `cargo test -p thorn-runtime custom_keymap_cannot_disable_ctrl_c_reserved_quit` |
| THEP-0012 | Counter MVP headless pipeline and required tests | Required Counter, keymap, runtime, headless tests are present; terminal demo exceeds MVP after headless baseline | `cargo test --workspace`; `cargo run -p thorn --example counter` |
| THEP-0013 | Horizontal crate layering and vertical core modules | `thorn-core` modules match `app/element/host/layout/paint/screen/input`; dependency direction stays core -> none, runtime -> core, adapters -> runtime/core | `cargo check --workspace --examples` |

## Aster Agent Ready Smoke

The required non-interactive simulated TUI smoke is:

```powershell
cargo run -p thorn --example aster_agent_tui_smoke
```

Passing output includes:

```text
ASTER_AGENT_READY
```

## Negative Constraints

- `thorn-core` does not depend on terminal, Win32, GUI, DOM, or process IO APIs.
- Terminal behavior is isolated to `thorn-terminal`.
- `docs/THEPs` must remain unchanged by implementation work.
- Aster-agent readiness must be proven by CLI, not only by prose.

## KeyDock Reference Notes

`apps/keydock` was inspected after the required CLI target passed. Its useful backend-shape lessons for future Thorn GUI work are:

- Keep Win32 HWND, message loop, DPI, COM, and unsafe blocks in a platform adapter.
- Let the app/core layer produce platform-independent snapshots or primitives.
- Convert platform input into framework events before touching app state.
- Keep renderer crates/adapters consuming snapshots; do not let them own business state.

No Win32 adapter was added in this pass, so no GUI-specific smoke is required.
