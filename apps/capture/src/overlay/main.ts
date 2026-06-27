import "./style.css";
import { submitAreaSelection } from "../shared/api";

declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke<T>(
          command: string,
          args?: Record<string, unknown>,
        ): Promise<T>;
      };
      window?: {
        getCurrentWindow(): {
          hide(): Promise<void>;
        };
      };
    };
  }
}

type DragState = Readonly<{
  startX: number;
  startY: number;
}>;

const overlayElement = document.querySelector<HTMLElement>("#overlay");
const selectionElement = document.querySelector<HTMLElement>("#selection");
const hintElement = document.querySelector<HTMLElement>("#hint");

if (!overlayElement || !selectionElement || !hintElement) {
  throw new Error("Overlay DOM is incomplete.");
}

const overlay = overlayElement;
const selection = selectionElement;
const hint = hintElement;

let dragState: DragState | null = null;
const defaultHint = "拖拽选择区域，Esc 取消。";

function resetOverlay() {
  dragState = null;
  selection.hidden = true;
  selection.style.left = "0px";
  selection.style.top = "0px";
  selection.style.width = "0px";
  selection.style.height = "0px";
  hint.textContent = defaultHint;
}

function updateSelectionBox(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
) {
  const left = Math.min(startX, currentX);
  const top = Math.min(startY, currentY);
  const width = Math.max(1, Math.abs(currentX - startX));
  const height = Math.max(1, Math.abs(currentY - startY));

  selection.hidden = false;
  selection.style.left = `${left}px`;
  selection.style.top = `${top}px`;
  selection.style.width = `${width}px`;
  selection.style.height = `${height}px`;

  hint.textContent = `选区 ${width} x ${height}`;
}

function createSelection(
  startX: number,
  startY: number,
  currentX: number,
  currentY: number,
) {
  return {
    x: Math.min(startX, currentX),
    y: Math.min(startY, currentY),
    width: Math.max(1, Math.abs(currentX - startX)),
    height: Math.max(1, Math.abs(currentY - startY)),
  };
}

async function hideOverlay() {
  try {
    await window.__TAURI__?.core?.invoke("cancel_capture");
  } catch {
    // Ignore command failures while the shell is still incomplete.
  }

  try {
    await window.__TAURI__?.window?.getCurrentWindow().hide();
  } catch {
    // Ignore window API failures while running in a plain browser preview.
  }
}

overlay.addEventListener("pointerdown", (event) => {
  dragState = {
    startX: event.clientX,
    startY: event.clientY,
  };
  updateSelectionBox(
    dragState.startX,
    dragState.startY,
    event.clientX,
    event.clientY,
  );
});

window.addEventListener("pointermove", (event) => {
  if (!dragState) {
    return;
  }

  updateSelectionBox(
    dragState.startX,
    dragState.startY,
    event.clientX,
    event.clientY,
  );
});

window.addEventListener("pointerup", (event) => {
  if (!dragState) {
    return;
  }

  const nextSelection = createSelection(
    dragState.startX,
    dragState.startY,
    event.clientX,
    event.clientY,
  );

  updateSelectionBox(
    dragState.startX,
    dragState.startY,
    event.clientX,
    event.clientY,
  );
  dragState = null;

  void (async () => {
    try {
      const result = await submitAreaSelection(nextSelection);
      hint.textContent = `已生成截图文件：${result.width} x ${result.height}。当前还没有接入剪贴板和通知。`;
    } catch {
      hint.textContent = "截图生成失败。当前只接了一部分主链路。";
    }
  })();
});

window.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") {
    return;
  }

  void hideOverlay();
});

window.addEventListener("focus", () => {
  resetOverlay();
});

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") {
    resetOverlay();
  }
});

resetOverlay();
