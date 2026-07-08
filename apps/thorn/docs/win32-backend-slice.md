# Win32 Backend Slice

This is the first CLI-checkable Win32 adapter slice for Thorn.

It follows the same boundary shape observed in `apps/keydock`:

- Core owns platform-independent UI trees, patches, and backend capabilities.
- The Win32 adapter owns window policy and future HWND/message-loop/unsafe code.
- Platform handles and Win32 API types do not appear in `thorn-core`.

## Current State

`thorn-win32` provides a dry-run presenter:

- `Win32BackendConfig` records host policy such as no-activate, topmost, and tool-window behavior.
- `Win32BackendPlan` reports the adapter capability set.
- `Win32DryRunPresenter` implements `thorn_core::BackendPresenter` and accepts a `ScreenPatch` without creating a real window.

This is intentionally not a real GUI backend yet. It is a testable adapter contract that keeps the layering honest before adding a Windows-only host, message loop, DPI setup, Direct2D/DirectWrite renderer, or `windows` crate dependency.

## Verification

```powershell
cargo test -p thorn-win32
```

The workspace checks also include this crate:

```powershell
cargo check --workspace --examples
cargo test --workspace
```

## Not Covered Yet

- Real HWND creation.
- Win32 message loop.
- DPI awareness.
- Direct2D/DirectWrite rendering.
- GUI smoke with a visible or hidden native window.

