import { useState, FormEvent } from "react";
import type { ReactNode } from "react";
import { ArrowLeft, CheckCircle2, KeyRound, LockKeyhole, UserRound } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { AuthField, AuthShell } from "@/modules/auth/AuthShell";
import type { AuthResponse } from "@/types/auth";

interface Props {
  onReset: (username: string, recoveryCode: string, newPassword: string) => Promise<AuthResponse>;
  onBack: () => void;
}

export function ForgotPasswordPage({ onReset, onBack }: Props) {
  const [username, setUsername] = useState("");
  const [recoveryCode, setRecoveryCode] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [done, setDone] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    if (password.length < 8) return setError("New password must be at least 8 characters.");
    if (password !== confirm) return setError("Passwords do not match.");
    setLoading(true);
    try {
      const res = await onReset(username, recoveryCode, password);
      if (!res.success) setError(res.message);
      else setDone(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not reset the password.");
    } finally {
      setLoading(false);
    }
  };

  if (done) {
    return (
      <AuthShell eyebrow="Password updated" title="You can sign in again" description="Your local Admin password has been changed. Return to the sign-in screen to continue." compact>
        <div className="auth-recovery">
          <div className="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-[rgba(74,139,63,0.14)] text-[#67b85a] ring-1 ring-[rgba(103,184,90,0.22)]">
            <CheckCircle2 className="h-6 w-6" />
          </div>
          <p className="mt-4 text-center text-sm font-semibold text-white">Recovery completed successfully.</p>
          <p className="mt-1 text-center text-[11px] leading-5 text-[#737971]">Use the new password the next time you sign in.</p>
        </div>
        <Button type="button" className="mt-5 w-full py-3" onClick={onBack}><ArrowLeft className="h-4 w-4" /> Return to Sign In</Button>
      </AuthShell>
    );
  }

  return (
    <AuthShell eyebrow="Offline recovery" title="Reset Admin password" description="Use the recovery code created during first-time setup to restore access without internet." compact>
      <button type="button" onClick={onBack} className="auth-link mt-1 inline-flex items-center gap-2"><ArrowLeft className="h-4 w-4" /> Back to Sign In</button>
      <form onSubmit={submit} className="auth-form">
        <AuthField label="Username" icon={<UserRound className="h-4 w-4" />}>
          <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Admin username" autoFocus className="auth-input" />
        </AuthField>
        <AuthField label="Recovery code" hint="Enter the code exactly as shown during setup" icon={<KeyRound className="h-4 w-4" />}>
          <input value={recoveryCode} onChange={(e) => setRecoveryCode(e.target.value.toUpperCase())} placeholder="Recovery code" className="auth-input font-mono tracking-[0.08em]" />
        </AuthField>
        <AuthField label="New password" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Minimum 8 characters" className="auth-input" />
        </AuthField>
        <AuthField label="Confirm new password" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} placeholder="Re-enter the new password" className="auth-input" />
        </AuthField>
        {error && <div className="auth-alert">{error}</div>}
        <Button type="submit" disabled={loading} className="mt-5 w-full py-3">{loading ? "Resetting password…" : "Reset Password"}</Button>
      </form>
    </AuthShell>
  );
}

export function RecoveryHint({ children }: { children: ReactNode }) {
  return <div className="auth-footer">{children}</div>;
}
