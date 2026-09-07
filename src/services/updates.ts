import { invokeCommand } from "@/services/tauri";
import { authService } from "@/services/auth";
import type { UpdateActionResponse, UpdateStatusResponse } from "@/types/updates";

const fallback: UpdateStatusResponse = {
  status: {
    current_version: "1.0.0",
    state: "unavailable",
    message: "The desktop updater is available only in the Tauri application.",
    update: null,
    progress: { downloaded_bytes: 0, total_bytes: null, percent: null },
    last_checked: null,
    service_configured: false,
    source: "Not configured",
    platform: "preview",
    architecture: "preview",
    checksum_present: false,
    recovery_available: false,
    recovery_created_at: null,
  },
  history: [],
};

export const updatesService = {
  getStatus() {
    return invokeCommand<UpdateStatusResponse>("get_update_status", {}, fallback);
  },
  check() {
    return invokeCommand<UpdateActionResponse>("check_for_updates", {}, { success: false, message: "The desktop updater is unavailable in browser preview." });
  },
  download() {
    return invokeCommand<UpdateActionResponse>("download_update", {}, { success: false, message: "The desktop updater is unavailable in browser preview." });
  },
  cancelDownload() {
    return invokeCommand<UpdateActionResponse>("cancel_update_download", {}, { success: false, message: "The desktop updater is unavailable in browser preview." });
  },
  async install(token: string) {
    let activeToken = token;
    try {
      const status = await authService.getStatus();
      if (status.token) activeToken = status.token;
    } catch {
      // Keep the authenticated context token if the status refresh is unavailable.
    }
    return invokeCommand<UpdateActionResponse>("install_update", { token: activeToken }, { success: false, message: "The desktop updater is unavailable in browser preview." });
  },
  restart() {
    return invokeCommand<UpdateActionResponse>("restart_update", {}, { success: false, message: "Restart is available in the Tauri desktop application." });
  },
};
