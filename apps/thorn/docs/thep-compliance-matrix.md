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
| THEP-0003 | Partial | `text`, `view`, `row`, `column`, `scroll_view`, `clip`, and `layer`; row/column sugar lowers and same-axis stack flattening is tested; helper-specific host/paint tests cover preserved `ScrollView`/`Clip`/`Layer` boundaries, scroll viewport clipping with retained logical content, clip lowering into `PaintPrimitive::Clip`, and layer lowering into `PaintPrimitive::Layer` with stable z-index | Missing real `Fragment`, `If`, `For`, `Slot`, `ThemeScope`, `TextInput`, `Image`, and broader composite component tests |
| THEP-0004 | Partial | `HostNode`, `HostNodeId`, `HostKind`, stable IDs, backend-free snapshot tests | Host node does not yet carry optional key, scope identity, style tokens, accessibility metadata, focus/input affordances, action binding, or debug provenance |
| THEP-0005 | Partial | `ThornApp`, `AppContext`, action queue, ordered update, request-render/invalidation, quit, facade builder `thorn::app(...).update(...).view(...).run()`, facade keymap/mode-keymap runtime chaining, app action can quit, `dispatch_key_intent`, `dispatch_key_action`, backend capabilities, theme contract, theme-fed render pipeline, facade tests cover builder runtime/keymap behavior | Services/ports are not modeled; facade surface still does not expose the full runtime/services chain |
| THEP-0006 | Partial | `render_pipeline` exposes host/layout/paint/screen; transparent same-axis stack flattening tested | Host normalization is not a separate pass; boundary metadata is incomplete, so full flattening legality is not implemented |
| THEP-0007 | Covered | `LayoutConstraints`, `BackendMetrics`, `LayoutNode` `rect`/`measured_size`/`content_rect`/`clip_rect`/`overflow`/`text_metrics`, deterministic row/column and constraint/size-varied layout tests exist; `Element -> HostNode` carries backend-independent `LayoutStyle { gap, padding, margin, fixed_size, min_size, flex_grow, main_axis_alignment, cross_axis_alignment, scroll_offset }`; `LayoutNode` reflects it through `rect`/`measured_size`/`content_rect`/`clip_rect`/`overflow`; `measured_size` records measurement before final rect/clip truncation and may reflect `fixed_size` or `min_size`; text nodes record backend-provided `line_height`/`baseline` metadata when available, view nodes do not falsely report text metrics, and metadata does not change geometry; row/column gap, padding, margin, fixed-size, min-size, main-axis flex grow, main/cross-axis alignment, scroll viewport clipping, and common terminal display-width handling tests cover child placement, retained logical offscreen nodes, clipping, overflow, deterministic extra-space distribution, deterministic group offsetting, and same-axis stack boundary preservation | None |
| THEP-0008 | Partial | `PaintPrimitive` includes FillRect/TextRun/Border/Cursor/Clip/Layer; `Cell` includes char/fg/bg/attrs/wide state; `ScreenPatch` includes dirty regions; unsupported capabilities are structured; headless can snapshot paint output; `thorn-terminal` lowers full and incremental `ScreenPatch` output to ANSI cursor movement plus SGR-styled text spans, merges adjacent same-style dirty cells on a row, and `TerminalRuntime::draw` now presents `render_patch()` output instead of full-screen plain-text clears | Unsupported errors are not yet tied to every host feature; terminal lowering still covers the current text/cell-grid path rather than every host feature-specific backend call |
| THEP-0009 | Partial | `DirtyKind`/`FrameInvalidation` merge semantics, runtime invalidation tracking, `PerfSink`, `NoopPerfSink`, dirty regions and backend output size stats; `thorn-runtime` now retains and reuses a real layout cache on `DirtyKind::Render` when size/structure state is unchanged, exposes machine-checkable `FrameStats { layout_cache_hit, layout_passes }`, and invalidates layout cache on `Layout`/`Structure`/`Theme`/`Full` | Paint cache and finer-grained dirty-node caches are not implemented; invalidation remains whole-frame rather than scoped retained subtrees; timings remain coarse |
| THEP-0010 | Partial | Workspace stages exist and headless/terminal examples run | Stage 2-8 are not fully implemented; current code has moved beyond MVP but not completed roadmap |
| THEP-0011 | Partial | `BackendInputEvent`, `InputThreadDriver`, bounded queue, shutdown signal, backend key conversion, `LayeredKeyMap`, priority tests, reserved Ctrl-C, app Esc override, mode `q`, focused control priority, text input control action resolution, built-in presets; input-thread/keymap utilities exist and tests pass; `thorn-terminal` stdio simulated loop now normalizes lines through a backend event source, enqueues `RuntimeInput` via `InputThreadDriver` + `BoundedInputQueue`, and only mutates app state when the UI thread drains that queue | Real terminal/runtime thread wiring is still a gap; terminal raw-mode input thread is not implemented; mouse/IME remain non-goals |
| THEP-0012 | Covered | Required Counter MVP tests and headless pipeline exist; counter CLI smoke works | MVP non-goals remain intentionally outside THEP-0012 |
| THEP-0013 | Covered | `cargo test -p thorn --lib` runs parser-based manifest tests that parse workspace and crate `Cargo.toml` files, verify THEP-0013 MVP crates (`thorn-core`, `thorn-runtime`, `thorn-headless`, `thorn-terminal`, `thorn`) are present, assert required dependency direction across core/runtime/MVP adapters/facade, and forbid lower-layer facade dependencies; extra adapter crates such as `thorn-win32`, if present, are constrained separately and are not used as MVP coverage evidence | None |

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
