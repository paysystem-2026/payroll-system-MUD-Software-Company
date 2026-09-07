import { useState, FormEvent } from "react";
import { ArrowRight, Check, KeyRound, LockKeyhole, ShieldCheck, UserRound } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { AuthField, AuthShell } from "@/modules/auth/AuthShell";
import type { AuthResponse } from "@/types/auth";

interface Props {
  onSetup: (username: string, password: string) => Promise<AuthResponse>;
  onComplete: () => Promise<void>;
}

export function FirstSetupPage({ onSetup, onComplete }: Props) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [recoveryCode, setRecoveryCode] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    if (username.trim().length < 3) return setError("Username must be at least 3 characters.");
    if (password.length < 8) return setError("Password must be at least 8 characters.");
    if (password !== confirm) return setError("Passwords do not match.");
    setLoading(true);
    try {
      const res = await onSetup(username, password);
      if (!res.success) setError(res.message);
      else setRecoveryCode(res.recovery_code ?? "");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not create the Admin account.");
    } finally {
      setLoading(false);
    }
  };

  if (recoveryCode) {
    return (
      <AuthShell
        eyebrow="Setup complete"
        title="Secure your recovery code"
        description="This is the offline recovery method for the single Admin account. Save it before continuing."
        compact
      >
        <div className="auth-recovery">
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-xl bg-[rgba(74,139,63,0.14)] text-[#67b85a] ring-1 ring-[rgba(103,184,90,0.22)]">
              <KeyRound className="h-5 w-5" />
            </div>
            <div>
              <p className="text-xs font-semibold text-white">Recovery code</p>
              <p className="mt-0.5 text-[10px] text-[#6f756d]">Shown once during setup</p>
            </div>
          </div>
          <div className="auth-recovery-code">{recoveryCode}</div>
          <div className="mt-3 flex items-start gap-2 text-[11px] leading-5 text-[#7d8479]">
            <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-[#67b85a]" />
            Keep this code private. You will need it to recover the Admin password offline.
          </div>
        </div>
        <Button type="button" className="mt-5 w-full py-3" onClick={() => void onComplete()}>
          Continue to Payroll System <ArrowRight className="h-4 w-4" />
        </Button>
      </AuthShell>
    );
  }

  return (
    <AuthShell
      eyebrow="First launch"
      title="Create the Admin account"
      description="Set up the one protected account that controls this local payroll workspace."
      footer={
        <div className="flex items-start gap-2">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-[#67b85a]" />
          <span>Password is hashed with Argon2id. The system stays local and offline-first.</span>
        </div>
      }
    >
      <form onSubmit={handleSubmit} className="auth-form">
        <AuthField label="Username" icon={<UserRound className="h-4 w-4" />}>
          <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Choose an Admin username" autoFocus className="auth-input" />
        </AuthField>
        <AuthField label="Password" hint="Minimum 8 characters" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Create a secure password" className="auth-input" />
        </AuthField>
        <AuthField label="Confirm password" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} placeholder="Enter the password again" className="auth-input" />
        </AuthField>

        {error && <div className="auth-alert">{error}</div>}

        <Button type="submit" disabled={loading} className="mt-5 w-full py-3">
          {loading ? "Creating secure account…" : "Create Admin Account"}
          {!loading && <Check className="h-4 w-4" />}
        </Button>
      </form>
    </AuthShell>
  );
}
