export interface AuthStatus {
  is_setup: boolean;
  is_authenticated: boolean;
  is_locked: boolean;
  admin_username: string | null;
  token?: string | null;
}

export interface AuthResponse {
  success: boolean;
  message: string;
  token: string | null;
  recovery_code?: string | null;
}

export interface SecuritySettings {
  auto_lock_minutes: number;
  session_timeout_minutes: number;
}


export interface AuditEntry {
  id: number; action: string; entity_type: string | null; entity_id: number | null; details: string | null; created_at: string;
}

export interface SystemInfo {
  app_version: string; database_version: string; database_path: string; database_size_bytes: number;
  employee_count: number; department_count: number; backup_count: number; paired_device_count: number; audit_count: number;
  platform: string; architecture: string;
}
