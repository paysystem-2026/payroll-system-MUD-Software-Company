import { ReactNode } from "react";
import { useAuthContext } from "@/stores/authContext";
import { FirstSetupPage } from "@/modules/auth/FirstSetupPage";
import { LoginPage } from "@/modules/auth/LoginPage";
import { LockScreenPage } from "@/modules/auth/LockScreenPage";

export function AuthGate({ children }: { children: ReactNode }) {
  const auth = useAuthContext();

  if (auth.loading || !auth.status) {
    return (
      <div className="auth-shell">
        <div className="auth-orb auth-orb-one" />
        <div className="auth-orb auth-orb-two" />
        <div className="auth-loading-card">
          <div className="flex items-center gap-3">
            <div className="h-9 w-9 animate-pulse rounded-xl bg-[#4a8b3f]/20" />
            <div>
              <p className="text-sm font-semibold text-white">Payroll System</p>
              <p className="text-xs text-[#777]">Preparing secure workspace…</p>
            </div>
          </div>
        </div>
      </div>
    );
  }

  if (!auth.status.is_setup) {
    return <FirstSetupPage onSetup={auth.setup} onComplete={auth.completeSetup} />;
  }

  // get_auth_status is intentionally session-agnostic on the Rust side.
  // The renderer must also possess the actual session token before entering
  // protected modules; otherwise a stale DB session could open the workspace
  // with no token for commands (for example Backup & Restore).
  if (!auth.status.is_authenticated || !auth.token) {
    return <LoginPage onLogin={auth.login} onForgotPassword={auth.forgotPassword} />;
  }

  if (auth.status.is_locked) {
    return <LockScreenPage username={auth.status.admin_username} onUnlock={auth.unlock} />;
  }

  return <>{children}</>;
}
