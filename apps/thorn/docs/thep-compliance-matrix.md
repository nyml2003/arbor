# THEP Gap Matrix

This file is implementation evidence, not the source of truth. `docs/THEPs/**` remains immutable and authoritative.

Status terms:

- `Covered`: required behavior has code plus a CLI-checkable test or smoke.
- `Partial`: some required behavior exists, but the THEP surface is not complete.
- `Gap`: behavior is not implemented yet.

## Current Truth

Thorn is aster-agent-ready for the current simulated headless/stdio loop, but it is not yet fully compliant with every accepted THEP.

The current reliable aster-agent base is:

```text
BackendInputEvent / RuntimeInput
  -> LayeredKeyMap
  -> KeyIntent
  -> IntentResolver
  -> KeyAction
  -> App Action
  -> ThornApp::update
  -> ThornApp::view
  -> Element
  -> Host Tree
  -> Layout
  -> PaintPrimitive
  -> Screen / dirty regions
  -> Headless or stdio terminal output
```

## Matrix

| THEP | Status | Covered Evidence | Remaining Gaps |
| --- | --- | --- | --- |
| THEP-0001 | Partial | Core/runtime/headless/terminal layering exists; simulated aster-agent TUI smoke prints `ASTER_AGENT_READY`; `thorn-core` has no terminal or Win32 dependency | Native GUI/Web backends are only adapter-shaped, not real backends |
| THEP-0002 | Partial | Separate modules for app, element, host, layout, paint, screen, input; tests exercise each stage | Layer model is still thin; component and backend model need richer host/layout semantics |
| THEP-0003 | Partial | `text`, `view`, `row`, `column`; row/column sugar lowers and same-axis stack flattening is tested | Missing real `Fragment`, `If`, `For`, `Slot`, `ThemeScope`, `TextInput`, `ScrollView`, `Image`, `Layer`, `Clip` element helpers and composite component tests |
| THEP-0004 | Partial | `HostNode`, `HostNodeId`, `HostKind`, stable IDs, backend-free snapshot tests | Host node does not yet carry optional key, scope identity, style tokens, accessibility metadata, focus/input affordances, action binding, or debug provenance |
| THEP-0005 | Partial | `ThornApp`, `AppContext`, action queue, ordered update, request-render, quit, builder-like runtime construction, app action can quit, `dispatch_key_intent`, `dispatch_key_action`, backend capabilities, theme placeholder | Public `thorn::app(initial_state).update(...).view(...).run()` facade is not implemented yet; services/ports are not modeled |
| THEP-0006 | Partial | `render_pipeline` exposes host/layout/paint/screen; transparent same-axis stack flattening tested | Host normalization is not a separate pass; boundary metadata is incomplete, so full flattening legality is not implemented |
| THEP-0007 | Gap | Basic deterministic row/column layout and resize tests exist | Missing `LayoutConstraints`, `BackendMetrics`, fixed/min/flex/gap/padding/margin/alignment/clip/scroll viewport, display-width text measurement, content/clip rects, overflow, baseline metrics |
| THEP-0008 | Partial | `PaintPrimitive` includes FillRect/TextRun/Border/Cursor/Clip/Layer; `Cell` includes char/fg/bg/attrs/wide state; `ScreenPatch` includes dirty regions; unsupported capabilities are structured; headless can snapshot paint output | Terminal lowering handles text and fill at screen level but not ANSI span merge; unsupported errors are not yet tied to every host feature |
| THEP-0009 | Partial | `DirtyKind`, merge tests, `FrameStats` THEP fields, `PerfSink`, `NoopPerfSink`, dirty regions and backend output size stats | Layout/paint caches are not implemented; phase timings are coarse; tests for real cache invalidation are not complete |
| THEP-0010 | Partial | Workspace stages exist and headless/terminal examples run | Stage 2-8 are not fully implemented; current code has moved beyond MVP but not completed roadmap |
| THEP-0011 | Covered for current scope | `BackendInputEvent`, `InputThreadDriver`, bounded queue, shutdown signal, backend key conversion, `LayeredKeyMap`, priority tests, reserved Ctrl-C, app Esc override, mode `q`, focused control priority, text input control action resolution, built-in presets | Real terminal raw-mode input thread is not implemented; mouse/IME remain non-goals |
| THEP-0012 | Covered | Required Counter MVP tests and headless pipeline exist; counter CLI smoke works | MVP non-goals remain intentionally outside THEP-0012 |
| THEP-0013 | Partial | Crate split follows core/runtime/adapters/facade; `thorn-win32` is adapter-only and does not pollute core | Need CLI tests that parse Cargo manifests and assert dependency direction |

## CLI Evidence

Primary checks:

```powershell
cargo fmt --all
cargo check --workspace --examples
cargo test --workspace
cargo run -p thorn --example aster_agent_tui_smoke
@('+','q') | cargo run -p thorn --example counter
git -c safe.directory=C:/Users/nyml/code/arbor -C C:/Users/nyml/code/arbor diff --name-status -- apps/thorn/docs/THEPs
```

Focused checks added after the corrected assessment:

```powershell
cargo test -p thorn-core input::tests
cargo test -p thorn-runtime
cargo test -p thorn-headless
```

## Do Not Overclaim

Do not mark all accepted THEPs as complete until the gaps above are implemented with machine-checkable tests.

