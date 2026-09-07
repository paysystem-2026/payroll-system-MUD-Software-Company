import { useEffect, useMemo, useState } from "react";
import type { LucideIcon } from "lucide-react";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, Clock3, FolderOpen, HardDriveDownload, History, Loader2, RefreshCw, RotateCcw, Save, Settings2, ShieldCheck, Trash2, Upload } from "lucide-react";
import { useAuthContext } from "@/stores/authContext";
import { backupService } from "@/services/backup";
import type { BackupRecord, BackupSettings } from "@/types/backup";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";

const formatBytes = (bytes: number) => bytes < 1024 * 1024 ? `${Math.max(1, Math.round(bytes / 1024))} KB` : `${(bytes / 1024 / 1024).toFixed(1)} MB`;
const typeLabel = (type: string) => type === "pre_restore" || type === "pre_import" ? "Safety" : type === "automatic" ? "Automatic" : type === "lan_received" ? "LAN Received" : "Manual";
const isDesktop = () => typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const errorMessage = (e: unknown, fallback: string) => {
  if (typeof e === "string" && e.trim()) return e;
  if (e instanceof Error && e.message) return e.message;
  if (e && typeof e === "object" && "message" in e) {
    const message = (e as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
};

const withTimeout = async <T,>(promise: Promise<T>, ms = 60000): Promise<T> => {
  let timer: number | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = window.setTimeout(() => reject(new Error("Backup operation timed out. Please try again.")), ms);
      }),
    ]);
  } finally {
    if (timer !== undefined) window.clearTimeout(timer);
  }
};

export function BackupPage() {
  const auth = useAuthContext();
  const token = auth.token;
  const [settings, setSettings] = useState<BackupSettings | null>(null);
  const [backups, setBackups] = useState<BackupRecord[]>([]);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const load = async () => {
    if (!token) return;
    setError("");
    try {
      const [s, b] = await Promise.all([backupService.getSettings(token), backupService.list(token)]);
      setSettings({ ...s });
      setBackups(b);
    } catch (e) { setError(e instanceof Error ? e.message : "Unable to load backup information."); }
  };
  useEffect(() => { void load(); }, [token]);


  const lastBackup = useMemo(() => backups.find(b => b.status === "completed") ?? null, [backups]);
  const automaticCount = backups.filter(b => b.backup_type === "automatic" && b.status === "completed").length;

  const chooseBackupLocation = async () => {
    if (!settings || !isDesktop()) return;
    setBusy("location"); setMessage(""); setError("");
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: settings.location || undefined,
        title: "Select Backup Folder",
      });
      if (selected && !Array.isArray(selected)) setSettings(current => current ? { ...current, location: selected } : current);
    } catch (e) {
      setError(errorMessage(e, "Unable to select the backup folder."));
    } finally { setBusy(""); }
  };

  const saveSettings = async () => {
    if (!token || !settings) return;
    setBusy("settings"); setMessage(""); setError("");
    try {
      const res = await withTimeout(backupService.updateSettings(token, settings));
      if (res.success) { setMessage(res.message); void load(); } else setError(res.message);
    } catch (e) { setError(e instanceof Error ? e.message : "Unable to save backup settings."); }
    finally { setBusy(""); }
  };

  const create = async () => {
    if (!token) return;
    setBusy("create"); setMessage(""); setError("");
    try {
      const res = await withTimeout(backupService.create(token));
      if (res.success) { setMessage(res.message); void load(); } else setError(res.message);
    } catch (e) { setError(e instanceof Error ? e.message : "Unable to create backup."); }
    finally { setBusy(""); }
  };

  const verify = async (backup: BackupRecord) => {
    if (!token) return;
    setBusy(`verify-${backup.id}`); setMessage(""); setError("");
    try {
      const res = await withTimeout(backupService.verify(token, backup.id));
      if (res.success) setMessage(`Verified: ${backup.file_path}`); else setError(res.message);
    } catch (e) { setError(errorMessage(e, "Unable to verify backup.")); }
    finally { setBusy(""); }
  };

  const finishRestore = (res: { success: boolean; message: string }) => {
    if (!res.success) { setError(res.message); setBusy(""); return; }
    setMessage(res.message);
    window.setTimeout(() => window.location.reload(), 900);
  };

  const restore = async (backup: BackupRecord) => {
    if (!token) return;
    const ok = await confirm(`Restore the backup from ${new Date(backup.created_at).toLocaleString()}?\n\nA safety backup of the current database will be created first. The restored database will then be upgraded to the current application version.`, { title: "Restore Payroll Backup", kind: "warning" });
    if (!ok) return;
    setBusy(`restore-${backup.id}`); setMessage(""); setError("");
    try {
      const res = await withTimeout(backupService.restore(token, backup.id), 120000);
      finishRestore(res);
    } catch (e) { setError(errorMessage(e, "Unable to restore backup.")); setBusy(""); }
  };

  const restoreFromFile = async () => {
    if (!token) return;
    setMessage(""); setError(""); setBusy("restore-file");
    try {
      if (!isDesktop()) {
        setError("Use the Tauri desktop app to select a backup file for restore.");
        setBusy("");
        return;
      }
      const selected = await open({
        multiple: false,
        directory: false,
        defaultPath: settings?.location || undefined,
        filters: [{ name: "Payroll Backup", extensions: ["pbak", "sqlite", "db"] }],
        title: "Select Payroll System Backup",
      });
      if (!selected || Array.isArray(selected)) { setBusy(""); return; }
      const ok = await confirm(`Restore this backup?\n\n${selected}\n\nThe current database will be protected with a safety backup first. The selected backup will be upgraded to the current application version after restore.`, { title: "Restore Selected Backup", kind: "warning" });
      if (!ok) { setBusy(""); return; }
      const res = await withTimeout(backupService.restoreFile(token, selected), 120000);
      finishRestore(res);
    } catch (e) {
      setBusy("");
      setError(e instanceof Error ? e.message : "Unable to restore the selected backup file.");
    }
  };

  const remove = async (backup: BackupRecord) => {
    if (!token) return;
    const ok = await confirm(`Delete this backup permanently?\n\n${new Date(backup.created_at).toLocaleString()} · ${formatBytes(backup.file_size)}`, { title: "Delete Backup", kind: "warning" });
    if (!ok) return;
    setBusy(`delete-${backup.id}`); setMessage(""); setError("");
    try {
      const res = await withTimeout(backupService.remove(token, backup.id));
      if (res.success) {
        setBackups(current => current.filter(item => item.id !== backup.id));
        setMessage("Backup deleted successfully.");
      } else setError(res.message);
    } catch (e) { setError(errorMessage(e, "Unable to delete backup.")); }
    finally { setBusy(""); }
  };

  return (
    <div className="pb-8">
      <PageHeader title="Backup & Restore" description="Secure, local and version-aware protection for your payroll database." />

      {(message || error) && <div className={`mb-5 flex items-start gap-3 rounded-2xl border px-4 py-3 text-[12px] ${error ? "border-[#4b3028] bg-[#1b1210] text-[#d7a99b]" : "border-[#30472d] bg-[#111811] text-[#9ac891]"}`}><CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0" /> <span className="break-all">{message || error}</span></div>}

      <div className="grid gap-4 md:grid-cols-3">
        <StatusCard icon={ShieldCheck} label="Protection" value={settings?.enabled ? "Automatic On" : "Automatic Off"} detail={settings?.enabled ? "Rust backup engine is active" : "Manual backups remain available"} />
        <StatusCard icon={Clock3} label="Last backup" value={lastBackup ? new Date(lastBackup.created_at).toLocaleString() : "No backup yet"} detail={lastBackup ? `${typeLabel(lastBackup.backup_type)} · ${formatBytes(lastBackup.file_size)}` : "Create your first backup"} />
        <StatusCard icon={History} label="Backup library" value={`${backups.length} saved`} detail={`${automaticCount} automatic · keep last ${settings?.retention ?? 7}`} />
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-[1.08fr_.92fr]">
        <Card className="overflow-hidden">
          <div className="border-b border-[#292929] bg-[#111] px-5 py-4"><SectionTitle icon={Settings2} title="Automatic Backup" description="Runs locally from Rust without internet access." /></div>
          {settings && <div className="grid gap-4 p-5 sm:grid-cols-2">
            <ToggleField label="Enable automatic backup" checked={settings.enabled} onChange={enabled => setSettings({...settings, enabled})} />
            <SelectField label="Frequency" value={settings.frequency} onChange={frequency => setSettings({...settings, frequency: frequency as BackupSettings["frequency"]})} options={["daily", "weekly"]} />
            <InputField label="Backup time" value={settings.time} onChange={time => setSettings({...settings, time})} type="time" />
            <InputField label="Keep last backups" value={String(settings.retention)} onChange={retention => setSettings({...settings, retention: Math.max(1, Number(retention) || 1)})} type="number" min={1} max={365} />
            <div className="sm:col-span-2">
              <span className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.06em] text-[#8a8a8a]">Local backup folder</span>
              <div className="flex gap-2">
                <input value={settings.location} onChange={e => setSettings({...settings, location: e.target.value})} placeholder="~/.payroll-system/backups" className="min-w-0 flex-1 rounded-xl border border-[#303030] bg-[#0f0f0f] px-3 py-2.5 text-[12px] text-white outline-none transition focus:border-[#4a8b3f]" />
                <Button variant="secondary" onClick={() => void chooseBackupLocation()} disabled={busy !== "" || !isDesktop()} title={!isDesktop() ? "Folder selection is available in the Tauri desktop app." : "Choose backup folder"}>
                  {busy === "location" ? <Loader2 className="h-4 w-4 animate-spin" /> : <FolderOpen className="h-4 w-4" />} Browse
                </Button>
              </div>
            </div>
          </div>}
          <div className="flex justify-end border-t border-[#242424] px-5 py-4"><Button onClick={saveSettings} disabled={busy !== ""}><Save className="h-4 w-4" />{busy === "settings" ? "Saving…" : "Save Settings"}</Button></div>
        </Card>

        <Card className="overflow-hidden">
          <div className="border-b border-[#292929] bg-[#111] px-5 py-4"><SectionTitle icon={HardDriveDownload} title="Create & Restore" description="Fast actions for secure backup and recovery." /></div>
          <div className="space-y-3 p-5">
            <ActionCard icon={HardDriveDownload} title="Create Backup Now" description="Snapshot, compress, encrypt and verify the current database." button={busy === "create" ? "Creating…" : "Create Backup"} onClick={create} disabled={busy !== ""} />
            <ActionCard icon={Upload} title="Restore from File" description="Choose a .pbak backup from this or an older Payroll System version." button={busy === "restore-file" ? "Opening…" : "Choose Backup"} onClick={restoreFromFile} disabled={busy !== ""} />
          </div>
          <div className="mx-5 mb-5 rounded-xl border border-[#2b3529] bg-[#101510] p-3 text-[11px] leading-5 text-[#8a9b86]"><span className="font-semibold text-[#a7c6a0]">Version safe:</span> older compatible backups are validated and upgraded through the current database migrations after restore.</div>
        </Card>
      </div>

      <Card className="mt-4 overflow-hidden">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#292929] bg-[#111] px-5 py-4"><SectionTitle icon={History} title="Backup History" description="Every saved backup can be verified, restored or deleted." /><Button variant="ghost" onClick={() => void load()} disabled={busy !== ""}><RefreshCw className={`h-4 w-4 ${busy === "refresh" ? "animate-spin" : ""}`} />Refresh</Button></div>
        {backups.length === 0 ? <div className="py-14 text-center text-[12px] text-[#777]">No backups have been created yet.</div> : <div className="overflow-x-auto"><table className="w-full min-w-[900px] text-left text-[12px]"><thead className="bg-[#0e0e0e] text-[10px] uppercase tracking-[0.1em] text-[#777]"><tr><th className="px-5 py-3">Date</th><th className="px-3 py-3">Type</th><th className="px-3 py-3">Size</th><th className="px-3 py-3">Security</th><th className="px-3 py-3">Status</th><th className="px-5 py-3 text-right">Actions</th></tr></thead><tbody>{backups.map(b => <tr key={b.id} className="border-t border-[#242424] hover:bg-white/[0.025]">
          <td className="px-5 py-3"><div className="text-[#ddd]">{new Date(b.created_at).toLocaleDateString()}</div><div className="mt-0.5 text-[10px] text-[#666]">{new Date(b.created_at).toLocaleTimeString()}</div></td>
          <td className="px-3 py-3"><span className="rounded-full border border-[#303030] bg-[#171717] px-2 py-1 text-[10px] text-[#aaa]">{typeLabel(b.backup_type)}</span></td>
          <td className="px-3 py-3 text-[#aaa]">{formatBytes(b.file_size)}</td>
          <td className="px-3 py-3"><span className="inline-flex items-center gap-1.5 text-[#78ad70]"><CheckCircle2 className="h-3.5 w-3.5" />AES-256-GCM</span></td>
          <td className="px-3 py-3"><span className="rounded-full border border-[#30472d] bg-[#142012] px-2 py-1 text-[10px] text-[#8fc686]">{b.status}</span></td>
          <td className="px-5 py-3"><div className="flex justify-end gap-1.5">
            <IconAction title="Verify backup" icon={ShieldCheck} busy={busy === `verify-${b.id}`} disabled={busy !== ""} onClick={() => void verify(b)} />
            <IconAction title="Restore this backup" icon={RotateCcw} busy={busy === `restore-${b.id}`} disabled={busy !== ""} onClick={() => void restore(b)} />
            <IconAction title="Delete backup" icon={Trash2} busy={busy === `delete-${b.id}`} disabled={busy !== ""} onClick={() => void remove(b)} />
          </div></td>
        </tr>)}</tbody></table></div>}
      </Card>
    </div>
  );
}

function SectionTitle({icon: Icon, title, description}:{icon: LucideIcon; title:string; description:string}) { return <div className="flex items-start gap-3"><div className="rounded-lg bg-[#242424] p-2"><Icon className="h-4 w-4 text-[#68a85f]" /></div><div><h2 className="text-[14px] font-semibold text-white">{title}</h2><p className="mt-1 text-[11px] text-[#777]">{description}</p></div></div>; }
function StatusCard({icon:Icon,label,value,detail}:{icon:LucideIcon;label:string;value:string;detail:string}) { return <Card className="p-4"><div className="flex gap-3"><div className="rounded-xl bg-[#4a8b3f]/10 p-2.5"><Icon className="h-4 w-4 text-[#68a85f]" /></div><div className="min-w-0"><p className="text-[10px] uppercase tracking-[0.12em] text-[#777]">{label}</p><p className="mt-1 truncate text-[14px] font-semibold text-white">{value}</p><p className="mt-1 text-[10px] text-[#666]">{detail}</p></div></div></Card>; }
function ActionCard({icon:Icon,title,description,button,onClick,disabled}:{icon:LucideIcon;title:string;description:string;button:string;onClick:()=>void;disabled:boolean}) { return <div className="flex items-center justify-between gap-4 rounded-2xl border border-[#292929] bg-[#101010] p-4"><div className="flex min-w-0 items-start gap-3"><div className="rounded-xl bg-[#4a8b3f]/10 p-2.5"><Icon className="h-4 w-4 text-[#68a85f]" /></div><div className="min-w-0"><p className="text-[13px] font-semibold text-white">{title}</p><p className="mt-1 text-[10px] leading-4 text-[#6f6f6f]">{description}</p></div></div><Button onClick={onClick} disabled={disabled} className="shrink-0">{disabled && (button.endsWith("…") || button === "Opening…") ? <Loader2 className="h-4 w-4 animate-spin" /> : <FolderOpen className="h-4 w-4" />}{button}</Button></div>; }
function IconAction({icon:Icon,title,busy,disabled,onClick}:{icon:LucideIcon;title:string;busy:boolean;disabled:boolean;onClick:()=>void}) { return <button type="button" title={title} aria-label={title} onClick={onClick} disabled={disabled} className="inline-flex h-9 w-9 items-center justify-center rounded-xl border border-[#2a2a2a] bg-[#151515] text-[#8b8b8b] transition hover:border-[#3c5738] hover:bg-[#182017] hover:text-[#78ad70] disabled:cursor-not-allowed disabled:opacity-40">{busy ? <Loader2 className="h-4 w-4 animate-spin" /> : <Icon className="h-4 w-4" />}</button>; }
function InputField({label,value,onChange,...props}:{label:string;value:string;onChange:(v:string)=>void;placeholder?:string;type?:string;min?:number;max?:number}) { return <label className="block"><span className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.06em] text-[#8a8a8a]">{label}</span><input {...props} value={value} onChange={e=>onChange(e.target.value)} className="w-full rounded-xl border border-[#303030] bg-[#0f0f0f] px-3 py-2.5 text-[12px] text-white outline-none transition focus:border-[#4a8b3f]" /></label>; }
function SelectField({label,value,onChange,options}:{label:string;value:string;onChange:(v:string)=>void;options:string[]}) { return <label className="block"><span className="mb-1.5 block text-[11px] font-medium uppercase tracking-[0.06em] text-[#8a8a8a]">{label}</span><select value={value} onChange={e=>onChange(e.target.value)} className="w-full rounded-xl border border-[#303030] bg-[#0f0f0f] px-3 py-2.5 text-[12px] text-white outline-none focus:border-[#4a8b3f]">{options.map(o=><option key={o}>{o}</option>)}</select></label>; }
function ToggleField({label,checked,onChange}:{label:string;checked:boolean;onChange:(v:boolean)=>void}) { return <label className="flex cursor-pointer items-center justify-between rounded-xl border border-[#303030] bg-[#101010] px-3 py-3"><span className="text-[12px] text-[#ddd]">{label}</span><button type="button" aria-pressed={checked} onClick={()=>onChange(!checked)} className={`relative h-6 w-11 rounded-full transition ${checked ? "bg-[#4a8b3f]" : "bg-[#333]"}`}><span className={`absolute top-1 h-4 w-4 rounded-full bg-white transition ${checked ? "left-6" : "left-1"}`} /></button></label>; }
