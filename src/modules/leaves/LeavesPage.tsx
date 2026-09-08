import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  CalendarDays,
  Check,
  Clock3,
  Edit3,
  History,
  Plus,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { InlineSkeleton } from "@/components/ui/PageSkeleton";
import { leaveService, type LeaveBalance, type LeaveRecord, type LeaveType } from "@/types/leaves";
import { staffService } from "@/services/staff";
import type { Employee } from "@/types/staff";

const inputClass =
  "mt-1 w-full rounded-xl border border-[#2a2a2a] bg-[#111111] px-3 py-2.5 text-sm text-white outline-none transition-all duration-200 placeholder:text-[#6f6f6f] focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/20";
const tabClass =
  "rounded-xl px-3 py-2 text-sm font-medium transition-all duration-200";

export function LeavesPage() {
  const [tab, setTab] = useState<"records" | "history" | "balances" | "types">("records");
  const [types, setTypes] = useState<LeaveType[]>([]);
  const [records, setRecords] = useState<LeaveRecord[]>([]);
  const [balances, setBalances] = useState<LeaveBalance[]>([]);
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [year, setYear] = useState(new Date().getFullYear());
  const [showLeave, setShowLeave] = useState(false);
  const [editingRecord, setEditingRecord] = useState<LeaveRecord | null>(null);
  const [showType, setShowType] = useState(false);
  const [editingType, setEditingType] = useState<LeaveType | null>(null);
  const [error, setError] = useState("");
  const [busyId, setBusyId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  const load = async () => {
    setError("");
    setLoading(true);
    try {
      const [leaveTypes, leaveRecords, leaveBalances, staff] = await Promise.all([
        leaveService.getTypes(),
        leaveService.getRecords(year),
        leaveService.getBalances(year),
        staffService.getEmployees(),
      ]);
      setTypes(leaveTypes);
      setRecords(leaveRecords);
      setBalances(leaveBalances);
      setEmployees(staff);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, [year]);

  const approvedCount = useMemo(
    () => records.filter((record) => record.status === "approved").length,
    [records],
  );
  const pendingCount = useMemo(
    () => records.filter((record) => record.status === "pending").length,
    [records],
  );
  const totalUsed = useMemo(
    () => balances.reduce((sum, balance) => sum + Number(balance.used || 0), 0),
    [balances],
  );

  const updateStatus = async (id: number, status: "approved" | "rejected") => {
    setBusyId(id);
    try {
      const result = await leaveService.updateStatus(id, status);
      if (!result.success) throw new Error(result.message);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusyId(null);
    }
  };

  const openCreateLeave = () => {
    setEditingRecord(null);
    setShowLeave(true);
  };

  return (
    <div className="leaves-page space-y-4 animate-[fade-in_.35s_ease-out]">
      <PageHeader
        title="Leaves"
        description="Manage leave types, employee requests, approvals, history, and yearly balances."
      />
      {loading ? <InlineSkeleton rows={6} /> : <>

      <div className="grid gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
        <SummaryCard icon={CalendarDays} label="Leave types" value={types.length} />
        <SummaryCard icon={Clock3} label="Pending requests" value={pendingCount} />
        <SummaryCard icon={Check} label="Approved requests" value={approvedCount} />
        <SummaryCard icon={History} label={`Days used · ${year}`} value={totalUsed} />
      </div>

      <Card className="border-[#292929] bg-[#121212]/95 p-1.5 shadow-[0_14px_38px_rgba(0,0,0,0.24)]">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center">
          <div className="flex flex-wrap items-center gap-2">
            <TabButton active={tab === "records"} onClick={() => setTab("records")}>
              Records
            </TabButton>
            <TabButton active={tab === "history"} onClick={() => setTab("history")}>
              History
            </TabButton>
            <TabButton active={tab === "balances"} onClick={() => setTab("balances")}>
              Balances
            </TabButton>
            <TabButton active={tab === "types"} onClick={() => setTab("types")}>
              Leave Types
            </TabButton>
          </div>
          <div className="flex flex-1 items-center justify-end gap-2">
            <select
              aria-label="Leave year"
              value={year}
              onChange={(event) => setYear(Number(event.target.value))}
              className="h-9 rounded-lg border border-[#2b2b2b] bg-[#111111] px-3 text-[12px] text-white outline-none transition-all duration-200 focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/10"
            >
              <option value={year - 1}>{year - 1}</option>
              <option value={year}>{year}</option>
              <option value={year + 1}>{year + 1}</option>
            </select>
            {tab === "records" && <Button onClick={openCreateLeave}><Plus className="h-4 w-4" /> Register Leave</Button>}
            {tab === "types" && (
              <Button
                onClick={() => {
                  setEditingType(null);
                  setShowType(true);
                }}
              >
                <Plus className="h-4 w-4" /> Add Leave Type
              </Button>
            )}
          </div>
        </div>
      </Card>

      {error && (
        <div className="rounded-xl border border-[#4a8b3f]/40 bg-[#4a8b3f]/10 px-4 py-3 text-sm text-white">
          {error}
        </div>
      )}

      {tab === "records" && (
        <RecordTable
          records={records}
          busyId={busyId}
          onEdit={(record) => {
            setEditingRecord(record);
            setShowLeave(true);
          }}
          onApprove={(id) => void updateStatus(id, "approved")}
          onReject={(id) => void updateStatus(id, "rejected")}
        />
      )}

      {tab === "history" && <HistoryTable records={records} />}

      {tab === "balances" && <BalanceTable balances={balances} year={year} />}

      {tab === "types" && (
        <LeaveTypeGrid
          types={types}
          onEdit={(type) => {
            setEditingType(type);
            setShowType(true);
          }}
          onDelete={async (id) => {
            const result = await leaveService.deleteType(id);
            if (!result.success) setError(result.message);
            else await load();
          }}
        />
      )}

      {showType && (
        <LeaveTypeForm
          type={editingType}
          onClose={() => setShowType(false)}
          onSaved={() => {
            setShowType(false);
            void load();
          }}
        />
      )}

      {showLeave && (
        <LeaveForm
          record={editingRecord}
          types={types}
          employees={employees}
          onClose={() => {
            setShowLeave(false);
            setEditingRecord(null);
          }}
          onSaved={() => {
            setShowLeave(false);
            setEditingRecord(null);
            void load();
          }}
        />
      )}
      </>}
    </div>
  );
}

function SummaryCard({ icon: Icon, label, value }: { icon: typeof CalendarDays; label: string; value: number }) {
  return (
    <Card className="group border-[#292929] bg-[#151515] p-3 transition-all duration-300 hover:-translate-y-0.5 hover:border-[#4a8b3f]/50 hover:shadow-[0_10px_26px_rgba(74,139,63,0.10)]">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-xs uppercase tracking-[0.12em] text-[#8d8d8d]">{label}</p>
          <p className="mt-1 text-[21px] font-semibold tracking-tight text-white">{value}</p>
        </div>
        <div className="rounded-xl border border-[#4a8b3f]/30 bg-[#4a8b3f]/10 p-2 text-[#4a8b3f] transition-transform duration-300 group-hover:scale-105">
          <Icon className="h-4 w-4" />
        </div>
      </div>
    </Card>
  );
}

function TabButton({ active, onClick, children }: { active: boolean; onClick: () => void; children: ReactNode }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`${tabClass} ${active ? "bg-[#4a8b3f] text-white shadow-[0_7px_18px_rgba(74,139,63,0.16)]" : "text-[#a6a6a6] hover:bg-[#1b1b1b] hover:text-white"}`}
    >
      {children}
    </button>
  );
}

function RecordTable({ records, busyId, onEdit, onApprove, onReject }: {
  records: LeaveRecord[];
  busyId: number | null;
  onEdit: (record: LeaveRecord) => void;
  onApprove: (id: number) => void;
  onReject: (id: number) => void;
}) {
  return (
    <Card className="leave-table-card overflow-hidden border-[#292929] bg-[#131313] p-0 shadow-[0_12px_34px_rgba(0,0,0,.18)]">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[760px]">
          <thead className="bg-[#161616]">
            <tr className="border-b border-[#2b2b2b] text-left">
              {["Employee", "Leave Type", "Dates", "Days", "Status", "Actions"].map((label) => (
                <th key={label} className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">{label}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {records.map((record) => (
              <tr key={record.id} className="border-b border-[#202020] transition-all duration-200 hover:bg-[#191919]">
                <td className="px-3.5 py-3 text-[12px] font-medium text-white">{record.employee_name}</td>
                <td className="px-3.5 py-3 text-[12px] text-[#d8d8d8]">{record.leave_type_name}</td>
                <td className="px-3.5 py-3 text-[12px] text-[#b6b6b6]">{record.start_date} — {record.end_date}</td>
                <td className="px-3.5 py-3 text-[12px] text-white">{record.days}</td>
                <td className="px-3.5 py-3"><StatusBadge status={record.status} /></td>
                <td className="px-4 py-4">
                  <div className="flex items-center gap-1">
                    <IconButton label="Edit leave" onClick={() => onEdit(record)}><Edit3 className="h-4 w-4" /></IconButton>
                    {record.status === "pending" && (
                      <>
                        <IconButton label="Approve leave" disabled={busyId === record.id} onClick={() => onApprove(record.id)}><Check className="h-4 w-4 text-[#4a8b3f]" /></IconButton>
                        <IconButton label="Reject leave" disabled={busyId === record.id} onClick={() => onReject(record.id)}><X className="h-4 w-4" /></IconButton>
                      </>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {records.length === 0 && <EmptyState title="No leave records" text="Register a leave request to start building the yearly record." />}
    </Card>
  );
}

function HistoryTable({ records }: { records: LeaveRecord[] }) {
  const history = [...records].sort((a, b) => b.created_at.localeCompare(a.created_at));
  const historyYear = history[0]?.start_date?.slice(0, 4) || String(new Date().getFullYear());
  return (
    <Card className="leave-table-card overflow-hidden border-[#292929] bg-[#131313] p-0 shadow-[0_12px_34px_rgba(0,0,0,.18)]">
      <div className="border-b border-[#292929] px-4 py-3.5">
        <div className="flex items-center gap-2 text-white"><History className="h-4 w-4 text-[#4a8b3f]" /> Leave history</div>
        <p className="mt-1 text-sm text-[#888]">All saved leave requests in {historyYear}.</p>
      </div>
      <div className="divide-y divide-[#202020]">
        {history.map((record) => (
          <div key={`history-${record.id}`} className="flex flex-col gap-2 px-5 py-4 transition-colors duration-200 hover:bg-[#171717] md:flex-row md:items-center md:justify-between">
            <div>
              <p className="text-sm font-medium text-white">{record.employee_name} · {record.leave_type_name}</p>
              <p className="mt-1 text-xs text-[#8d8d8d]">{record.start_date} — {record.end_date} · {record.days} day(s)</p>
            </div>
            <StatusBadge status={record.status} />
          </div>
        ))}
        {history.length === 0 && <EmptyState title="No history yet" text="Completed and pending requests will appear here." />}
      </div>
    </Card>
  );
}

function BalanceTable({ balances, year }: { balances: LeaveBalance[]; year: number }) {
  return (
    <Card className="leave-table-card overflow-hidden border-[#292929] bg-[#131313] p-0 shadow-[0_12px_34px_rgba(0,0,0,.18)]">
      <div className="border-b border-[#292929] px-4 py-3.5">
        <p className="text-base font-semibold text-white">Leave balances</p>
        <p className="mt-1 text-sm text-[#888]">Entitlement and usage for {year}.</p>
      </div>
      <div className="overflow-x-auto">
        <table className="w-full min-w-[680px]">
          <thead className="bg-[#161616]">
            <tr className="border-b border-[#2b2b2b] text-left">
              {["Employee", "Leave Type", "Entitled", "Used", "Remaining"].map((label) => (
                <th key={label} className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">{label}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {balances.map((balance) => (
              <tr key={`${balance.employee_id}-${balance.leave_type_id}`} className="border-b border-[#202020] transition-all duration-200 hover:bg-[#191919]">
                <td className="px-3.5 py-3 text-[12px] font-medium text-white">{balance.employee_name}</td>
                <td className="px-3.5 py-3 text-[12px] text-[#d8d8d8]">{balance.leave_type_name}</td>
                <td className="px-3.5 py-3 text-[12px] text-white">{balance.entitled}</td>
                <td className="px-3.5 py-3 text-[12px] text-[#b6b6b6]">{balance.used}</td>
                <td className="px-3.5 py-3 text-[12px] font-semibold text-[#4a8b3f]">{balance.remaining}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {balances.length === 0 && <EmptyState title="No balances yet" text="Balances will appear when employees and leave types are configured." />}
    </Card>
  );
}

function LeaveTypeGrid({ types, onEdit, onDelete }: { types: LeaveType[]; onEdit: (type: LeaveType) => void; onDelete: (id: number) => void }) {
  return (
    <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
      {types.map((type) => (
        <Card key={type.id} className="leave-type-card group border-[#292929] bg-[#131313] p-4 transition-all duration-300 hover:-translate-y-0.5 hover:border-[#4a8b3f]/40 hover:shadow-[0_12px_28px_rgba(74,139,63,0.10)]">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-[14px] font-semibold text-white">{type.name}</p>
              <p className="mt-1 text-xs uppercase tracking-[0.12em] text-[#858585]">{type.code}</p>
            </div>
            <span className="rounded-full border border-[#4a8b3f]/30 bg-[#4a8b3f]/10 px-2 py-0.5 text-[10px] font-medium text-[#4a8b3f]">{type.is_paid ? "Paid" : "Unpaid"}</span>
          </div>
          <div className="mt-4 grid grid-cols-2 gap-2.5 rounded-lg border border-[#242424] bg-[#161616] p-2.5">
            <div><p className="text-[11px] uppercase tracking-[0.08em] text-[#777]">Annual entitlement</p><p className="mt-1 text-lg font-semibold text-white">{type.default_days} days</p></div>
            <div><p className="text-[11px] uppercase tracking-[0.08em] text-[#777]">Carry forward</p><p className="mt-1 text-lg font-semibold text-white">{type.carry_forward ? "Yes" : "No"}</p></div>
          </div>
          <div className="mt-3 flex items-center justify-end gap-1.5">
            <Button variant="secondary" onClick={() => onEdit(type)}><Edit3 className="h-4 w-4" /> Edit</Button>
            <button type="button" onClick={() => onDelete(type.id)} className="inline-flex items-center gap-2 rounded-xl px-3 py-2 text-sm text-[#bcbcbc] transition hover:bg-[#1b1b1b] hover:text-white"><Trash2 className="h-4 w-4" /> Delete</button>
          </div>
        </Card>
      ))}
      {types.length === 0 && <div className="md:col-span-2 xl:col-span-3"><Card><EmptyState title="No leave types configured" text="Create your first leave type to start tracking annual balances." /></Card></div>}
    </div>
  );
}

function LeaveTypeForm({ type, onClose, onSaved }: { type: LeaveType | null; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(type?.name ?? "");
  const [code, setCode] = useState(type?.code ?? "");
  const [days, setDays] = useState(String(type?.default_days ?? ""));
  const [paid, setPaid] = useState(type?.is_paid ?? true);
  const [carryForward, setCarryForward] = useState(type?.carry_forward ?? false);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const save = async () => {
    if (!name.trim() || !code.trim()) return setError("Name and code are required.");
    const numericDays = Number(days);
    if (!Number.isFinite(numericDays) || numericDays < 0) return setError("Entitled days must be zero or greater.");
    setSaving(true);
    setError("");
    try {
      const result = await leaveService.saveType({ id: type?.id, name: name.trim(), code: code.trim().toUpperCase(), default_days: numericDays, is_paid: paid, carry_forward: carryForward });
      if (!result.success) throw new Error(result.message);
      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return <Modal title={type ? "Edit Leave Type" : "Add Leave Type"} subtitle="Define the entitlement and policy used by the leave balance engine." onClose={onClose}>
    <div className="space-y-4">
      <Field label="Leave name"><input className={inputClass} value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Annual Leave" /></Field>
      <Field label="Code"><input className={inputClass} value={code} onChange={(e) => setCode(e.target.value)} placeholder="e.g. ANNUAL" /></Field>
      <Field label="Annual entitlement (days)"><input type="number" min="0" step="0.5" className={inputClass} value={days} onChange={(e) => setDays(e.target.value)} /></Field>
      <label className="flex items-center justify-between rounded-xl border border-[#262626] bg-[#141414] px-3 py-3 text-sm text-white">
        <span>Paid leave</span><input type="checkbox" checked={paid} onChange={(e) => setPaid(e.target.checked)} className="accent-[#4a8b3f]" />
      </label>
      <label className="flex items-center justify-between rounded-xl border border-[#262626] bg-[#141414] px-3 py-3 text-sm text-white">
        <span>Allow carry forward</span><input type="checkbox" checked={carryForward} onChange={(e) => setCarryForward(e.target.checked)} className="accent-[#4a8b3f]" />
      </label>
      {error && <p className="rounded-lg border border-[#4a8b3f]/30 bg-[#4a8b3f]/10 px-3 py-2 text-sm text-white">{error}</p>}
      <div className="flex justify-end gap-2 pt-2"><Button variant="secondary" onClick={onClose}>Cancel</Button><Button onClick={() => void save()} disabled={saving}>{saving ? "Saving…" : "Save Leave Type"}</Button></div>
    </div>
  </Modal>;
}

function LeaveForm({ record, types, employees, onClose, onSaved }: { record: LeaveRecord | null; types: LeaveType[]; employees: Employee[]; onClose: () => void; onSaved: () => void }) {
  const [employeeId, setEmployeeId] = useState(String(record?.employee_id ?? ""));
  const [typeId, setTypeId] = useState(String(record?.leave_type_id ?? ""));
  const [start, setStart] = useState(record?.start_date ?? "");
  const [end, setEnd] = useState(record?.end_date ?? "");
  const [reason, setReason] = useState(record?.reason ?? "");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const days = useMemo(() => {
    if (!start || !end) return 0;
    const diff = (new Date(`${end}T00:00:00`).getTime() - new Date(`${start}T00:00:00`).getTime()) / 86400000;
    return diff >= 0 ? diff + 1 : 0;
  }, [start, end]);

  const save = async () => {
    if (!employeeId || !typeId || !start || !end) return setError("Employee, leave type, start date, and end date are required.");
    if (days <= 0) return setError("End date must be on or after the start date.");
    setSaving(true);
    setError("");
    try {
      const result = await leaveService.saveRecord({
        id: record?.id,
        employee_id: Number(employeeId),
        leave_type_id: Number(typeId),
        start_date: start,
        end_date: end,
        days,
        reason: reason.trim() || null,
      });
      if (!result.success || result.id == null) throw new Error(result.message || "The leave could not be saved.");

      // Verify against the complete record set first. This avoids rejecting a
      // successful save when the stored date format/year filter differs from
      // the form's display value. Then the parent reloads the selected year.
      const savedRecords = await leaveService.getRecords(0);
      const exists = savedRecords.some((item) => Number(item.id) === Number(result.id));
      if (!exists) throw new Error("Leave was saved but could not be verified in the database. Please try again.");

      onSaved();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return <Modal title={record ? "Edit Leave Request" : "Register Leave"} subtitle="Create a clean record that feeds the employee's yearly balance." onClose={onClose} maxWidth="max-w-2xl">
    <div className="grid gap-4 md:grid-cols-2">
      <Field label="Employee"><select className={inputClass} value={employeeId} onChange={(e) => setEmployeeId(e.target.value)}><option value="">Select employee</option>{employees.map((employee) => <option value={employee.id} key={employee.id}>{employee.first_name} {employee.last_name}</option>)}</select></Field>
      <Field label="Leave type"><select className={inputClass} value={typeId} onChange={(e) => setTypeId(e.target.value)}><option value="">Select leave type</option>{types.map((type) => <option value={type.id} key={type.id}>{type.name}</option>)}</select></Field>
      <Field label="Start date"><input type="date" className={inputClass} value={start} onChange={(e) => setStart(e.target.value)} /></Field>
      <Field label="End date"><input type="date" className={inputClass} value={end} onChange={(e) => setEnd(e.target.value)} /></Field>
      <Field label="Calculated days"><div className="mt-1 rounded-xl border border-[#4a8b3f]/25 bg-[#4a8b3f]/10 px-3 py-2.5 text-sm font-semibold text-white">{days} day(s)</div></Field>
      <Field label="Reason"><input className={inputClass} value={reason} onChange={(e) => setReason(e.target.value)} placeholder="Optional" /></Field>
    </div>
    {error && <p className="mt-4 rounded-lg border border-[#4a8b3f]/30 bg-[#4a8b3f]/10 px-3 py-2 text-sm text-white">{error}</p>}
    <div className="mt-5 flex justify-end gap-2"><Button variant="secondary" onClick={onClose}>Cancel</Button><Button onClick={() => void save()} disabled={saving}>{saving ? "Saving…" : record ? "Save Changes" : "Register Leave"}</Button></div>
  </Modal>;
}

function Modal({ title, subtitle, children, onClose, maxWidth = "max-w-md" }: { title: string; subtitle: string; children: ReactNode; onClose: () => void; maxWidth?: string }) {
  return <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 p-4 backdrop-blur-sm animate-[fade-in_.2s_ease-out]" role="dialog" aria-modal="true">
    <div className={`w-full ${maxWidth} animate-[slide-up_.25s_ease-out]`}>
      <Card className="border-[#2b2b2b] bg-[#111111] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.45)]">
        <div className="flex items-start justify-between gap-4">
          <div><h2 className="text-lg font-semibold text-white">{title}</h2><p className="mt-1 text-sm text-[#888]">{subtitle}</p></div>
          <button type="button" aria-label="Close" onClick={onClose} className="rounded-lg p-2 text-[#777] transition hover:bg-[#1b1b1b] hover:text-white"><X className="h-4 w-4" /></button>
        </div>
        <div className="mt-5">{children}</div>
      </Card>
    </div>
  </div>;
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return <label className="block text-xs font-medium uppercase tracking-[0.08em] text-[#8b8b8b]">{label}{children}</label>;
}

function StatusBadge({ status }: { status: string }) {
  const normalized = status.toLowerCase();
  const label = normalized.charAt(0).toUpperCase() + normalized.slice(1);
  const classes = normalized === "approved"
    ? "border-[#4a8b3f]/35 bg-[#4a8b3f]/10 text-[#4a8b3f]"
    : normalized === "pending"
      ? "border-[#545454] bg-[#1a1a1a] text-white"
      : "border-[#3a3a3a] bg-[#181818] text-[#a5a5a5]";
  return <span className={`inline-flex rounded-full border px-2.5 py-1 text-xs font-medium ${classes}`}>{label}</span>;
}

function IconButton({ label, children, onClick, disabled = false }: { label: string; children: ReactNode; onClick: () => void; disabled?: boolean }) {
  return <button type="button" aria-label={label} title={label} disabled={disabled} onClick={onClick} className="rounded-lg p-2 text-[#8e8e8e] transition-all duration-200 hover:bg-[#1b1b1b] hover:text-white disabled:cursor-not-allowed disabled:opacity-40">{children}</button>;
}

function EmptyState({ title, text }: { title: string; text: string }) {
  return <div className="flex flex-col items-center justify-center px-6 py-14 text-center"><Sparkles className="h-5 w-5 text-[#4a8b3f]" /><p className="mt-3 text-sm font-medium text-white">{title}</p><p className="mt-1 max-w-md text-sm text-[#777]">{text}</p></div>;
}
