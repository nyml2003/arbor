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
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_live
```

Press Enter to increment the counter. Press `q` or Esc to quit.

Snapshot demo:

```powershell
cargo run --manifest-path apps/thorn/Cargo.toml -p thorn --example counter_demo
```

The demo prints the MVP counter screen before and after a simulated `+1` button press.

## Current MVP

Status: complete for THEP-0008.

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

It intentionally does not include a real terminal runtime, `Memo`, `Show`, `For`, `Input`, async effects, resize handling, mouse handling, or render cache.
