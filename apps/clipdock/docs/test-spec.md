# ClipDock Test Spec

## App Layer

- New text item appears at the top of history.
- Empty and whitespace-only clipboard text is ignored.
- Repeated text is deduplicated and moved to the top.
- History is capped at 20 items.
- Pointer down/up on an item emits a paste command.
- Pointer down/up on the close button emits a close command.
- Resize recomputes non-overlapping item rects.
- Snapshot returns a primitive tree rooted in a surface.

## Platform Layer

Manual v0.1 checks:

- Starting `clipdock.exe` opens only the GUI window, no console window.
- Copying text in another application updates ClipDock.
- Clicking a history item pastes the selected text into the foreground app.
- The title strip drags the window.
- The close button closes the window.

## Boundary Checks

Run:

```powershell
rg -n "crate::platform|windows::Win32|unsafe|HWND|SendInput|arbor_ui_windows" apps\clipdock\src\app
rg -n "Graphics::Gdi|Win32_Graphics_Gdi|BeginPaint|EndPaint|InvalidateRect|ScreenToClient|PAINTSTRUCT|SetClipboardViewer|ChangeClipboardChain|WM_DRAWCLIPBOARD" apps\clipdock\src packages\arbor-ui-windows\src
```
