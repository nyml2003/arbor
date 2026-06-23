export const IpcChannels = {
  FS_LIST_DIRECTORY: "fs:listDirectory",
  FS_READ_TEXT: "fs:readText",
  FS_WRITE_TEXT: "fs:writeText",
  DIALOG_SELECT_DIRECTORY: "dialog:selectDirectory",
  MEMVFS_STATUS: "memvfs:status",
  MEMVFS_START: "memvfs:start",
  MEMVFS_STOP: "memvfs:stop",
  MEMVFS_RESET: "memvfs:reset",
  MEMVFS_REQUEST: "memvfs:request",
} as const;

export type IpcChannel = (typeof IpcChannels)[keyof typeof IpcChannels];
