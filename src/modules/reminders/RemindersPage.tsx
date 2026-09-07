import { FormEvent, useEffect, useMemo, useState } from "react";
import { Bell, CalendarClock, CheckCircle2, Clock3, Pencil, Plus, RotateCcw, Search, Trash2, X, Eye } from "lucide-react";
import { useAuthContext } from "@/stores/authContext";
import { reminderService } from "@/services/reminders";
import { settingsService, type ReminderSettings } from "@/services/settings";
import type { Reminder, ReminderRecurrence, ReminderType } from "@/types/reminders";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";

const types: { value: ReminderType; label: string }[] = [
  { value: "general", label: "General" }, { value: "payroll", label: "Payroll" }, { value: "leave", label: "Leave" },
  { value: "loan", label: "Loan / Advance" }, { value: "backup", label: "Backup" }, { value: "update", label: "Updates" },
];
const recurrences: { value: ReminderRecurrence; label: string }[] = [
  { value: "none", label: "Once" }, { value: "daily", label: "Daily" }, { value: "weekly", label: "Weekly" }, { value: "monthly", label: "Monthly" },
];
const inputClass = "w-full rounded-xl border border-[#303030] bg-[#101010] px-3 py-2.5 text-[12px] text-white outline-none focus:border-[#4a8b3f]";
const typeLabel = (t: ReminderType) => types.find(x => x.value === t)?.label ?? "General";
const formatDue = (value: unknown) => { const s = typeof value === "string" ? value : ""; if (!s) return "Not scheduled"; const normalized = s.includes("T") ? s : s.replace(" ", "T"); const d = new Date(/Z$|[+-]\d{2}:?\d{2}$/.test(normalized) ? normalized : `${normalized}Z`); return Number.isNaN(d.getTime()) ? s : d.toLocaleString([], { dateStyle: "medium", timeStyle: "short" }); };
const toLocalInput = (d: Date) => { const p = (n: number) => String(n).padStart(2, "0"); return `${d.getFullYear()}-${p(d.getMonth()+1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`; };
const isDue = (r: Reminder) => !r.is_completed && new Date(r.due_date).getTime() <= Date.now() && (!r.snoozed_until || new Date(r.snoozed_until).getTime() <= Date.now());
const isSnoozed = (r: Reminder) => !r.is_completed && !!r.snoozed_until && new Date(r.snoozed_until).getTime() > Date.now();

type Filter = "all" | "pending" | "due" | "snoozed" | "completed";

export function RemindersPage() {
  const auth = useAuthContext(), token = auth.token;
  const [items, setItems] = useState<Reminder[]>([]), [busy, setBusy] = useState(""), [error, setError] = useState(""), [message, setMessage] = useState("");
  const [settings, setSettings] = useState<ReminderSettings | null>(null);
  const [openForm, setOpenForm] = useState(false), [editing, setEditing] = useState<Reminder | null>(null);
  const [query, setQuery] = useState(""), [category, setCategory] = useState("all"), [filter, setFilter] = useState<Filter>("all");
  const load = async () => { if (!token) return; try { const all = await reminderService.list(token); setItems(Array.isArray(all) ? all.filter(Boolean) : []); } catch (e) { setError(e instanceof Error ? e.message : "Unable to load reminders."); } };
  const loadSettings = async () => { try { setSettings(await settingsService.getReminderSettings()); } catch { setSettings(null); } };
  useEffect(() => { void Promise.all([load(), loadSettings()]); }, [token]);
  useEffect(() => { const seconds = settings?.polling_interval_seconds ?? 30; const t = window.setInterval(() => void load(), Math.max(5, seconds) * 1000); return () => window.clearInterval(t); }, [token, settings?.polling_interval_seconds]);

  const pending = useMemo(() => items.filter(r => !r.is_completed), [items]);
  const due = useMemo(() => items.filter(isDue), [items]);
  const completed = useMemo(() => items.filter(r => r.is_completed), [items]);
  const recurring = useMemo(() => items.filter(r => r.recurrence !== "none"), [items]);
  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return items.filter(r => {
      if (!settings?.show_completed && r.is_completed) return false;
      if (category !== "all" && r.reminder_type !== category) return false;
      if (filter === "pending" && (r.is_completed || isDue(r) || isSnoozed(r))) return false;
      if (filter === "due" && !isDue(r)) return false;
      if (filter === "snoozed" && !isSnoozed(r)) return false;
      if (filter === "completed" && !r.is_completed) return false;
      if (q && !`${r.title} ${r.message ?? ""} ${r.reminder_type}`.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [items, query, category, filter, settings?.show_completed]);

  const emptyDraft = () => ({ title: "", message: "", reminder_type: (settings?.default_category ?? "general") as ReminderType, due_date: toLocalInput(new Date(Date.now() + 3600000)), recurrence: "none" as ReminderRecurrence });
  const [draft, setDraft] = useState(emptyDraft());
  useEffect(() => { if (openForm) setDraft(editing ? { title: editing.title, message: editing.message ?? "", reminder_type: editing.reminder_type, due_date: (() => { const d = new Date(editing.due_date); return Number.isNaN(d.getTime()) ? toLocalInput(new Date(Date.now()+3600000)) : toLocalInput(d); })(), recurrence: editing.recurrence } : emptyDraft()); }, [openForm, editing, settings?.default_category]);

  const save = async (e: FormEvent) => { e.preventDefault(); if (!token) return; setBusy("save"); setError(""); setMessage(""); try { const title = draft.title.trim(); if (!title) throw new Error("Reminder title is required."); const dueDate = new Date(draft.due_date); if (Number.isNaN(dueDate.getTime())) throw new Error("Please choose a valid date and time."); await reminderService.save(token, { id: editing?.id, title, message: draft.message?.trim() || null, reminderType: draft.reminder_type, dueDate: dueDate.toISOString(), recurrence: draft.recurrence }); setOpenForm(false); setEditing(null); setMessage(editing ? "Reminder updated." : "Reminder created."); await load(); } catch (e) { setError(e instanceof Error ? e.message : "Unable to save reminder."); } finally { setBusy(""); } };
  const act = async (kind: "complete" | "snooze" | "delete" | "read", r: Reminder) => { if (!token) return; setBusy(`${kind}-${r.id}`); setError(""); try { if (kind === "complete") await reminderService.complete(token, r.id); else if (kind === "snooze") await reminderService.snooze(token, r.id, settings?.default_snooze_minutes ?? 60); else if (kind === "read") await reminderService.markRead(token, r.id); else await reminderService.remove(token, r.id); setMessage(kind === "complete" ? "Reminder completed." : kind === "snooze" ? `Reminder snoozed for ${settings?.default_snooze_minutes ?? 60} minutes.` : kind === "read" ? "Reminder marked as read." : "Reminder deleted."); await load(); } catch (e) { setError(e instanceof Error ? e.message : "Reminder action failed."); } finally { setBusy(""); } };

  const status = (r: Reminder) => { if (r.is_completed) return ["Completed", "bg-[#202020] text-[#8a8a8a]"]; if (isSnoozed(r)) return ["Snoozed", "bg-[#252017] text-[#d6bc7e]"]; if (isDue(r)) return ["Due", "bg-[#183017] text-[#9ed394]"]; return ["Pending", "bg-[#202020] text-[#bdbdbd]"]; };
  const stats = [{ label: "Pending", value: pending.length, icon: Clock3 }, { label: "Due now", value: due.length, icon: Bell }, { label: "Completed", value: completed.length, icon: CheckCircle2 }, { label: "Recurring", value: recurring.length, icon: RotateCcw }];

  return <div className="space-y-5">
    <PageHeader title="Reminders" description="Manage payroll, leave, loan, backup and system tasks on schedule." />
    {(error || message) && <div className={`rounded-xl border px-4 py-3 text-[12px] ${error ? "border-[#493b2b] bg-[#19150f] text-[#e4c58d]" : "border-[#29432a] bg-[#111811] text-[#9ed394]"}`}>{error || message}</div>}
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{stats.map(x => <Card key={x.label} className="p-3.5"><div className="flex items-center justify-between"><div><p className="text-[10px] uppercase tracking-[.14em] text-[#707070]">{x.label}</p><p className="mt-1 text-xl font-semibold text-white">{x.value}</p></div><x.icon className="h-4.5 w-4.5 text-[#4a8b3f]"/></div></Card>)}</div>
    <Card className="overflow-hidden">
      <div className="flex flex-col gap-3 border-b border-[#292929] p-4 lg:flex-row lg:items-center lg:justify-between">
        <div><h2 className="text-sm font-semibold text-white">Reminder schedule</h2><p className="mt-1 text-[11px] text-[#707070]">Search, filter and manage live SQLite reminders.</p></div>
        <Button onClick={() => { setError(""); setMessage(""); setEditing(null); setOpenForm(true); }}><Plus className="h-4 w-4"/> New reminder</Button>
      </div>
      <div className="grid gap-2 border-b border-[#242424] bg-[#111111] p-3 md:grid-cols-[1fr_150px_150px]">
        <div className="relative"><Search className="absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-[#666]"/><input className={inputClass+" pl-9"} placeholder="Search title, description..." value={query} onChange={e=>setQuery(e.target.value)}/></div>
        <select className={inputClass} value={category} onChange={e=>setCategory(e.target.value)}><option value="all">All categories</option>{types.map(t=><option key={t.value} value={t.value}>{t.label}</option>)}</select>
        <select className={inputClass} value={filter} onChange={e=>setFilter(e.target.value as Filter)}><option value="all">All statuses</option><option value="pending">Pending</option><option value="due">Due</option><option value="snoozed">Snoozed</option><option value="completed">Completed</option></select>
      </div>
      <div className="flex gap-1 overflow-x-auto border-b border-[#242424] px-3 py-2">{(["all","pending","due","snoozed","completed"] as Filter[]).map(f => <button key={f} type="button" onClick={()=>setFilter(f)} className={`rounded-lg px-3 py-1.5 text-[10px] font-semibold capitalize transition ${filter===f?"bg-[#4a8b3f] text-white":"text-[#777] hover:bg-[#1e1e1e] hover:text-white"}`}>{f}</button>)}</div>
      {filtered.length === 0 ? <div className="p-12 text-center"><CalendarClock className="mx-auto h-8 w-8 text-[#4a8b3f]"/><p className="mt-3 text-sm text-white">{items.length ? "No reminders match your filters" : "No reminders yet"}</p><p className="mt-1 text-xs text-[#707070]">{items.length ? "Try another search or status/category filter." : "Create your first payroll or system reminder."}</p></div> :
      <div className="divide-y divide-[#242424]">{filtered.map(r => { const [label, badge] = status(r); return <div key={r.id} className="grid gap-3 px-4 py-3.5 transition hover:bg-[#151515] md:grid-cols-[minmax(0,1fr)_190px_auto] md:items-center">
        <div className="min-w-0"><div className="flex flex-wrap items-center gap-1.5"><span className="rounded-full bg-[#183017] px-2 py-1 text-[9px] font-semibold text-[#9ed394]">{typeLabel(r.reminder_type)}</span><span className={`rounded-full px-2 py-1 text-[9px] font-semibold ${badge}`}>{label}</span>{r.recurrence!=="none"&&<span className="rounded-full border border-[#303030] px-2 py-1 text-[9px] text-[#888]">↻ {r.recurrence}</span>}</div><p className={`mt-1.5 truncate text-[12px] font-semibold ${r.is_completed?"text-[#666] line-through":"text-white"}`}>{r.title}</p>{r.message&&<p className="mt-0.5 truncate text-[10px] text-[#707070]">{r.message}</p>}</div>
        <div className="text-left md:text-right"><p className="text-[10px] text-[#777]">{r.is_completed ? "Completed" : isSnoozed(r) ? "Snoozed until" : "Due"}</p><p className="mt-0.5 text-[10px] text-[#bdbdbd]">{formatDue(r.is_completed && r.completed_at ? r.completed_at : isSnoozed(r) ? r.snoozed_until : r.due_date)}</p></div>
        <div className="flex items-center justify-start gap-1 md:justify-end">{!r.is_completed&&<Button variant="ghost" title="Complete" onClick={()=>void act("complete",r)} disabled={busy!==""}><CheckCircle2 className="h-4 w-4"/></Button>}{!r.is_completed&&<Button variant="ghost" title="Snooze" onClick={()=>void act("snooze",r)} disabled={busy!==""}><Clock3 className="h-4 w-4"/></Button>}{!r.is_completed&&!r.read_at&&<Button variant="ghost" title="Mark as read" onClick={()=>void act("read",r)} disabled={busy!==""}><Eye className="h-4 w-4"/></Button>}<Button variant="ghost" title="Edit" onClick={()=>{setEditing(r);setOpenForm(true)}}><Pencil className="h-4 w-4"/></Button><Button variant="ghost" title="Delete" onClick={()=>void act("delete",r)} disabled={busy!==""}><Trash2 className="h-4 w-4"/></Button></div>
      </div>})}</div>}
    </Card>
    {openForm&&<div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4"><Card className="w-full max-w-lg p-5"><div className="mb-5 flex items-center justify-between"><div><h2 className="text-base font-semibold text-white">{editing?"Edit reminder":"New reminder"}</h2><p className="text-[11px] text-[#707070]">Set the schedule and recurrence for this reminder.</p></div><button onClick={()=>setOpenForm(false)} className="text-[#777] hover:text-white"><X className="h-5 w-5"/></button></div><form onSubmit={save} className="space-y-4"><div><label className="text-[11px] text-[#999]">Title</label><input className={inputClass+" mt-1"} value={draft.title} onChange={e=>setDraft({...draft,title:e.target.value})} maxLength={200} required/></div><div><label className="text-[11px] text-[#999]">Description</label><textarea className={inputClass+" mt-1 min-h-20 resize-y"} value={draft.message??""} onChange={e=>setDraft({...draft,message:e.target.value})} maxLength={2000}/></div><div className="grid gap-3 sm:grid-cols-2"><div><label className="text-[11px] text-[#999]">Category</label><select className={inputClass+" mt-1"} value={draft.reminder_type} onChange={e=>setDraft({...draft,reminder_type:e.target.value as ReminderType})}>{types.map(t=><option key={t.value} value={t.value}>{t.label}</option>)}</select></div><div><label className="text-[11px] text-[#999]">Recurrence</label><select className={inputClass+" mt-1"} value={draft.recurrence} onChange={e=>setDraft({...draft,recurrence:e.target.value as ReminderRecurrence})}>{recurrences.map(r=><option key={r.value} value={r.value}>{r.label}</option>)}</select></div></div><div><label className="text-[11px] text-[#999]">Due date & time</label><input type="datetime-local" className={inputClass+" mt-1"} value={draft.due_date.slice(0,16)} onChange={e=>setDraft({...draft,due_date:e.target.value})} required/></div><div className="flex justify-end gap-2 pt-2"><Button type="button" variant="secondary" onClick={()=>setOpenForm(false)}>Cancel</Button><Button type="submit" disabled={busy==="save"}>{busy==="save"?"Saving…":editing?"Save changes":"Create reminder"}</Button></div></form></Card></div>}
  </div>;
}
