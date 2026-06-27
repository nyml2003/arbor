export const IpcChannels = {
  FS_LIST_DIRECTORY: "fs:listDirectory",
  FS_READ_TEXT: "fs:readText",
  FS_WRITE_TEXT: "fs:writeText",
  DIALOG_SELECT_DIRECTORY: "dialog:selectDirectory",
  MANAGE_LIST: "manage:list",
  MANAGE_CREATE: "manage:create",
  MANAGE_UPDATE: "manage:update",
  MANAGE_COMPLETE: "manage:complete",
  MANAGE_RESTORE: "manage:restore",
  MEMVFS_STATUS: "memvfs:status",
  MEMVFS_START: "memvfs:start",
  MEMVFS_STOP: "memvfs:stop",
  MEMVFS_RESET: "memvfs:reset",
  MEMVFS_REQUEST: "memvfs:request",
} as const;

export type IpcChannel = (typeof IpcChannels)[keyof typeof IpcChannels];
