import { useState, FormEvent } from "react";
import { ArrowRight, LockKeyhole, ShieldCheck, UserRound } from "lucide-react";
import { Button } from "@/components/ui/Button";
import type { AuthResponse } from "@/types/auth";
import { ForgotPasswordPage } from "@/modules/auth/ForgotPasswordPage";
import { AuthField, AuthShell } from "@/modules/auth/AuthShell";

interface Props {
  onLogin: (username: string, password: string) => Promise<AuthResponse>;
  onForgotPassword: (username: string, recoveryCode: string, newPassword: string) => Promise<AuthResponse>;
}

export function LoginPage({ onLogin, onForgotPassword }: Props) {
  const [forgot, setForgot] = useState(false);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const res = await onLogin(username, password);
      if (!res.success) setError(res.message);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to sign in.");
    } finally {
      setLoading(false);
    }
  };

  if (forgot) return <ForgotPasswordPage onReset={onForgotPassword} onBack={() => setForgot(false)} />;

  return (
    <AuthShell
      eyebrow="Welcome back"
      title="Sign in to Payroll System"
      description="Continue to the protected local payroll workspace using the Admin account created at setup."
      footer={
        <div className="flex items-start gap-2">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-[#67b85a]" />
          <span>Single-user local authentication. Passwords are never stored as plain text.</span>
        </div>
      }
    >
      <form onSubmit={submit} className="auth-form">
        <AuthField label="Username" icon={<UserRound className="h-4 w-4" />}>
          <input value={username} onChange={(e) => setUsername(e.target.value)} placeholder="Admin username" autoFocus className="auth-input" />
        </AuthField>
        <AuthField label="Password" icon={<LockKeyhole className="h-4 w-4" />}>
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Admin password" className="auth-input" />
        </AuthField>

        {error && <div className="auth-alert">{error}</div>}

        <Button type="submit" disabled={loading} className="mt-5 w-full py-3">
          {loading ? "Signing in…" : "Sign In"}
          {!loading && <ArrowRight className="h-4 w-4" />}
        </Button>
      </form>

      <div className="mt-4 flex items-center justify-between gap-4">
        <span className="text-[10px] text-[#686e67]">Protected local session</span>
        <button type="button" onClick={() => setForgot(true)} className="auth-link">Forgot password?</button>
      </div>
    </AuthShell>
  );
}
