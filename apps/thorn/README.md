# Thorn

Rust TUI framework experiment under Arbor.

Thorn does not extend `arbor-tui`. It is a fresh vertical slice that validates:

- signal-driven dynamic primitive slots
- Row/Col layout with fixed size, flex, padding and gap
- theme token resolution for dark and light themes
- in-memory screen rendering and row dirty diff
- `TestApp` assertions for MVP behavior

## Workspace

```text
apps/thorn/
  crates/
    thorn-core/      pure reactive/view/layout/theme/render/testing core
    thorn-terminal/  terminal backend boundary and memory backend stub
    thorn/           user facade and prelude
```

## Verify

```powershell
cargo check --manifest-path apps/thorn/Cargo.toml --workspace
cargo test --manifest-path apps/thorn/Cargo.toml --workspace
```

## Demo

Interactive crossterm demo:

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example keyboard_counter
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_live
```

`keyboard_counter` is the stateful Action Runtime smoke demo. Press `+` or `-` to change the counter. Press Ctrl-Q or Esc to quit. Mouse input is not supported.

Snapshot demo:

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_demo
```

The demo prints the MVP counter screen as an in-memory snapshot.

## Current MVP

Status: complete for THEP-0008. The next keyboard Action Runtime slice from THEP-0010 is underway.

The MVP covers the counter flow from THEP-0008:

```text
Signal write
  -> Effect rerun
  -> Primitive slot update
  -> Flex layout
  -> Theme resolve
  -> Screen render
  -> Diff dirty regions
  -> TestApp assert
```

It intentionally does not include mouse input, `Memo`, `Show`, `For`, `Input`, async effects, full keyboard command dispatch, or render cache.

The Action Runtime slice now includes platform-neutral keyboard and resize input, crossterm-to-core conversion, keymap-driven action dispatch, a stateful `ThornApp` builder, `TestRuntime` scripted key/resize tests, and the `keyboard_counter` demo.
