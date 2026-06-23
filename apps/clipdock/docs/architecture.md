# ClipDock Architecture

## Boundary

ClipDock is a second validation app for the Rust-native GUI path:

- `arbor-ui-core`: geometry, pointer events, component DSL, primitive tree.
- `arbor-ui-windows`: Direct2D/DirectWrite primitive renderer.
- `apps/clipdock/src/app`: product state, layout, interaction, view DSL.
- `apps/clipdock/src/platform/windows`: Win32 window, clipboard listener, clipboard read/write, input injection.

The app layer must not import `windows::Win32`, `HWND`, `SendInput`, `unsafe`, or `arbor_ui_windows`.

## Windows API Choices

Use modern listener registration:

- `AddClipboardFormatListener`
- `RemoveClipboardFormatListener`
- `WM_CLIPBOARDUPDATE`

Do not use the legacy clipboard viewer chain (`SetClipboardViewer`, `ChangeClipboardChain`, `WM_DRAWCLIPBOARD`).

Rendering stays GDI-free in the main path. The window procedure validates paint through `DefWindowProcW`, then redraws with Direct2D via `arbor-ui-windows::Renderer`.

## Unsafe Scope

Unsafe code is allowed only inside `src/platform/windows` and only around FFI calls or Win32 ABI glue.

Rules:

- Convert Win32 handles and buffers into Rust values at the platform boundary.
- Return app commands from the pure layer instead of letting app state call platform APIs.
- Keep clipboard memory locking/unlocking inside `clipboard` module.
- Keep input injection inside `input` module.
- Keep HWND lifetime and window userdata inside `host` module.

## Milestone

v0.1 is complete when:

- `cargo test` passes for ClipDock.
- `cargo check --target x86_64-pc-windows-msvc` passes.
- `cargo clippy --all-targets -- -D warnings` passes.
- Boundary scans show app layer has no Win32/unsafe/native renderer imports.

