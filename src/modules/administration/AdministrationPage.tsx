import { PageSkeleton } from "@/components/ui/PageSkeleton";
import { useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, Clock3, Database, Info, KeyRound, Lock, RefreshCw, Save, Search, ShieldCheck, UserRound, Activity, HardDrive, Smartphone, Trash2, Copy } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { useAuthContext } from "@/stores/authContext";
import { authService, getAuthSessionToken } from "@/services/auth";
import type { AuditEntry, SystemInfo } from "@/types/auth";

type Profile = { id:number; username:string; created_at:string; updated_at:string };
type Section = "account" | "security" | "audit" | "system";

const input = "mt-1 w-full rounded-xl border border-[#303030] bg-[#101010] px-3 py-2.5 text-xs text-white outline-none focus:border-[#4a8b3f] focus:ring-1 focus:ring-[#4a8b3f]";
const formatDate = (value: string) => { const d = new Date(value.includes("T") ? value : value.replace(" ", "T") + "Z"); return Number.isNaN(d.getTime()) ? value : d.toLocaleString([], { dateStyle:"medium", timeStyle:"short" }); };
const formatBytes = (n:number) => n < 1024*1024 ? `${Math.max(1, Math.round(n/1024))} KB` : `${(n/1024/1024).toFixed(1)} MB`;
const errorMessage = (e: unknown, fallback: string) => {
  if (typeof e === "string" && e.trim()) return e;
  if (e instanceof Error && e.message) return e.message;
  if (e && typeof e === "object" && "message" in e) {
    const message = (e as { message?: unknown }).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
};

export function AdministrationPage() {
  const auth = useAuthContext();
  const token = getAuthSessionToken(auth.token);
  const [section,setSection]=useState<Section>("account");
  const [profile,setProfile]=useState<Profile|null>(null);
  const [security,setSecurity]=useState({auto_lock_minutes:15,session_timeout_minutes:480});
  const [audit,setAudit]=useState<AuditEntry[]>([]);
  const [system,setSystem]=useState<SystemInfo|null>(null);
  const [auditSearch,setAuditSearch]=useState("");
  const [busy,setBusy]=useState(""); const [message,setMessage]=useState(""); const [error,setError]=useState("");
  const [usernamePassword,setUsernamePassword]=useState(""); const [newUsername,setNewUsername]=useState("");
  const [currentPassword,setCurrentPassword]=useState(""); const [newPassword,setNewPassword]=useState(""); const [confirmPassword,setConfirmPassword]=useState(""); const [recoveryCode,setRecoveryCode]=useState("");

  const loadBase=async()=>{ if(!token)return; setError(""); try { const [p,s]=await Promise.all([authService.getAdminProfile(),authService.getSecuritySettings()]); setProfile(p); setSecurity({auto_lock_minutes:Number.isFinite(Number(s.auto_lock_minutes))?Number(s.auto_lock_minutes):15,session_timeout_minutes:Number.isFinite(Number(s.session_timeout_minutes))?Number(s.session_timeout_minutes):480}); } catch(e){setError(errorMessage(e,"Unable to load administration data."));} };
  const loadAudit=async()=>{ if(!token)return; try{setAudit(await authService.getAuditLogs(token,auditSearch,200));}catch(e){setError(errorMessage(e,"Unable to load audit logs."));} };
  const loadSystem=async()=>{ if(!token)return; try{setSystem(await authService.getSystemInfo(token));}catch(e){setError(errorMessage(e,"Unable to load system information."));} };
  useEffect(()=>{void loadBase();},[token]);
  useEffect(()=>{if(section==="audit")void loadAudit(); if(section==="system")void loadSystem();},[section,token]);

  const changeUsername=async()=>{
    setMessage("");setError("");
    const username=newUsername.trim();
    if(username.length<3||username.length>50){setError("Username must be between 3 and 50 characters.");return;}
    if(!/^[A-Za-z0-9._-]+$/.test(username)){setError("Use only letters, numbers, dot, underscore or hyphen.");return;}
    if(!usernamePassword){setError("Enter your current password to authorize the change.");return;}
    setBusy("username");
    try{const r=await authService.updateUsername(token,usernamePassword,username); if(!r.success){setError(r.message);return;} setProfile(p=>p?{...p,username}:p); setUsernamePassword(""); setNewUsername(""); setMessage("Username updated successfully.");}
    catch(e){setError(errorMessage(e,"Unable to update username."));} finally{setBusy("");}
  };

  const changePassword=async()=>{
    setMessage("");setError("");
    if(newPassword.length<8){setError("New password must be at least 8 characters.");return;}
    if(newPassword!==confirmPassword){setError("New passwords do not match.");return;}
    setBusy("password");
    try{const r=await authService.changePassword(token,currentPassword,newPassword); if(!r.success){setError(r.message);return;} setRecoveryCode(r.recovery_code??""); setMessage("Password changed successfully. A new recovery key was generated."); setCurrentPassword("");setNewPassword("");setConfirmPassword("");}
    catch(e){setError(errorMessage(e,"Unable to change password."));} finally{setBusy("");}
  };
  const clearAudit=async()=>{
    setMessage("");setError("");
    if(!window.confirm("Clear the audit history? This cannot be undone. A new audit entry will record the cleanup.")) return;
    setBusy("audit");
    try{const removed=await authService.clearAuditLogs(token); setAudit([]); setMessage(`Audit history cleared. ${removed} entries removed.`); await loadAudit();}
    catch(e){setError(errorMessage(e,"Unable to clear audit logs."));} finally{setBusy("");}
  };

  const saveSecurity=async()=>{
    setMessage("");setError("");
    const a=Math.round(Number(security.auto_lock_minutes)), t=Math.round(Number(security.session_timeout_minutes));
    if(!Number.isFinite(a)||!Number.isFinite(t)||a<0||a>10080||t<1||t>43200){setError("Use valid security times within the allowed limits.");return;}
    if(a>t){setError("Automatic lock cannot be longer than the session timeout.");return;}
    setBusy("security");
    try{const r=await authService.updateSecuritySettings(token,a,t); if(!r.success){setError(r.message);return;} setSecurity({auto_lock_minutes:a,session_timeout_minutes:t}); setMessage("Security settings saved successfully.");}
    catch(e){setError(errorMessage(e,"Unable to save security settings."));} finally{setBusy("");}
  };

  const nav: {id:Section;label:string;icon:typeof UserRound}[]=[{id:"account",label:"Admin User",icon:UserRound},{id:"security",label:"Security",icon:Lock},{id:"audit",label:"Audit Logs",icon:Activity},{id:"system",label:"System Information",icon:Database}];

  return <div className="pb-8">
    <PageHeader title="Administration" description="Manage the administrator account, security, audit trail and system information."/>
    <div className="mb-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{nav.map(n=>{const Icon=n.icon;return <button key={n.id} onClick={()=>{setSection(n.id);setMessage("");setError("");}} className={`rounded-2xl border p-4 text-left transition ${section===n.id?"border-[#4a8b3f] bg-[#142012]":"border-[#292929] bg-[#151515] hover:border-[#3a3a3a]"}`}><Icon className={`h-5 w-5 ${section===n.id?"text-[#67b85a]":"text-[#707070]"}`}/><p className="mt-3 text-xs font-semibold text-white">{n.label}</p><p className="mt-1 text-[10px] text-[#707070]">{n.id==="account"?"Administrator account":""}{n.id==="security"?"Sessions and protection":""}{n.id==="audit"?"Security activity history":""}{n.id==="system"?"Application health and details":""}</p></button>})}</div>
    {(message||error)&&<div className={`mb-5 flex items-center gap-2 rounded-xl border px-4 py-3 text-xs ${error?"border-[#493b2b] bg-[#19150f] text-[#e4c58d]":"border-[#294126] bg-[#142012] text-[#a8d9a0]"}`}>{error?<AlertCircle className="h-4 w-4"/>:<CheckCircle2 className="h-4 w-4"/>}{error||message}</div>}

    {section==="account"&&<div className="grid gap-5 xl:grid-cols-2">
      <Card className="overflow-hidden"><div className="border-b border-[#292929] px-5 py-4"><div className="flex items-center gap-3"><div className="flex h-10 w-10 items-center justify-center rounded-xl bg-[#183017]"><UserRound className="h-5 w-5 text-[#67b85a]"/></div><div><h2 className="text-sm font-semibold text-white">Administrator account</h2><p className="mt-1 text-[11px] text-[#707070]">Version 1 uses a single administrator account.</p></div></div></div><div className="space-y-4 p-5"><div><p className="text-[10px] uppercase tracking-wider text-[#707070]">Current username</p><p className="mt-1 text-base font-semibold text-white">{profile?.username??"Loading…"}</p></div><div className="border-t border-[#292929] pt-4"><p className="text-xs font-semibold text-white">Change username</p><p className="mt-1 text-[10px] leading-4 text-[#707070]">Changing the username requires the current password and is recorded in the audit trail.</p><label className="mt-3 block text-[11px] text-[#999]">New username<input autoComplete="username" maxLength={50} className={input} value={newUsername} onChange={e=>setNewUsername(e.target.value)} placeholder="e.g. admin.mud"/></label><label className="mt-3 block text-[11px] text-[#999]">Current password<input type="password" autoComplete="current-password" className={input} value={usernamePassword} onChange={e=>setUsernamePassword(e.target.value)}/></label><div className="mt-3 flex items-center justify-between gap-3"><span className="text-[10px] text-[#666]">Allowed: A–Z, 0–9, . _ -</span><Button onClick={()=>void changeUsername()} disabled={busy==="username"||!newUsername||!usernamePassword}><UserRound className="h-4 w-4"/>{busy==="username"?"Updating…":"Update username"}</Button></div></div><div className="flex items-center gap-2 rounded-xl border border-[#294126] bg-[#142012] p-3 text-[11px] text-[#9ed394]"><ShieldCheck className="h-4 w-4"/> Password hashing, authenticated sessions and audit logging are enabled.</div></div></Card>
      <Card className="overflow-hidden"><div className="border-b border-[#292929] px-5 py-4"><div className="flex items-center gap-2"><KeyRound className="h-4 w-4 text-[#4a8b3f]"/><div><h2 className="text-sm font-semibold text-white">Change password</h2><p className="mt-1 text-[11px] text-[#707070]">Use your current password to set a new administrator password.</p></div></div></div><div className="space-y-4 p-5"><label className="block text-[11px] text-[#999]">Current password<input type="password" autoComplete="current-password" className={input} value={currentPassword} onChange={e=>setCurrentPassword(e.target.value)}/></label><label className="block text-[11px] text-[#999]">New password<input type="password" autoComplete="new-password" className={input} value={newPassword} onChange={e=>setNewPassword(e.target.value)}/></label><label className="block text-[11px] text-[#999]">Confirm new password<input type="password" autoComplete="new-password" className={input} value={confirmPassword} onChange={e=>setConfirmPassword(e.target.value)}/></label><Button onClick={()=>void changePassword()} disabled={busy==="password"||!currentPassword||!newPassword||!confirmPassword}><KeyRound className="h-4 w-4"/>{busy==="password"?"Changing…":"Change password"}</Button>{recoveryCode&&<div className="rounded-xl border border-[#294126] bg-[#142012] p-4"><div className="flex items-start gap-3"><ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-[#67b85a]"/><div className="min-w-0 flex-1"><p className="text-xs font-semibold text-white">New recovery key</p><p className="mt-1 text-[10px] leading-4 text-[#8f9f8b]">The previous recovery key is invalid. Save this new key somewhere secure; it is shown only after this password change.</p><div className="mt-3 flex items-center gap-2"><code className="min-w-0 flex-1 overflow-x-auto rounded-lg border border-[#31462f] bg-[#0c100c] px-3 py-2 text-center text-xs font-bold tracking-[0.18em] text-[#d9ead5]">{recoveryCode}</code><button type="button" title="Copy recovery key" onClick={()=>void navigator.clipboard?.writeText(recoveryCode)} className="grid h-9 w-9 shrink-0 place-items-center rounded-lg border border-[#31462f] bg-[#101510] text-[#78b96e] hover:bg-[#1a2419]"><Copy className="h-4 w-4"/></button></div></div></div></div>}</div></Card>
    </div>}

    {section==="security"&&<Card className="overflow-hidden"><div className="border-b border-[#292929] px-5 py-4"><div className="flex items-center gap-2"><Lock className="h-4 w-4 text-[#4a8b3f]"/><div><h2 className="text-sm font-semibold text-white">Session security</h2><p className="mt-1 text-[11px] text-[#707070]">Protect the workspace with inactivity locking and session expiry.</p></div></div></div><div className="grid gap-5 p-5 md:grid-cols-2"><label className="block text-[11px] text-[#999]">Automatic lock (minutes)<input type="number" min="0" max="10080" className={input} value={security.auto_lock_minutes} onChange={e=>setSecurity({...security,auto_lock_minutes:Number(e.target.value)})}/><span className="mt-1 block text-[10px] text-[#666]">Locks the workspace after inactivity. Use 0 to disable automatic locking.</span></label><label className="block text-[11px] text-[#999]">Session timeout (minutes)<input type="number" min="1" max="43200" className={input} value={security.session_timeout_minutes} onChange={e=>setSecurity({...security,session_timeout_minutes:Number(e.target.value)})}/><span className="mt-1 block text-[10px] text-[#666]">Expires the authenticated session.</span></label></div><div className="flex items-center justify-between border-t border-[#292929] px-5 py-4"><div className="flex items-center gap-2 text-[10px] text-[#707070]"><Clock3 className="h-4 w-4"/> Security changes are audited.</div><Button onClick={()=>void saveSecurity()} disabled={busy==="security"}><Save className="h-4 w-4"/>{busy==="security"?"Saving…":"Save security"}</Button></div></Card>}

    {section==="audit"&&<Card className="overflow-hidden"><div className="flex flex-col gap-3 border-b border-[#292929] px-5 py-4 md:flex-row md:items-center md:justify-between"><div><h2 className="text-sm font-semibold text-white">Audit Logs</h2><p className="mt-1 text-[11px] text-[#707070]">Review security and important system activity.</p></div><div className="flex gap-2"><div className="relative"><Search className="absolute left-3 top-2.5 h-4 w-4 text-[#666]"/><input className="w-64 rounded-xl border border-[#303030] bg-[#101010] py-2.5 pl-9 pr-3 text-xs text-white outline-none focus:border-[#4a8b3f]" placeholder="Search activity…" value={auditSearch} onChange={e=>setAuditSearch(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void loadAudit();}}/></div><Button variant="secondary" onClick={()=>void loadAudit()}><RefreshCw className="h-4 w-4"/>Refresh</Button><Button variant="secondary" onClick={()=>void clearAudit()} disabled={busy==="audit"||audit.length===0}><Trash2 className="h-4 w-4"/>{busy==="audit"?"Clearing…":"Clear history"}</Button></div></div>{audit.length===0?<div className="p-12 text-center"><Activity className="mx-auto h-8 w-8 text-[#4a8b3f]"/><p className="mt-3 text-sm text-white">No audit entries found</p></div>:<div className="divide-y divide-[#242424]">{audit.map(a=><div key={a.id} className="grid gap-2 px-5 py-4 md:grid-cols-[180px_1fr_220px]"><div><p className="text-[11px] font-semibold text-white">{a.action.replace(/_/g," ")}</p><p className="mt-1 text-[10px] text-[#666]">{formatDate(a.created_at)}</p></div><div><p className="text-[11px] text-[#aaa]">{a.entity_type??"System"}{a.entity_id!=null?` #${a.entity_id}`:""}</p>{a.details&&<p className="mt-1 break-words text-[10px] text-[#707070]">{a.details}</p>}</div><div className="md:text-right"><span className="inline-flex rounded-full border border-[#294126] bg-[#142012] px-2.5 py-1 text-[9px] font-semibold text-[#9ed394]">Recorded</span></div></div>)}</div>}</Card>}

    {section==="system"&&<div className="space-y-5">{!system?<PageSkeleton variant="dashboard" />:<><div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">{[["Application",system.app_version,HardDrive],["Database",system.database_version,Database],["Platform",system.platform,Smartphone],["Architecture",system.architecture,Info]].map(([label,value,Icon])=>{const I=Icon as typeof HardDrive;return <Card key={String(label)} className="p-4"><I className="h-5 w-5 text-[#4a8b3f]"/><p className="mt-3 text-[10px] uppercase tracking-wider text-[#707070]">{label as string}</p><p className="mt-1 text-sm font-semibold text-white">{String(value)}</p></Card>})}</div><div className="grid gap-5 xl:grid-cols-2"><Card className="p-5"><div className="flex items-center gap-2"><Database className="h-4 w-4 text-[#4a8b3f]"/><h2 className="text-sm font-semibold text-white">Database</h2></div><p className="mt-3 break-all text-[11px] text-[#777]">{system.database_path}</p><p className="mt-2 text-xs text-white">Database size: <span className="text-[#9ed394]">{formatBytes(system.database_size_bytes)}</span></p></Card><Card className="p-5"><h2 className="text-sm font-semibold text-white">System statistics</h2><div className="mt-4 grid grid-cols-2 gap-3">{[["Employees",system.employee_count],["Departments",system.department_count],["Backups",system.backup_count],["Paired devices",system.paired_device_count],["Audit entries",system.audit_count]].map(([l,v])=><div key={String(l)} className="rounded-xl border border-[#292929] bg-[#101010] p-3"><p className="text-[10px] text-[#707070]">{l as string}</p><p className="mt-1 text-lg font-semibold text-white">{String(v)}</p></div>)}</div></Card></div></>}</div>}

    <Card className="mt-5 p-4"><div className="flex items-start gap-3"><Info className="mt-0.5 h-4 w-4 shrink-0 text-[#4a8b3f]"/><div><p className="text-xs font-semibold text-white">Administration boundary</p><p className="mt-1 text-[11px] leading-5 text-[#777]">Administration manages the administrator, security, audit trail and system information. Application configuration remains reserved for the Settings module.</p></div></div></Card>
  </div>;
}
