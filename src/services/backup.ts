import { invokeCommand } from "@/services/tauri";
import type { BackupRecord, BackupResponse, BackupSettings, BackupStatusInfo } from "@/types/backup";

const defaultSettings: BackupSettings = {
  enabled: true,
  frequency: "daily",
  time: "02:00",
  retention: 7,
  location: "~/.payroll-system/backups",
};

export const backupService = {
  getStatus(token: string) {
    return invokeCommand<BackupStatusInfo>("get_backup_status", { token }, {
      last_backup: null,
      next_backup: null,
      backup_count: 0,
      settings: defaultSettings,
    });
  },
  getSettings(token: string) {
    return invokeCommand<BackupSettings>("get_backup_settings", { token }, defaultSettings);
  },
  updateSettings(token: string, settings: BackupSettings) {
    return invokeCommand<BackupResponse>("update_backup_settings", { token, settings }, { success: true, message: "Settings saved" });
  },
  create(token: string) {
    return invokeCommand<BackupResponse>("create_backup", { token }, { success: false, message: "Backup is available in the desktop app." });
  },
  list(token: string) {
    return invokeCommand<BackupRecord[]>("list_backups", { token }, []);
  },
  verify(token: string, backupId: number) {
    return invokeCommand<BackupResponse>("verify_backup", { token, backupId }, { success: false, message: "Verification is available in the desktop app." });
  },
  restore(token: string, backupId: number) {
    return invokeCommand<BackupResponse>("restore_backup", { token, backupId }, { success: false, message: "Restore is available in the desktop app." });
  },
  restoreFile(token: string, path: string) {
    return invokeCommand<BackupResponse>("restore_backup_file", { token, path }, { success: false, message: "Restore is available in the desktop app." });
  },
  remove(token: string, backupId: number) {
    return invokeCommand<BackupResponse>("delete_backup", { token, backupId }, { success: false, message: "Delete is available in the desktop app." });
  },
};
