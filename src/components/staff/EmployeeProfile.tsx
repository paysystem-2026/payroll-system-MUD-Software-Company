import { useEffect, useState } from "react";
import type { Employee, SalaryRecord, AddSalaryRequest, EmployeePayrollOverride, EmployeePayrollOverrideRequest } from "@/types/staff";
import type { PayrollRule } from "@/types/payroll";
import { staffService } from "@/services/staff";
import { payrollService } from "@/services/payroll";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { ArrowLeft, UserCircle, Plus, X, SlidersHorizontal, Trash2 } from "lucide-react";

interface EmployeeProfileProps { employee: Employee; onClose: () => void; }
const inputCls = "w-full rounded-xl border border-[#2e2e2e] bg-[#0d0d0d] px-3 py-2 text-[13px] text-[#e8e8e8] focus:outline-none focus:border-[#4a8b3f]";

export function EmployeeProfile({ employee, onClose }: EmployeeProfileProps) {
  const [salaryHistory, setSalaryHistory] = useState<SalaryRecord[]>([]);
  const [rules, setRules] = useState<PayrollRule[]>([]);
  const [overrides, setOverrides] = useState<EmployeePayrollOverride[]>([]);
  const [showSalaryForm, setShowSalaryForm] = useState(false);
  const [showOverrideForm, setShowOverrideForm] = useState(false);
  const [salary, setSalary] = useState("");
  const [effectiveDate, setEffectiveDate] = useState("");
  const [reason, setReason] = useState("");
  const [ruleId, setRuleId] = useState("");
  const [overrideType, setOverrideType] = useState<"fixed"|"percentage"|"formula">("fixed");
  const [overrideValue, setOverrideValue] = useState("");
  const [overrideFormula, setOverrideFormula] = useState("");
  const [overrideBase, setOverrideBase] = useState("");
  const [overrideDate, setOverrideDate] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const load = async () => {
    const [salaryData, ruleData, overrideData] = await Promise.all([
      staffService.getSalaryHistory(employee.id),
      payrollService.getRules(),
      staffService.getEmployeePayrollOverrides(employee.id),
    ]);
    setSalaryHistory(salaryData);
    setRules(ruleData.rules.filter(r => r.is_active));
    setOverrides(overrideData);
  };
  useEffect(() => { void load(); }, [employee.id]);

  const currentSalary = salaryHistory[0]?.base_salary;

  const handleAddSalary = async () => {
    setError("");
    if (!salary || Number(salary) < 0) return setError("Enter a valid salary.");
    if (!effectiveDate) return setError("Effective date is required.");
    setSaving(true);
    try {
      const req: AddSalaryRequest = { employee_id: employee.id, base_salary: Number(salary), effective_date: effectiveDate, reason: reason || null };
      const res = await staffService.addSalaryRecord(req);
      if (!res.success) setError(res.message); else { setSalary(""); setEffectiveDate(""); setReason(""); setShowSalaryForm(false); await load(); }
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setSaving(false); }
  };

  const handleSaveOverride = async () => {
    setError("");
    const selectedRule = rules.find(r => String(r.id) === ruleId);
    if (!selectedRule) return setError("Select a payroll component.");
    if (!overrideDate) return setError("Effective date is required.");
    if (overrideType !== "formula" && (overrideValue === "" || Number.isNaN(Number(overrideValue)))) return setError("Enter a valid value.");
    if (overrideType === "formula" && !overrideFormula.trim()) return setError("Enter a formula.");
    setSaving(true);
    try {
      const req: EmployeePayrollOverrideRequest = {
        employee_id: employee.id, rule_id: selectedRule.id, override_type: overrideType,
        value: overrideType === "formula" ? null : Number(overrideValue),
        formula_expression: overrideType === "formula" ? overrideFormula.trim() : null,
        base_reference: overrideBase || selectedRule.base_reference || null,
        effective_date: overrideDate, is_active: true,
      };
      const res = await staffService.saveEmployeePayrollOverride(req);
      if (!res.success) setError(res.message); else { setRuleId(""); setOverrideValue(""); setOverrideFormula(""); setOverrideBase(""); setOverrideDate(""); setShowOverrideForm(false); await load(); }
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setSaving(false); }
  };

  const removeOverride = async (override: EmployeePayrollOverride) => {
    if (!confirm(`Remove ${override.rule_name} override for this employee?`)) return;
    await staffService.deleteEmployeePayrollOverride(override.id);
    await load();
  };

  return (
    <div className="dashboard-reveal">
      <button onClick={onClose} className="mb-4 inline-flex items-center gap-2 text-[13px] text-[#888888] transition hover:text-white"><ArrowLeft className="h-4 w-4" /> Back to Staff Records</button>
      <div className="mb-6 flex items-start justify-between">
        <div className="flex items-center gap-4">
          <div className="h-14 w-14 overflow-hidden rounded-full border border-[#303030] bg-[#202020]">{employee.photo_path ? <img src={employee.photo_path} alt="Employee" className="h-full w-full object-cover" /> : <div className="grid h-full w-full place-items-center"><UserCircle className="h-8 w-8 text-[#4a8b3f]" /></div>}</div>
          <div><h1 className="text-[20px] font-semibold text-white">{employee.first_name} {employee.last_name}</h1><p className="mt-1 font-mono text-[13px] text-[#888888]">{employee.employee_code}</p></div>
        </div>
        <span className={`rounded-full px-3 py-1 text-[12px] font-medium ${employee.employment_status === "active" ? "bg-[#1e3a1a] text-[#6fa966]" : "bg-[#2a2a2a] text-[#8c8c8c]"}`}>{employee.employment_status}</span>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card><h2 className="mb-4 text-[14px] font-semibold text-white">Personal Information</h2><div className="grid grid-cols-2 gap-x-4 gap-y-3"><Info label="Full Name" value={`${employee.first_name} ${employee.last_name}`} /><Info label="Gender" value={employee.gender} /><Info label="Date of Birth" value={employee.date_of_birth} /><Info label="Dependants" value={String(employee.dependants)} /><Info label="National ID" value={employee.national_id} /><Info label="RSSB Number" value={employee.rssb_number} /><Info label="Phone" value={employee.phone} /><Info label="Email" value={employee.email} /><div className="col-span-2"><Info label="Address" value={employee.address} /></div></div></Card>
        <Card><h2 className="mb-4 text-[14px] font-semibold text-white">Employment Information</h2><div className="grid grid-cols-2 gap-x-4 gap-y-3"><Info label="Department" value={employee.department_name} /><Info label="Position" value={employee.position_title} /><Info label="Contract Type" value={employee.contract_type_name || "Not assigned"} /><Info label="Grade" value={employee.grade} /><Info label="Date of Start" value={employee.hire_date} /><Info label="Bank Name" value={employee.bank_name} /><Info label="Account Number" value={employee.bank_account} /><Info label="Current Basic Salary" value={currentSalary !== undefined ? currentSalary.toLocaleString() : "No salary record"} highlight /></div></Card>
      </div>

      <Card className="mt-4">
        <div className="mb-4 flex items-center justify-between"><div><h2 className="text-[14px] font-semibold text-white">Employee Payroll Components</h2><p className="mt-1 text-[12px] text-[#888888]">Set employee-specific amounts or formulas without changing the global payroll rules.</p></div><Button onClick={() => setShowOverrideForm(true)}><SlidersHorizontal className="h-4 w-4" /> Add Component</Button></div>
        {showOverrideForm && <div className="mb-4 rounded-2xl border border-[#2e2e2e] bg-[#141414] p-4"><div className="mb-3 flex items-center justify-between"><h3 className="text-[13px] font-medium text-white">Employee-specific payroll component</h3><button onClick={() => setShowOverrideForm(false)} className="text-[#777] hover:text-white"><X className="h-4 w-4" /></button></div><div className="grid gap-3 md:grid-cols-4"><Field label="Component"><select value={ruleId} onChange={e=>setRuleId(e.target.value)} className={inputCls}><option value="">Select component</option>{rules.map(r=><option key={r.id} value={r.id}>{r.name} ({r.code})</option>)}</select></Field><Field label="Mode"><select value={overrideType} onChange={e=>setOverrideType(e.target.value as any)} className={inputCls}><option value="fixed">Fixed amount</option><option value="percentage">Percentage</option><option value="formula">Formula</option></select></Field><Field label={overrideType === "formula" ? "Formula" : "Value"}>{overrideType === "formula" ? <input value={overrideFormula} onChange={e=>setOverrideFormula(e.target.value)} className={inputCls} placeholder="e.g. BASIC * 0.15"/> : <input type="number" step="0.01" value={overrideValue} onChange={e=>setOverrideValue(e.target.value)} className={inputCls} placeholder="0.00"/>}</Field><Field label="Effective Date"><input type="date" value={overrideDate} onChange={e=>setOverrideDate(e.target.value)} className={`${inputCls} [color-scheme:dark]`} /></Field></div><div className="mt-3 flex items-center justify-between gap-3"><p className="text-[11px] text-[#777]">Leave base reference empty to keep the global base.</p><Button onClick={handleSaveOverride} disabled={saving}>{saving ? "Saving…" : "Save Component"}</Button></div></div>}
        {overrides.length === 0 ? <p className="py-6 text-center text-[13px] text-[#777]">No employee-specific components. Global rules will apply.</p> : <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{overrides.map(o=><div key={o.id} className="rounded-2xl border border-[#2b2b2b] bg-[#181818] p-4 transition hover:-translate-y-0.5 hover:border-[#4a8b3f]"><div className="flex items-start justify-between gap-3"><div><div className="text-[13px] font-semibold text-white">{o.rule_name}</div><div className="mt-1 font-mono text-[11px] text-[#666]">{o.rule_code}</div></div><button onClick={()=>removeOverride(o)} className="rounded-lg p-1.5 text-[#777] hover:bg-[#222] hover:text-white" title="Remove override"><Trash2 className="h-4 w-4"/></button></div><div className="mt-4 grid grid-cols-2 gap-3"><Info label="Mode" value={o.override_type}/><Info label="Value" value={o.override_type === "formula" ? o.formula_expression : `${o.value ?? 0}${o.override_type === "percentage" ? "%" : ""}`} /><Info label="Base" value={o.base_reference || "Global base"}/><Info label="Effective" value={o.effective_date}/></div></div>)}</div>}
      </Card>

      <Card className="mt-4">
        <div className="mb-4 flex items-center justify-between"><div><h2 className="text-[14px] font-semibold text-white">Salary History</h2><p className="mt-1 text-[12px] text-[#888888]">Every salary change is preserved with its effective date and reason.</p></div><Button onClick={() => setShowSalaryForm(true)}><Plus className="h-4 w-4" /> Add Salary Change</Button></div>
        {showSalaryForm && <div className="mb-4 rounded-xl border border-[#2e2e2e] bg-[#242424] p-4"><div className="grid grid-cols-3 gap-3"><Field label="Basic Salary"><input type="number" min="0" step="0.01" value={salary} onChange={e=>setSalary(e.target.value)} className={inputCls}/></Field><Field label="Effective Date"><input type="date" value={effectiveDate} onChange={e=>setEffectiveDate(e.target.value)} className={`${inputCls} [color-scheme:dark]`}/></Field><Field label="Reason"><input value={reason} onChange={e=>setReason(e.target.value)} className={inputCls} placeholder="Promotion / adjustment"/></Field></div><div className="mt-3 flex justify-end"><Button onClick={handleAddSalary} disabled={saving}>{saving?"Saving…":"Save Salary Record"}</Button></div></div>}
        {error && <div className="mb-4 rounded-xl border border-[#353535] bg-[#202020] px-4 py-3 text-[12px] text-white">{error}</div>}
        {salaryHistory.length === 0 ? <p className="py-6 text-center text-[13px] text-[#777]">No salary history recorded.</p> : <table className="w-full"><thead><tr className="border-b border-[#2e2e2e] text-left"><th className="px-3 py-2 text-[11px] uppercase tracking-wider text-[#888]">Effective Date</th><th className="px-3 py-2 text-[11px] uppercase tracking-wider text-[#888]">Basic Salary</th><th className="px-3 py-2 text-[11px] uppercase tracking-wider text-[#888]">Reason</th><th className="px-3 py-2 text-[11px] uppercase tracking-wider text-[#888]">Recorded</th></tr></thead><tbody>{salaryHistory.map(r=><tr key={r.id} className="border-b border-[#222]"><td className="px-3 py-3 text-[13px] text-white">{r.effective_date}</td><td className="px-3 py-3 text-[13px] font-medium text-[#4a8b3f]">{r.base_salary.toLocaleString()}</td><td className="px-3 py-3 text-[13px] text-[#888]">{r.reason ?? "—"}</td><td className="px-3 py-3 text-[12px] text-[#888]">{r.created_at}</td></tr>)}</tbody></table>}
      </Card>
    </div>
  );
}
function Info({ label, value, highlight }: { label:string; value:string|null|undefined; highlight?:boolean }) { return <div><p className="text-[11px] uppercase tracking-wider text-[#888]">{label}</p><p className={`mt-0.5 text-[13px] ${highlight?"font-semibold text-[#4a8b3f]":"text-white"}`}>{value || "—"}</p></div>; }
function Field({ label, children }: { label:string; children:React.ReactNode }) { return <div><label className="mb-1.5 block text-[12px] font-medium text-white">{label}</label>{children}</div>; }
