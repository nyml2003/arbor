import { invoke } from "@tauri-apps/api/core";
import type {
  AreaSelection,
  CaptureResult,
  CaptureSettings,
} from "./contracts";

export function getSettings(): Promise<CaptureSettings> {
  return invoke<CaptureSettings>("get_settings");
}

export function updateSettings(
  settings: CaptureSettings,
): Promise<CaptureSettings> {
  return invoke<CaptureSettings>("update_settings", { settings });
}

export function beginAreaCapture(): Promise<void> {
  return invoke("begin_area_capture");
}

export function openLastCapture(): Promise<void> {
  return invoke("open_last_capture");
}

export function submitAreaSelection(
  selection: AreaSelection,
): Promise<CaptureResult> {
  return invoke<CaptureResult>("submit_area_selection", { selection });
}

export function getLastSelection(): Promise<AreaSelection | null> {
  return invoke<AreaSelection | null>("get_last_selection");
}

export function getLastCaptureResult(): Promise<CaptureResult | null> {
  return invoke<CaptureResult | null>("get_last_capture_result");
}
