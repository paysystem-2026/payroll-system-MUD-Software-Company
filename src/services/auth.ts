import { invoke } from "@tauri-apps/api/core";
import type { AuthStatus, AuthResponse, SecuritySettings } from "@/types/auth";

const isDesktop =
  typeof window !== "undefined" &&
  Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

interface DevAuthStore {
  admin: { username: string; password: string; recoveryCode: string } | null;
  loggedIn: boolean;
  locked: boolean;
}

const DEV_KEY = "payroll-dev-auth-v1";

function loadDevStore(): DevAuthStore {
  if (typeof window === "undefined") {
    return { admin: null, loggedIn: false, locked: false };
  }
  try {
    const raw = sessionStorage.getItem(DEV_KEY);
    if (raw) return JSON.parse(raw) as DevAuthStore;
  } catch {
    // Ignore malformed development-only state.
  }
  return { admin: null, loggedIn: false, locked: false };
}

let devStore: DevAuthStore = loadDevStore();

export const getAuthSessionToken = (provided?: string | null) => {
  const candidate = provided?.trim();
  if (candidate) return candidate;
  if (typeof window !== "undefined") return sessionStorage.getItem("payroll_session_token") ?? "";
  return "";
};

// Desktop security operations must use the backend's current session as the
// source of truth. Renderer sessionStorage can be stale after a reload, lock,
// or WebView lifecycle event, so refresh the token from SQLite before any
// privileged Administration operation.
async function resolveDesktopSessionToken(provided?: string | null): Promise<string> {
  const local = getAuthSessionToken(provided);
  if (!isDesktop) return local;

  const status = await invoke<AuthStatus>("get_auth_status");
  const backendToken = status.token?.trim() ?? "";
  if (!status.is_authenticated || !backendToken) {
    throw new Error(status.is_locked
      ? "Session is locked. Unlock the workspace first."
      : "Authentication session is unavailable. Please sign in again.");
  }

  if (typeof window !== "undefined") sessionStorage.setItem("payroll_session_token", backendToken);
  return backendToken;
}

function persistDevStore() {
  if (typeof window === "undefined") return;
  sessionStorage.setItem(DEV_KEY, JSON.stringify(devStore));
}

function devState(): AuthStatus {
  return {
    is_setup: !!devStore.admin,
    is_authenticated: devStore.loggedIn,
    is_locked: devStore.locked,
    admin_username: devStore.admin?.username ?? null,
  };
}

function createRecoveryCode() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const values = new Uint32Array(16);
  crypto.getRandomValues(values);
  return Array.from(values, (v) => chars[v % chars.length]).join("");
}

export const authService = {
  getStatus(): Promise<AuthStatus> {
    return isDesktop ? invoke<AuthStatus>("get_auth_status") : Promise.resolve(devState());
  },

  setup(username: string, password: string): Promise<AuthResponse> {
    if (isDesktop) return invoke<AuthResponse>("setup_admin", { request: { username, password } });
    if (devStore.admin) return Promise.resolve({ success: false, message: "Admin already exists", token: null });
    if (username.trim().length < 3) return Promise.resolve({ success: false, message: "Username must be at least 3 characters", token: null });
    if (password.length < 8) return Promise.resolve({ success: false, message: "Password must be at least 8 characters", token: null });
    const recoveryCode = createRecoveryCode();
    devStore.admin = { username: username.trim(), password, recoveryCode };
    devStore.loggedIn = true;
    devStore.locked = false;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Setup complete", token: "preview", recovery_code: recoveryCode });
  },

  login(username: string, password: string): Promise<AuthResponse> {
    if (isDesktop) return invoke<AuthResponse>("login", { request: { username, password } });
    const admin = devStore.admin;
    if (!admin || admin.username !== username.trim() || admin.password !== password) {
      return Promise.resolve({ success: false, message: "Invalid credentials", token: null });
    }
    devStore.loggedIn = true;
    devStore.locked = false;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Login successful", token: "preview" });
  },

  logout(token: string): Promise<AuthResponse> {
    if (isDesktop) return invoke<AuthResponse>("logout", { token });
    devStore.loggedIn = false;
    devStore.locked = false;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Logged out", token: null });
  },

  lock(token: string): Promise<AuthResponse> {
    if (isDesktop) return invoke<AuthResponse>("lock_session", { token });
    if (!devStore.loggedIn) return Promise.resolve({ success: false, message: "Not logged in", token: null });
    devStore.locked = true;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Session locked", token: null });
  },

  unlock(token: string, password: string): Promise<AuthResponse> {
    if (isDesktop) {
      return invoke<AuthResponse>("unlock_session", {
        token: token || "",
        password,
      });
    }

    if (!devStore.admin || devStore.admin.password !== password) {
      return Promise.resolve({ success: false, message: "Invalid password", token: null });
    }

    devStore.locked = false;
    devStore.loggedIn = true;
    persistDevStore();

    return Promise.resolve({
      success: true,
      message: "Unlocked",
      token: "preview",
    });
  },

  forgotPassword(username: string, recoveryCode: string, newPassword: string): Promise<AuthResponse> {
    if (isDesktop) {
      return invoke<AuthResponse>("forgot_password", {
        request: { username, recovery_code: recoveryCode, new_password: newPassword },
      });
    }
    const admin = devStore.admin;
    if (!admin || admin.username !== username.trim() || admin.recoveryCode !== recoveryCode.replace(/\s+/g, "").toUpperCase()) {
      return Promise.resolve({ success: false, message: "Recovery details are incorrect", token: null });
    }
    if (newPassword.length < 8) {
      return Promise.resolve({ success: false, message: "New password must be at least 8 characters", token: null });
    }
    if (!devStore.admin) return Promise.resolve({ success: false, message: "Admin account is unavailable", token: null });
    devStore.admin.password = newPassword;
    devStore.loggedIn = false;
    devStore.locked = false;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Password reset successfully", token: null });
  },

  changePassword(token: string, currentPassword: string, newPassword: string): Promise<AuthResponse> {
    if (isDesktop) {
      return resolveDesktopSessionToken(token)
        .then((activeToken) => invoke<AuthResponse>("change_password", { token: activeToken, currentPassword, newPassword }));
    }
    if (!devStore.admin || !devStore.loggedIn) return Promise.resolve({ success: false, message: "Please log in first", token: null });
    if (devStore.admin.password !== currentPassword) return Promise.resolve({ success: false, message: "Current password is incorrect", token: null });
    if (newPassword.length < 8) return Promise.resolve({ success: false, message: "New password must be at least 8 characters", token: null });
    const recoveryCode = createRecoveryCode();
    devStore.admin.password = newPassword;
    devStore.admin.recoveryCode = recoveryCode;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Password changed and recovery key regenerated", token, recovery_code: recoveryCode });
  },


  updateUsername(token: string, currentPassword: string, newUsername: string): Promise<AuthResponse> {
    if (isDesktop) {
      return resolveDesktopSessionToken(token)
        .then((activeToken) => invoke<AuthResponse>("update_admin_username", { token: activeToken, currentPassword, newUsername }));
    }
    if (!devStore.admin || !devStore.loggedIn) return Promise.resolve({ success: false, message: "Please log in first", token: null });
    const username = newUsername.trim();
    if (username.length < 3 || username.length > 50) return Promise.resolve({ success: false, message: "Username must be between 3 and 50 characters", token: null });
    if (devStore.admin.password !== currentPassword) return Promise.resolve({ success: false, message: "Current password is incorrect", token: null });
    devStore.admin.username = username;
    persistDevStore();
    return Promise.resolve({ success: true, message: "Username updated successfully", token });
  },

  getAdminProfile(): Promise<{ id: number; username: string; created_at: string; updated_at: string }> {
    return isDesktop
      ? resolveDesktopSessionToken().then((token) => invoke("get_admin_profile", { token }))
      : Promise.resolve({ id: 1, username: devStore.admin?.username ?? "Admin", created_at: new Date().toISOString(), updated_at: new Date().toISOString() });
  },

  getSecuritySettings(): Promise<SecuritySettings> {
    return isDesktop
      ? invoke<SecuritySettings>("get_security_settings")
      : Promise.resolve({ auto_lock_minutes: 15, session_timeout_minutes: 480 });
  },

  getAuditLogs(token: string, search = "", limit = 100): Promise<import("@/types/auth").AuditEntry[]> {
    return isDesktop ? resolveDesktopSessionToken(token).then((activeToken) => invoke("get_audit_logs", { token: activeToken, search, limit })) : Promise.resolve([]);
  },

  clearAuditLogs(token: string): Promise<number> {
    return isDesktop
      ? resolveDesktopSessionToken(token).then((activeToken) => invoke<number>("clear_audit_logs", { token: activeToken }))
      : Promise.resolve(0);
  },

  getSystemInfo(token: string): Promise<import("@/types/auth").SystemInfo> {
    return isDesktop ? resolveDesktopSessionToken(token).then((activeToken) => invoke("get_system_info", { token: activeToken })) : Promise.resolve({ app_version: "1.0.0", database_version: "0", database_path: "Preview", database_size_bytes: 0, employee_count: 0, department_count: 0, backup_count: 0, paired_device_count: 0, audit_count: 0, platform: "browser", architecture: "unknown" });
  },

  updateSecuritySettings(token: string, autoLockMinutes: number, sessionTimeoutMinutes: number): Promise<AuthResponse> {
    if (isDesktop) {
      return resolveDesktopSessionToken(token)
        .then((activeToken) => invoke<AuthResponse>("update_security_settings", { token: activeToken, autoLockMinutes, sessionTimeoutMinutes }));
    }
    return Promise.resolve({ success: true, message: "Preview settings updated", token: null });
  },
};
