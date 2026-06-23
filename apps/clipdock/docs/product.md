# ClipDock Product Plan

## Name

ClipDock

The name follows KeyDock: short, literal, dock-shaped, and suitable as a sibling native utility in Arbor.

## Product Shape

ClipDock is a small always-on-top clipboard history surface.

v0.1 focuses on a single reliable workflow:

1. User copies text anywhere in Windows.
2. ClipDock receives a clipboard update.
3. The copied text appears at the top of a compact history list.
4. User clicks an item.
5. ClipDock writes that text back to the clipboard and sends `Ctrl+V`.

## v0.1 Scope

- Text only: `CF_UNICODETEXT`.
- In memory only: no disk persistence.
- History cap: 20 items.
- Duplicate handling: newest duplicate moves to the top.
- List UI: title, close button, status text, visible recent items.
- Pointer feedback: hover, press, ripple.
- Drag support: title strip is draggable.

## Explicitly Out Of Scope

- Image clipboard history.
- Rich text formats.
- Persistence, encryption, sync, search, pinning.
- Cross-platform adapters.
- Shared host crate extraction.

Those are deferred until the second app proves the app/core/renderer boundaries.

