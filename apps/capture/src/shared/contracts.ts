export type CaptureSettings = Readonly<{
  hotkey: string;
  notification_enabled: boolean;
  cache_limit: number;
  launch_on_login: boolean;
}>;

export type CaptureResult = Readonly<{
  file_path: string;
  width: number;
  height: number;
  copied: boolean;
  notified: boolean;
}>;

export type CaptureError = Readonly<{
  code: string;
  message: string;
}>;

export type AreaSelection = Readonly<{
  x: number;
  y: number;
  width: number;
  height: number;
}>;

export const defaultCaptureSettings: CaptureSettings = {
  hotkey: "CommandOrControl+Shift+4",
  notification_enabled: true,
  cache_limit: 50,
  launch_on_login: false,
};
