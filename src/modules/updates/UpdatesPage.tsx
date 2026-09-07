import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  AlertCircle, CheckCircle2, CircleDashed, Download, HardDriveDownload, History,
  Info, RefreshCw, RotateCcw, ShieldCheck, Smartphone, XCircle,
} from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { useAuthContext } from "@/stores/authContext";
import { updatesService } from "@/services/updates";
import type { UpdateStatusResponse } from "@/types/updates";

const formatBytes = (value: number | null | undefined) => {
  if (value == null || !Number.isFinite(value)) return "—";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
};

const formatDate = (value: string | null) => {
  if (!value) return "Never";
  const date = new Date(value.includes("T") ? value : `${value.replace(" ", "T")}Z`);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString([], { dateStyle: "medium", timeStyle: "short" });
};

const statusLabel = (state: string) => ({
  idle: "Ready",
  checking: "Checking…",
  up_to_date: "Up to date",
  available: "Update available",
  downloading: "Downloading…",
  downloaded: "Ready to install",
  installing: "Installing…",
  installed_pending_restart: "Restart required",
  rejected: "Update rejected",
  unavailable: "Service unavailable",
  error: "Update error",
}[state] ?? state.replace(/_/g, " "));

export function UpdatesPage() {
  const { token } = useAuthContext();
  const [data, setData] = useState<UpdateStatusResponse | null>(null);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    try {
      setData(await updatesService.getStatus());
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to load update status.");
    }
  }, []);

  useEffect(() => {
    void load();
    const unlisten = listen("update://status", () => void load());
    return () => { void unlisten.then((fn) => fn()); };
  }, [load]);

  const run = async (action: string, fn: () => Promise<{ success: boolean; message: string }>) => {
    setBusy(action); setError(""); setMessage("");
    try {
      const result = await fn();
      if (result.success) setMessage(result.message); else setError(result.message);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Update operation failed.");
      await load();
    } finally { setBusy(""); }
  };

  if (!data) {
    return <div className="p-6"><PageHeader title="Updates" description="Secure application updates, verification and installation." /><Card className="p-10 text-center"><RefreshCw className="mx-auto h-6 w-6 animate-spin text-[#4a8b3f]" /><p className="mt-3 text-xs text-[#777]">Loading update service…</p></Card></div>;
  }

  const { status, history } = data;
  const update = status.update;
  const progress = status.progress.percent == null ? 0 : Math.max(0, Math.min(100, status.progress.percent));
  const canDownload = status.state === "available" && Boolean(update?.compatible && update?.signature_present) && !busy;
  const canInstall = status.state === "downloaded";
  const canRestart = status.state === "installed_pending_restart";

  return (
    <div className="min-w-0 pb-8">
      <PageHeader title="Updates" description="Check, verify and safely install signed Payroll System releases without blocking the workspace." />

      {(message || error) && (
        <div className={`mb-5 flex items-start gap-2 rounded-xl border px-4 py-3 text-xs ${error ? "border-[#59462b] bg-[#1b1710] text-[#e7d0a4]" : "border-[#294126] bg-[#142012] text-[#b9dfb2]"}`}>
          {error ? <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" /> : <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0" />}
          <span>{error || message}</span>
        </div>
      )}

      <div className="grid gap-4 xl:grid-cols-[1.5fr_1fr]">
        <Card className="overflow-hidden">
          <div className="border-b border-[#292929] px-5 py-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <p className="text-[10px] font-semibold uppercase tracking-[0.14em] text-[#5fa453]">Current release</p>
                <h2 className="mt-1 text-xl font-semibold text-white">v{status.current_version}</h2>
                <p className="mt-1 text-[11px] text-[#707070]">{status.platform} · {status.architecture}</p>
              </div>
              <span className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[10px] font-semibold ${status.state === "error" || status.state === "rejected" ? "border-[#59462b] bg-[#1b1710] text-[#e7d0a4]" : status.state === "available" || status.state === "downloaded" ? "border-[#294126] bg-[#142012] text-[#a8d9a0]" : "border-[#303030] bg-[#202020] text-[#bdbdbd]"}`}>
                {status.state === "error" || status.state === "rejected" ? <XCircle className="h-3 w-3" /> : status.state === "available" || status.state === "downloaded" ? <CheckCircle2 className="h-3 w-3" /> : <CircleDashed className="h-3 w-3" />}
                {statusLabel(status.state)}
              </span>
            </div>
          </div>

          <div className="space-y-5 p-5">
            <div className="rounded-xl border border-[#292929] bg-[#101010] p-4">
              <div className="flex items-start gap-3">
                <ShieldCheck className="mt-0.5 h-5 w-5 shrink-0 text-[#4a8b3f]" />
                <div>
                  <p className="text-sm font-semibold text-white">{status.message}</p>
                  <p className="mt-1 text-[11px] leading-5 text-[#777]">Updates are verified before installation. Database and payroll data stay outside the application bundle.</p>
                </div>
              </div>
            </div>

            {update && (
              <div className="space-y-4 rounded-xl border border-[#294126] bg-[#142012] p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-[10px] uppercase tracking-wider text-[#7fa978]">Available release</p>
                    <p className="mt-1 text-lg font-semibold text-white">v{update.version}</p>
                  </div>
                  <div className="text-right text-[10px] text-[#8fb489]">
                    <p>{formatBytes(update.size_bytes)}</p>
                    <p className="mt-1">{update.target}</p>
                  </div>
                </div>
                <div className="grid gap-2 text-[11px] sm:grid-cols-4">
                  <div className="rounded-lg border border-[#294126] bg-[#101810] p-3"><span className="text-[#6f876b]">Compatibility</span><p className="mt-1 font-medium text-white">{update.compatible ? "Supported" : "Rejected"}</p></div>
                  <div className="rounded-lg border border-[#294126] bg-[#101810] p-3"><span className="text-[#6f876b]">Signature</span><p className="mt-1 font-medium text-white">{update.signature_present ? "Verified" : "Missing"}</p></div>
                  <div className="rounded-lg border border-[#294126] bg-[#101810] p-3"><span className="text-[#6f876b]">SHA-256</span><p className="mt-1 font-medium text-white">{status.checksum_present ? "Manifest supplied" : "Signature integrity"}</p></div>
                  <div className="rounded-lg border border-[#294126] bg-[#101810] p-3"><span className="text-[#6f876b]">Release date</span><p className="mt-1 font-medium text-white">{formatDate(update.date)}</p></div>
                </div>
                <div>
                  <p className="text-[10px] font-semibold uppercase tracking-wider text-[#6f876b]">Release notes</p>
                  <p className="mt-2 whitespace-pre-wrap text-[11px] leading-5 text-[#d0d0d0]">{update.notes || "No release notes were supplied by the update server."}</p>
                </div>
              </div>
            )}

            {(status.state === "downloading" || status.state === "downloaded") && (
              <div>
                <div className="mb-2 flex justify-between text-[10px] text-[#777]"><span>{status.state === "downloaded" ? "Download verified" : "Downloading"}</span><span>{progress.toFixed(0)}% · {formatBytes(status.progress.downloaded_bytes)}{status.progress.total_bytes != null ? ` / ${formatBytes(status.progress.total_bytes)}` : ""}</span></div>
                <div className="h-2 overflow-hidden rounded-full bg-[#242424]"><div className="h-full rounded-full bg-[#4a8b3f] transition-all" style={{ width: `${status.state === "downloaded" ? 100 : progress}%` }} /></div>
              </div>
            )}

            <div className="flex flex-wrap gap-2">
              <Button variant="secondary" onClick={() => void run("check", updatesService.check)} disabled={Boolean(busy)}><RefreshCw className={`h-4 w-4 ${busy === "check" ? "animate-spin" : ""}`} />{busy === "check" ? "Checking…" : "Check for Updates"}</Button>
              {canDownload && <Button onClick={() => void run("download", updatesService.download)} disabled={Boolean(busy)}><Download className="h-4 w-4" />{busy === "download" ? "Downloading…" : "Download Update"}</Button>}
              {status.state === "downloading" && <Button variant="secondary" onClick={() => void run("cancel", updatesService.cancelDownload)} disabled={Boolean(busy)}><XCircle className="h-4 w-4" />Cancel</Button>}
              {canInstall && token && <Button onClick={() => void run("install", () => updatesService.install(token))} disabled={Boolean(busy)}><HardDriveDownload className="h-4 w-4" />{busy === "install" ? "Installing…" : "Install Update"}</Button>}
              {canRestart && <Button onClick={() => void run("restart", updatesService.restart)} disabled={Boolean(busy)}><RotateCcw className="h-4 w-4" />Restart & Update</Button>}
            </div>
          </div>
        </Card>

        <div className="space-y-4">
          <Card className="p-5">
            <div className="flex items-center gap-2"><Info className="h-4 w-4 text-[#4a8b3f]" /><h2 className="text-sm font-semibold text-white">Update service</h2></div>
            <div className="mt-4 space-y-3 text-[11px]">
              <div className="flex justify-between gap-3"><span className="text-[#707070]">Service</span><span className={status.service_configured ? "text-[#a8d9a0]" : "text-[#c6ad7e]"}>{status.service_configured ? status.source : "Not configured"}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[#707070]">Last checked</span><span className="text-white">{formatDate(status.last_checked)}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[#707070]">Platform</span><span className="text-white">{status.platform}</span></div>
              <div className="flex justify-between gap-3"><span className="text-[#707070]">Architecture</span><span className="text-white">{status.architecture}</span></div>
            </div>
            {!status.service_configured && <div className="mt-4 rounded-lg border border-[#59462b] bg-[#1b1710] p-3 text-[10px] leading-5 text-[#d0bc91]">The application is still fully usable. A production build must provide the signed updater endpoint and public signing key; no fake update is shown when those are absent.</div>}
          </Card>

          <Card className="p-5">
            <div className="flex items-center gap-2"><Smartphone className="h-4 w-4 text-[#4a8b3f]" /><h2 className="text-sm font-semibold text-white">Safety & recovery</h2></div>
            <ul className="mt-3 space-y-2 text-[10px] leading-5 text-[#777]">
              <li>• Signed updater packages only.</li>
              <li>• Platform and architecture are checked before installation.</li>
              <li>• A protected database backup is created before installation.</li>
              <li>• {status.recovery_available ? `Recovery point ready · ${formatDate(status.recovery_created_at)}` : "No pre-update recovery point yet."}</li>
              <li>• The Tauri updater installs the signed application bundle; the updater does not replace SQLite data.</li>
              <li>• Automatic binary rollback is not faked; recovery uses the protected database snapshot and a separately published signed release if rollback is ever enabled.</li>
            </ul>
          </Card>
        </div>
      </div>

      <Card className="mt-4 overflow-hidden">
        <div className="flex items-center gap-2 border-b border-[#292929] px-5 py-4"><History className="h-4 w-4 text-[#4a8b3f]" /><div><h2 className="text-sm font-semibold text-white">Update history</h2><p className="mt-0.5 text-[10px] text-[#707070]">Checks, availability, downloads, validation and installation events.</p></div></div>
        {history.length === 0 ? <div className="p-10 text-center text-[11px] text-[#666]">No update activity recorded yet.</div> : <div className="divide-y divide-[#242424]">{history.map((entry) => <div key={entry.id} className="grid gap-2 px-5 py-3 md:grid-cols-[150px_120px_1fr_150px]"><div><p className="text-[10px] font-semibold uppercase tracking-wider text-[#a0a0a0]">{entry.event_type || entry.status}</p><p className="mt-1 text-[9px] text-[#606060]">{formatDate(entry.created_at)}</p></div><div className="text-[10px] text-white">{entry.version_from ? `v${entry.version_from} → ` : ""}v{entry.version_to}</div><div className="break-words text-[10px] text-[#777]">{entry.details || "—"}</div><div className="md:text-right"><span className="inline-flex rounded-full border border-[#303030] bg-[#202020] px-2 py-1 text-[9px] text-[#aaa]">{entry.status}</span></div></div>)}</div>}
      </Card>
    </div>
  );
}
