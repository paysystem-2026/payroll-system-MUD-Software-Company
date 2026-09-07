import { useState, useEffect, useCallback, useRef } from "react";
import { authService } from "@/services/auth";
import type { AuthStatus } from "@/types/auth";

const TOKEN_KEY = "payroll_session_token";

export function useAuth() {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [token, setToken] = useState<string | null>(() => sessionStorage.getItem(TOKEN_KEY));
  const lockTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await authService.getStatus();
      if (next.token) { sessionStorage.setItem(TOKEN_KEY, next.token); setToken(next.token); }
      else if (!next.is_authenticated) { sessionStorage.removeItem(TOKEN_KEY); setToken(null); }
      setStatus(next);
      return next;
    } catch {
      const fallback: AuthStatus = { is_setup: false, is_authenticated: false, is_locked: false, admin_username: null, token: null };
      setStatus(fallback);
      return fallback;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const storeToken = useCallback((next: string | null) => {
    if (next) sessionStorage.setItem(TOKEN_KEY, next);
    else sessionStorage.removeItem(TOKEN_KEY);
    setToken(next);
  }, []);

  const setup = useCallback(async (username: string, password: string) => {
    const res = await authService.setup(username, password);
    if (res.success && res.token) storeToken(res.token);
    return res;
  }, [storeToken]);

  const completeSetup = useCallback(async () => {
    await refresh();
  }, [refresh]);

  const login = useCallback(async (username: string, password: string) => {
    const res = await authService.login(username, password);
    if (res.success && res.token) {
      storeToken(res.token);
      await refresh();
    }
    return res;
  }, [refresh, storeToken]);

  const logout = useCallback(async () => {
    const activeToken = token;
    // Update the UI first so a background status refresh can never immediately
    // put the user back into the authenticated workspace. Persisted session
    // cleanup is still awaited when a desktop token exists.
    storeToken(null);
    setStatus((current) => ({
      is_setup: current?.is_setup ?? true,
      is_authenticated: false,
      is_locked: false,
      admin_username: null,
    }));
    if (lockTimerRef.current) {
      clearTimeout(lockTimerRef.current);
      lockTimerRef.current = null;
    }
    if (activeToken) {
      try { await authService.logout(activeToken); } catch { /* UI is already logged out. */ }
    }
  }, [token, storeToken]);

  const lock = useCallback(async () => {
    const activeToken = token;
    if (!activeToken) return;
    try {
      const res = await authService.lock(activeToken);
      if (res.success) {
        // Do not refresh here: get_auth_status is intentionally session-agnostic
        // and a racing refresh could re-open the workspace before the DB update
        // is observed. AuthGate must react immediately to the local lock state.
        setStatus((current) => ({
          is_setup: current?.is_setup ?? true,
          is_authenticated: true,
          is_locked: true,
          admin_username: current?.admin_username ?? null,
        }));
        if (lockTimerRef.current) {
          clearTimeout(lockTimerRef.current);
          lockTimerRef.current = null;
        }
      }
    } catch {
      // Keep the workspace available if the lock command itself fails.
    }
  }, [token]);

  const unlock = useCallback(async (password: string) => {
    // A locked session must remain unlockable even if the renderer lost
    // sessionStorage during a reload/restart. The Rust backend can recover
    // the newest locked session securely after verifying the Admin password.
    const activeToken = token ?? "";
    const res = await authService.unlock(activeToken, password);

    if (res.success && res.token) {
      storeToken(res.token);
      await refresh();
    }

    return res;
  }, [token, refresh, storeToken]);

  const forgotPassword = useCallback((username: string, recoveryCode: string, newPassword: string) =>
    authService.forgotPassword(username, recoveryCode, newPassword), []);

  const resetLockTimer = useCallback(async () => {
    if (!status?.is_authenticated || status.is_locked) return;
    if (lockTimerRef.current) clearTimeout(lockTimerRef.current);
    try {
      const settings = await authService.getSecuritySettings();
      if (settings.auto_lock_minutes <= 0) {
        lockTimerRef.current = null;
        return;
      }
      lockTimerRef.current = setTimeout(() => void lock(), settings.auto_lock_minutes * 60 * 1000);
    } catch {
      lockTimerRef.current = setTimeout(() => void lock(), 15 * 60 * 1000);
    }
  }, [status, lock]);

  useEffect(() => {
    if (status?.is_authenticated && !status.is_locked) void resetLockTimer();
    return () => {
      if (lockTimerRef.current) clearTimeout(lockTimerRef.current);
    };
  }, [status, resetLockTimer]);

  useEffect(() => {
    const onActivity = () => {
      if (status?.is_authenticated && !status.is_locked) void resetLockTimer();
    };
    window.addEventListener("mousemove", onActivity);
    window.addEventListener("keydown", onActivity);
    return () => {
      window.removeEventListener("mousemove", onActivity);
      window.removeEventListener("keydown", onActivity);
    };
  }, [status, resetLockTimer]);

  return { status, loading, token, setup, completeSetup, login, logout, lock, unlock, forgotPassword, refresh };
}
