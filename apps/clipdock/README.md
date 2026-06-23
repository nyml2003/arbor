# ClipDock

ClipDock is a native Windows clipboard dock for Arbor's Rust-native GUI track.

v0.1 is intentionally narrow:

- text clipboard history only
- in-memory history only
- max 20 recent items
- Direct2D rendering through `arbor-ui-windows`
- pure app state and view composition through `arbor-ui-core`
- Windows clipboard/input APIs isolated under `src/platform/windows`

Run:

```powershell
cargo run --manifest-path apps\clipdock\Cargo.toml
```

