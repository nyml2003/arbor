export const IpcChannels = {
  FS_LIST_DIRECTORY: "fs:listDirectory",
  FS_READ_TEXT: "fs:readText",
  DIALOG_SELECT_DIRECTORY: "dialog:selectDirectory",
} as const;

export type IpcChannel = (typeof IpcChannels)[keyof typeof IpcChannels];
