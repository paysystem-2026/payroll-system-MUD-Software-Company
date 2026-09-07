export type BackupType = "automatic" | "manual" | "pre_restore";
export type BackupStatus = "completed" | "failed" | "restoring";

export interface BackupRecord {
  id: number;
  file_path: string;
  file_size: number;
  backup_type: BackupType;
  status: BackupStatus;
  created_at: string;
  checksum: string | null;
  encrypted: boolean;
  database_version: string | null;
  app_version: string | null;
}

export interface BackupSettings {
  enabled: boolean;
  frequency: "daily" | "weekly";
  time: string;
  retention: number;
  location: string;
}

export interface BackupStatusInfo {
  last_backup: BackupRecord | null;
  next_backup: string | null;
  backup_count: number;
  settings: BackupSettings;
}

export interface BackupResponse {
  success: boolean;
  message: string;
  backup?: BackupRecord | null;
}
