import gorillaDoctorsLogo from "@/assets/logo/gorilla-doctors.jpeg";
import type { ReactNode } from "react";
import { ShieldCheck, Sparkles } from "lucide-react";
import { WindowControls } from "@/components/common/WindowControls";

interface AuthShellProps {
  eyebrow: string;
  title: string;
  description: string;
  children: ReactNode;
  footer?: ReactNode;
  compact?: boolean;
}

export function AuthShell({ eyebrow, title, description, children, footer, compact = false }: AuthShellProps) {
  return (
    <div className="auth-shell">
      <div className="auth-titlebar" data-tauri-drag-region>
        <div className="auth-titlebar-brand" data-tauri-drag-region>Payroll System</div>
        <WindowControls />
      </div>
      <div className="auth-grid" aria-hidden="true" />
      <div className="auth-glow auth-glow-a" aria-hidden="true" />
      <div className="auth-glow auth-glow-b" aria-hidden="true" />

      <div className={`auth-frame ${compact ? "auth-frame-compact" : ""}`}>
        <aside className="auth-brand">
          <div className="auth-brand-mark overflow-hidden bg-white/95 p-1">
            <img
              src={gorillaDoctorsLogo}
              alt="Gorilla Doctors"
              className="h-full w-full object-contain rounded-[inherit]"
            />
          </div>
          <div>
            <p className="auth-brand-kicker">Payroll System V1</p>
            <p className="auth-brand-title">Gorilla Doctors</p>
          </div>

          <div className="auth-brand-rule" />
          <p className="auth-brand-copy">
            A focused local payroll workspace built for speed, clarity and secure everyday operations.
          </p>

          <div className="auth-brand-points">
            <div className="auth-brand-point"><span>01</span><div><strong>Private by design</strong><p>Local-first data and one secure Admin account.</p></div></div>
            <div className="auth-brand-point"><span>02</span><div><strong>Payroll-ready</strong><p>Structured records, configurable rules and history.</p></div></div>
            <div className="auth-brand-point"><span>03</span><div><strong>Clean & fast</strong><p>Designed for a calm, practical desktop workflow.</p></div></div>
          </div>

          <div className="auth-brand-footer">
            <ShieldCheck className="h-4 w-4" />
            <span>Offline-first • Single Admin • Protected workspace</span>
          </div>
        </aside>

        <main className="auth-panel">
          <div className="auth-panel-topline">
            <span>{eyebrow}</span>
            <span className="auth-panel-status"><Sparkles className="h-3.5 w-3.5" /> Secure access</span>
          </div>

          <div className="auth-heading">
            <h1>{title}</h1>
            <p>{description}</p>
          </div>

          {children}
          {footer && <div className="auth-footer">{footer}</div>}
        </main>
      </div>
      <div className="auth-company-footer">Powered by <strong>MUD software company</strong><span>•</span><span>Support: 0780937633</span></div>
    </div>
  );
}

export function AuthField({ label, icon, children, hint }: { label: string; icon: ReactNode; children: ReactNode; hint?: string }) {
  return (
    <label className="auth-field">
      <span className="auth-field-label">{label}</span>
      <span className="auth-field-control">
        <span className="auth-field-icon">{icon}</span>
        {children}
      </span>
      {hint && <span className="auth-field-hint">{hint}</span>}
    </label>
  );
}
