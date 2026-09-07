import { useState, FormEvent } from "react";
import { LockKeyhole, ShieldCheck, UserRound } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { AuthField, AuthShell } from "@/modules/auth/AuthShell";
import type { AuthResponse } from "@/types/auth";

interface Props {
  username: string | null;
  onUnlock: (password: string) => Promise<AuthResponse>;
}

export function LockScreenPage({ username, onUnlock }: Props) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const res = await onUnlock(password);
      if (!res.success) {
        setError(res.message);
        setPassword("");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to unlock.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <AuthShell
      eyebrow="Session protected"
      title="Payroll System is locked"
      description="Unlock with the current Admin password to continue working. Your data remains safely on this device."
      compact
      footer={<div className="flex items-center gap-2"><ShieldCheck className="h-4 w-4 text-[#67b85a]" /> Protected local session</div>}
    >
      {username && (
        <div className="mt-6 inline-flex items-center gap-2 rounded-full border border-[#2d322c] bg-[#0e100e] px-3 py-1.5 text-[11px] text-[#b8beb6]">
          <UserRound className="h-3.5 w-3.5 text-[#67b85a]" /> {username}
        </div>
      )}
      <form onSubmit={submit} className="auth-form">
        <AuthField label="Password" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Admin password" autoFocus className="auth-input" />
        </AuthField>
        {error && <div className="auth-alert">{error}</div>}
        <Button type="submit" disabled={loading} className="mt-5 w-full py-3">{loading ? "Unlocking…" : "Unlock"}</Button>
      </form>
    </AuthShell>
  );
}
