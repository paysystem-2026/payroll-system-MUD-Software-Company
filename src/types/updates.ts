export interface UpdateMetadata {
  version: string;
  current_version: string;
  notes: string | null;
  date: string | null;
  size_bytes: number | null;
  target: string;
  platform: string;
  architecture: string;
  compatible: boolean;
  signature_present: boolean;
}

export interface UpdateProgress {
  downloaded_bytes: number;
  total_bytes: number | null;
  percent: number | null;
}

export interface UpdateStatus {
  current_version: string;
  state: string;
  message: string;
  update: UpdateMetadata | null;
  progress: UpdateProgress;
  last_checked: string | null;
  service_configured: boolean;
  source: string;
  platform: string;
  architecture: string;
  checksum_present: boolean;
  recovery_available: boolean;
  recovery_created_at: string | null;
}

export interface UpdateHistoryEntry {
  id: number;
  version_from: string | null;
  version_to: string;
  status: string;
  event_type: string | null;
  details: string | null;
  size_bytes: number | null;
  created_at: string;
}

export interface UpdateStatusResponse {
  status: UpdateStatus;
  history: UpdateHistoryEntry[];
}

export interface UpdateActionResponse {
  success: boolean;
  message: string;
}
