import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ToastPayload } from "../shared/contracts";
import "./style.css";

const toast = document.querySelector<HTMLElement>("#toast");
const toastOpen = document.querySelector<HTMLButtonElement>("#toastOpen");
const toastClose = document.querySelector<HTMLButtonElement>("#toastClose");
const toastMessage = document.querySelector<HTMLElement>("#toastMessage");

if (!toast || !toastOpen || !toastClose || !toastMessage) {
  throw new Error("Toast DOM is incomplete.");
}

let activePath = "";
let hideTimer: number | null = null;

async function hideToast() {
  clearHideTimer();
  await invoke("hide_toast");
}

function scheduleHide() {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
  }

  hideTimer = window.setTimeout(() => {
    void hideToast();
  }, 4500);
}

function clearHideTimer() {
  if (hideTimer !== null) {
    window.clearTimeout(hideTimer);
    hideTimer = null;
  }
}

toastOpen.addEventListener("click", () => {
  if (!activePath) {
    return;
  }

  void (async () => {
    await invoke("open_capture_path", { path: activePath });
    await hideToast();
  })();
});

toastClose.addEventListener("click", () => {
  void hideToast();
});

toast.addEventListener("mouseenter", () => {
  clearHideTimer();
});

toast.addEventListener("mouseleave", () => {
  scheduleHide();
});

window.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") {
    return;
  }

  void hideToast();
});

void listen<ToastPayload>("capture-toast", (event) => {
  const payload = event.payload;
  activePath = payload.file_path;
  toastMessage.textContent = `截图已完成：${payload.width} x ${payload.height}`;
  scheduleHide();
});
