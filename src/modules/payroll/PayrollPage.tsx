import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { Banknote, Calculator, CheckCircle2, FileText, FlaskConical, History, Landmark, Plus, WalletCards, Table2 } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { payrollService } from "@/services/payroll";
import type { Loan, PayrollPeriod, PayrollRecord, PayrollRule, Payslip } from "@/types/payroll";
import { RuleTable } from "@/components/payroll/RuleTable";
import { RuleEditModal } from "@/components/payroll/RuleEditModal";
import { FormulaTester } from "@/components/payroll/FormulaTester";
import { PreviewPanel } from "@/components/payroll/PreviewPanel";
import { VersionHistoryModal } from "@/components/payroll/VersionHistoryModal";
import { PayrollReportPanel } from "@/components/payroll/PayrollReportPanel";
import { PayslipDocument } from "@/components/payroll/PayslipDocument";
import { staffService } from "@/services/staff";
import type { Employee } from "@/types/staff";

const money = (value:number) => new Intl.NumberFormat(undefined,{minimumFractionDigits:0,maximumFractionDigits:2}).format(value || 0);

type Tab = "run" | "periods" | "history" | "report" | "loans" | "payslips" | "configuration";

export function PayrollPage() {
  const [tab,setTab] = useState<Tab>("run");
  const [periods,setPeriods] = useState<PayrollPeriod[]>([]);
  const [selectedPeriodId,setSelectedPeriodId] = useState<number|null>(null);
  const [records,setRecords] = useState<PayrollRecord[]>([]);
  const [loans,setLoans] = useState<Loan[]>([]);
  const [payslips,setPayslips] = useState<Payslip[]>([]);
  const [rules,setRules] = useState<PayrollRule[]>([]);
  const [employees,setEmployees] = useState<Employee[]>([]);
  const [busy,setBusy] = useState(false);
  const [error,setError] = useState("");
  const [success,setSuccess] = useState("");

  const [editingRule,setEditingRule] = useState<PayrollRule|null>(null);
  const [showRuleModal,setShowRuleModal] = useState(false);
  const [showVersions,setShowVersions] = useState(false);
  const [versionRuleId,setVersionRuleId] = useState<number|null>(null);

  const selectedPeriod = useMemo(() => periods.find(p=>p.id===selectedPeriodId) ?? null,[periods,selectedPeriodId]);

  const loadAll = useCallback(async () => {
    setError("");
    try {
      const [periodRows,loanRows,payslipRows,ruleRows,employeeRows] = await Promise.all([
        payrollService.getPeriods(), payrollService.getLoans(), payrollService.getPayslips(), payrollService.getRules(), staffService.getEmployees(),
      ]);
      setPeriods(periodRows); setLoans(loanRows); setPayslips(payslipRows); setRules(ruleRows.rules); setEmployees(employeeRows);
      const nextId = selectedPeriodId ?? periodRows[0]?.id ?? null;
      setSelectedPeriodId(nextId);
      if(nextId) setRecords(await payrollService.getRecords(nextId)); else setRecords([]);
    } catch(e) { setError(e instanceof Error?e.message:String(e)); }
  },[selectedPeriodId]);

  useEffect(()=>{ void loadAll(); },[loadAll]);

  const calculate = async () => {
    if(!selectedPeriodId) return;
    setBusy(true); setError(""); setSuccess("");
    try { const res=await payrollService.calculatePeriod(selectedPeriodId); setRecords(res.records); if(!res.success) setError(res.errors.join(" ") || res.message); else setSuccess(res.message); await refreshPeriodsOnly(); }
    catch(e){setError(e instanceof Error?e.message:String(e));} finally{setBusy(false);}
  };

  const finalize = async () => {
    if(!selectedPeriodId || !selectedPeriod) return;
    if(!confirm(`Finalize ${selectedPeriod.period_name}? This freezes the payroll and prepares payslips.`)) return;
    setBusy(true); setError(""); setSuccess("");
    try { const res=await payrollService.finalizePeriod(selectedPeriodId); if(!res.success) setError(res.errors.join(" ") || res.message); else {setRecords(res.records);setSuccess(res.message);await loadAll();} }
    catch(e){setError(e instanceof Error?e.message:String(e));} finally{setBusy(false);}
  };

  const refreshPeriodsOnly = async () => setPeriods(await payrollService.getPeriods());

  return (
    <div className="space-y-3 animate-[fade-in_.25s_ease-out]">
      <PageHeader title="Payments / Payroll" description="Prepare, calculate, review and finalize payroll with frozen, traceable results." />
      {!(typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)) && (
        <div className="flex items-start gap-3 rounded-xl border border-[#4a8b3f]/25 bg-[#4a8b3f]/8 px-4 py-3 text-sm text-[#e8e8e8] shadow-[0_8px_24px_rgba(0,0,0,0.14)]">
          <div className="mt-1 h-2.5 w-2.5 shrink-0 rounded-full bg-[#4a8b3f] shadow-[0_0_12px_rgba(74,139,63,0.55)]" />
          <div>
            <p className="font-medium text-white">Preview mode</p>
            <p className="mt-0.5 text-[#9c9c9c]">Payroll changes are kept in this browser session. The Tauri desktop app uses the secure SQLite database.</p>
          </div>
        </div>
      )}

      <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-5">
        <Kpi icon={Banknote} label="Payroll periods" value={periods.length} />
        <Kpi icon={WalletCards} label="Employees" value={records.length || employees.filter(e=>e.is_active).length} />
        <Kpi icon={Calculator} label="Gross" value={money(selectedPeriod?.total_gross ?? records.reduce((s,r)=>s+r.gross_earnings,0))} />
        <Kpi icon={FileText} label="Net pay" value={money(selectedPeriod?.total_net ?? records.reduce((s,r)=>s+r.net_pay,0))} />
        <Kpi icon={Landmark} label="Employer contrib." value={money(selectedPeriod?.employer_contributions ?? records.reduce((s,r)=>s+r.employer_contributions,0))} />
      </div>

      <div className="rounded-2xl border border-[#2b2b2b] bg-[#121212] p-2 shadow-[0_16px_45px_rgba(0,0,0,0.25)]">
        <div className="flex flex-wrap gap-2">
          <Tab active={tab==="run"} onClick={()=>setTab("run")} icon={Calculator} label="Run Payroll" />
          <Tab active={tab==="periods"} onClick={()=>setTab("periods")} icon={History} label="Payroll Periods" />
          <Tab active={tab==="history"} onClick={()=>setTab("history")} icon={CheckCircle2} label="Payroll History" />
          <Tab active={tab==="report"} onClick={()=>setTab("report")} icon={Table2} label="Payroll Report" />
          <Tab active={tab==="loans"} onClick={()=>setTab("loans")} icon={WalletCards} label="Loans / Advances" />
          <Tab active={tab==="payslips"} onClick={()=>setTab("payslips")} icon={FileText} label="Payslips" />
          <Tab active={tab==="configuration"} onClick={()=>setTab("configuration")} icon={FlaskConical} label="Configuration" />
        </div>
      </div>

      {(error || success) && <div className={`rounded-xl border px-4 py-3 text-sm ${error ? "border-[#4a8b3f]/50 bg-[#4a8b3f]/10 text-white" : "border-[#4a8b3f]/50 bg-[#4a8b3f]/10 text-white"}`}>{error || success}</div>}

      {tab==="run" && <RunPanel periods={periods} selectedPeriod={selectedPeriod} selectedPeriodId={selectedPeriodId} setSelectedPeriodId={setSelectedPeriodId} records={records} calculate={calculate} finalize={finalize} busy={busy} />}
      {tab==="periods" && <PeriodsPanel periods={periods} selectedPeriodId={selectedPeriodId} onSelect={(id)=>{setSelectedPeriodId(id);void payrollService.getRecords(id).then(setRecords)}} onSaved={loadAll} />}
      {tab==="history" && <HistoryPanel periods={periods.filter(p=>p.status==="closed"||p.status==="locked")} onSelect={(id)=>{setSelectedPeriodId(id);void payrollService.getRecords(id).then(setRecords);setTab("run");}} />}
      {tab==="report" && <PayrollReportPanel period={selectedPeriod} records={records} />}
      {tab==="loans" && <LoansPanel loans={loans} employees={employees} onSaved={loadAll} />}
      {tab==="payslips" && <PayslipPanel payslips={payslips} periods={periods} />}
      {tab==="configuration" && <ConfigurationPanel rules={rules} setRules={setRules} editingRule={editingRule} setEditingRule={setEditingRule} showModal={showRuleModal} setShowModal={setShowRuleModal} showVersions={showVersions} setShowVersions={setShowVersions} versionRuleId={versionRuleId} setVersionRuleId={setVersionRuleId} />}
    </div>
  );
}

function Kpi({icon:Icon,label,value}:{icon:typeof Banknote;label:string;value:string|number}){return <Card className="border-[#2b2b2b] bg-[#121212] px-2.5 py-1.5 transition-all duration-300 hover:-translate-y-0.5 hover:border-[#4a8b3f]/50"><div className="flex items-center gap-2"><div className="rounded-md bg-[#4a8b3f]/15 p-1 text-[#6fa765]"><Icon className="h-3.5 w-3.5"/></div><div className="min-w-0"><p className="text-[8px] uppercase tracking-[0.1em] text-[#808080]">{label}</p><p className="mt-0.5 truncate text-xs font-semibold text-white">{value}</p></div></div></Card>}
function Tab({active,onClick,icon:Icon,label}:{active:boolean;onClick:()=>void;icon:typeof Banknote;label:string}){return <button onClick={onClick} className={`inline-flex items-center gap-2 rounded-xl px-3 py-2 text-sm font-medium transition-all duration-200 ${active?"bg-[#4a8b3f] text-white shadow-[0_8px_20px_rgba(74,139,63,.18)]":"bg-[#1b1b1b] text-[#9a9a9a] hover:bg-[#242424] hover:text-white"}`}><Icon className="h-4 w-4"/>{label}</button>}

function RunPanel({periods,selectedPeriod,selectedPeriodId,setSelectedPeriodId,records,calculate,finalize,busy}:{periods:PayrollPeriod[];selectedPeriod:PayrollPeriod|null;selectedPeriodId:number|null;setSelectedPeriodId:(id:number|null)=>void;records:PayrollRecord[];calculate:()=>Promise<void>;finalize:()=>Promise<void>;busy:boolean}){
  const canFinalize=!!selectedPeriod && selectedPeriod.status!=="closed" && selectedPeriod.status!=="locked" && records.length>0;
  const status=selectedPeriod?.status??"not selected";
  const statusLabel=status==="processing"?"Calculated / review":status;
  return <div className="space-y-3">
    <Card className="border-[#2b2b2b] bg-[#121212] p-4 shadow-[0_12px_32px_rgba(0,0,0,.18)] transition-all duration-200">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
        <div className="min-w-0 flex-1"><div className="mb-1.5 flex items-center gap-2"><div className="rounded-lg bg-[#4a8b3f]/12 p-1.5 text-[#75ad6b]"><Calculator className="h-4 w-4"/></div><div><h3 className="text-sm font-semibold text-white">Run payroll</h3><p className="text-[10px] text-[#777]">Select a period, calculate, review the frozen results, then finalize.</p></div></div>
          <select value={selectedPeriodId??""} onChange={e=>setSelectedPeriodId(Number(e.target.value)||null)} className="mt-2 w-full rounded-xl border border-[#2b2b2b] bg-[#0d0d0d] px-3 py-2.5 text-xs text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/15">{periods.length===0?<option value="">Create a payroll period first</option>:periods.map(p=><option key={p.id} value={p.id}>{p.period_name} · {p.start_date} → {p.end_date} · {p.status}</option>)}</select>
        </div>
        <div className="flex flex-wrap items-center gap-2"><span className="rounded-full border border-[#2b2b2b] bg-[#1a1a1a] px-2.5 py-1.5 text-[10px] text-[#888]">Status: <b className="text-[#bbb]">{statusLabel}</b></span><Button className="h-8 px-3 text-xs" onClick={()=>void calculate()} disabled={!selectedPeriodId||busy}><Calculator className="h-3.5 w-3.5"/>{busy?"Calculating…":"Calculate"}</Button><Button variant="secondary" className="h-8 px-3 text-xs" onClick={()=>void finalize()} disabled={!canFinalize||busy}><CheckCircle2 className="h-3.5 w-3.5"/>Finalize & payslips</Button></div>
      </div>
    </Card>
    <RecordsTable records={records}/>
  </div>
}

function RecordsTable({records}:{records:PayrollRecord[]}){
 return <Card className="overflow-hidden border-[#2b2b2b] bg-[#121212] shadow-[0_12px_32px_rgba(0,0,0,.16)]">
  <div className="flex items-center justify-between gap-3 border-b border-[#242424] px-4 py-3"><div><h3 className="text-sm font-semibold text-white">Calculation review</h3><p className="mt-0.5 text-[10px] text-[#777]">Every row is a frozen snapshot from the payroll engine.</p></div><span className="rounded-full bg-[#1b1b1b] px-2 py-1 text-[9px] text-[#777]">{records.length} employees</span></div>
  {records.length===0?<div className="px-5 py-12 text-center text-xs text-[#666]">No payroll records yet. Select a period and run Calculate.</div>:<div className="max-h-[52vh] overflow-auto"><table className="w-full min-w-[760px] text-left text-[11px]"><thead className="sticky top-0 z-10 bg-[#171717] text-[9px] uppercase tracking-[.08em] text-[#777]"><tr><th className="px-3 py-2.5">Employee</th><th className="px-3 py-2.5 text-right">Basic</th><th className="px-3 py-2.5 text-right">Gross / Base</th><th className="px-3 py-2.5 text-right">Tax</th><th className="px-3 py-2.5 text-right">Deductions</th><th className="px-3 py-2.5 text-right">Net</th><th className="px-3 py-2.5 text-right">Employer</th><th className="px-3 py-2.5">Status</th></tr></thead><tbody className="divide-y divide-[#242424]">{records.map(r=><tr key={r.id} className="transition-colors duration-150 hover:bg-[#181818]"><td className="px-3 py-2.5"><div className="font-medium text-white">{r.employee_name}</div><div className="text-[9px] text-[#666]">{r.employee_code}</div></td><td className="px-3 py-2.5 text-right tabular-nums text-[#ccc]">{money(r.base_salary)}</td><td className="px-3 py-2.5 text-right tabular-nums text-[#ccc]">{money(r.gross_earnings)}</td><td className="px-3 py-2.5 text-right tabular-nums text-[#ccc]">{money(r.total_tax)}</td><td className="px-3 py-2.5 text-right tabular-nums text-[#ccc]">{money(r.total_deductions)}</td><td className="px-3 py-2.5 text-right font-semibold tabular-nums text-[#7fba75]">{money(r.net_pay)}</td><td className="px-3 py-2.5 text-right tabular-nums text-[#ccc]">{money(r.employer_contributions)}</td><td className="px-3 py-2.5"><span className="rounded-full border border-[#4a8b3f]/35 bg-[#4a8b3f]/10 px-2 py-1 text-[9px] text-[#86b67d]">{r.status}</span></td></tr>)}</tbody></table></div>}
 </Card>
}
function PeriodsPanel({periods,selectedPeriodId,onSelect,onSaved}:{periods:PayrollPeriod[];selectedPeriodId:number|null;onSelect:(id:number)=>void;onSaved:()=>Promise<void>}){const [form,setForm]=useState({period_name:"",start_date:"",end_date:"",pay_date:""});const [busy,setBusy]=useState(false);const save=async()=>{setBusy(true);const res=await payrollService.savePeriod({...form,pay_date:form.pay_date||null});setBusy(false);if(!res.success){alert(res.message);return;}setForm({period_name:"",start_date:"",end_date:"",pay_date:""});await onSaved();};return <div className="grid gap-4 xl:grid-cols-[360px_1fr]">
  <Card className="border-[#2b2b2b] bg-[#121212] p-5"><h3 className="font-semibold text-white">Create payroll period</h3><div className="mt-4 space-y-3"><Field label="Period name"><input value={form.period_name} onChange={e=>setForm({...form,period_name:e.target.value})} placeholder="August 2026" className={input}/></Field><Field label="Start date"><input type="date" value={form.start_date} onChange={e=>setForm({...form,start_date:e.target.value})} className={input}/></Field><Field label="End date"><input type="date" value={form.end_date} onChange={e=>setForm({...form,end_date:e.target.value})} className={input}/></Field><Field label="Pay date"><input type="date" value={form.pay_date} onChange={e=>setForm({...form,pay_date:e.target.value})} className={input}/></Field><Button onClick={save} disabled={busy||!form.period_name||!form.start_date||!form.end_date}><Plus className="h-4 w-4"/> {busy?"Saving...":"Create Period"}</Button></div></Card>
  <Card className="overflow-hidden border-[#2b2b2b] bg-[#121212]"><div className="border-b border-[#242424] px-5 py-4"><h3 className="font-semibold text-white">Payroll periods</h3></div><div className="divide-y divide-[#242424]">{periods.map(p=><button key={p.id} onClick={()=>onSelect(p.id)} className={`flex w-full items-center justify-between gap-4 px-5 py-4 text-left transition-colors hover:bg-[#171717] ${selectedPeriodId===p.id?"bg-[#4a8b3f]/10":""}`}><div><div className="font-medium text-white">{p.period_name}</div><div className="mt-1 text-xs text-[#777]">{p.start_date} → {p.end_date} {p.pay_date?`· Pay date ${p.pay_date}`:""}</div></div><div className="text-right"><div className="text-sm font-semibold text-[#76b36b]">{money(p.total_net)}</div><div className="text-xs text-[#777]">{p.record_count} employees · {p.status}</div></div></button>)}{periods.length===0&&<div className="px-5 py-12 text-center text-sm text-[#777]">No payroll periods yet.</div>}</div></Card>
</div>}

function HistoryPanel({periods,onSelect}:{periods:PayrollPeriod[];onSelect:(id:number)=>void}){return <Card className="border-[#2b2b2b] bg-[#121212] p-5"><h3 className="font-semibold text-white">Finalized payroll history</h3><div className="mt-4 space-y-2">{periods.map(p=><button key={p.id} onClick={()=>onSelect(p.id)} className="flex w-full items-center justify-between rounded-xl border border-[#242424] bg-[#151515] px-4 py-3 text-left transition hover:border-[#4a8b3f]/50"><div><div className="font-medium text-white">{p.period_name}</div><div className="text-xs text-[#777]">{p.start_date} → {p.end_date}</div></div><div className="text-right"><div className="font-semibold text-[#76b36b]">{money(p.total_net)}</div><div className="text-xs text-[#777]">{p.status}</div></div></button>)}{periods.length===0&&<div className="py-12 text-center text-sm text-[#777]">No finalized payroll yet.</div>}</div></Card>}

function LoansPanel({loans,employees,onSaved}:{loans:Loan[];employees:Employee[];onSaved:()=>Promise<void>}){const [employeeId,setEmployeeId]=useState<number>(employees[0]?.id??0);const [principal,setPrincipal]=useState("");const [interest,setInterest]=useState("0");const [total,setTotal]=useState("");const [installment,setInstallment]=useState("");const [count,setCount]=useState("");const [start,setStart]=useState("");const save=async()=>{const res=await payrollService.saveLoan({employee_id:employeeId,principal:Number(principal),interest_rate:Number(interest),total_amount:Number(total||principal),installment_amount:Number(installment),total_installments:Number(count),start_date:start});if(!res.success)alert(res.message);else{setPrincipal("");setTotal("");setInstallment("");setCount("");await onSaved();}};return <div className="grid gap-4 xl:grid-cols-[390px_1fr]"><Card className="border-[#2b2b2b] bg-[#121212] p-5"><h3 className="font-semibold text-white">Loan / advance</h3><div className="mt-4 space-y-3"><Field label="Employee"><select value={employeeId} onChange={e=>setEmployeeId(Number(e.target.value))} className={input}>{employees.map(e=><option key={e.id} value={e.id}>{e.employee_code} · {e.first_name} {e.last_name}</option>)}</select></Field><Field label="Principal"><input value={principal} onChange={e=>setPrincipal(e.target.value)} type="number" className={input}/></Field><Field label="Interest rate"><input value={interest} onChange={e=>setInterest(e.target.value)} type="number" className={input}/></Field><Field label="Total amount"><input value={total} onChange={e=>setTotal(e.target.value)} type="number" className={input}/></Field><Field label="Installment"><input value={installment} onChange={e=>setInstallment(e.target.value)} type="number" className={input}/></Field><Field label="Installments"><input value={count} onChange={e=>setCount(e.target.value)} type="number" className={input}/></Field><Field label="Start date"><input value={start} onChange={e=>setStart(e.target.value)} type="date" className={input}/></Field><Button onClick={save} disabled={!employeeId||!principal||!installment||!count||!start}><Plus className="h-4 w-4"/> Save Loan</Button></div></Card><Card className="overflow-hidden border-[#2b2b2b] bg-[#121212]"><div className="border-b border-[#242424] px-5 py-4"><h3 className="font-semibold text-white">Loans & advances</h3></div><div className="overflow-x-auto"><table className="w-full text-sm"><thead className="bg-[#171717] text-xs uppercase tracking-wide text-[#777]"><tr><th className="px-4 py-3 text-left">Employee</th><th className="px-4 py-3 text-left">Principal</th><th className="px-4 py-3 text-left">Installment</th><th className="px-4 py-3 text-left">Progress</th><th className="px-4 py-3 text-left">Remaining</th><th className="px-4 py-3 text-left">Status</th></tr></thead><tbody className="divide-y divide-[#242424]">{loans.map(l=><tr key={l.id} className="hover:bg-[#171717]"><td className="px-4 py-3 text-white">{l.employee_name}</td><td className="px-4 py-3">{money(l.principal)}</td><td className="px-4 py-3">{money(l.installment_amount)}</td><td className="px-4 py-3">{l.paid_installments}/{l.total_installments}</td><td className="px-4 py-3 text-[#76b36b]">{money(l.remaining_amount)}</td><td className="px-4 py-3">{l.status}</td></tr>)}</tbody></table></div></Card></div>}

function PayslipPanel({ payslips, periods }: { payslips: Payslip[]; periods: PayrollPeriod[] }) {
  const [selected, setSelected] = useState<Payslip | null>(null);
  const selectedPeriod = selected ? periods.find((p) => p.period_name === selected.period_name) ?? null : null;
  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(360px,1fr)_minmax(520px,760px)]">
      <Card className="overflow-hidden border-[#2b2b2b] bg-[#121212]">
        <div className="border-b border-[#242424] px-5 py-4">
          <h3 className="font-semibold text-white">Payslips</h3>
          <p className="mt-1 text-sm text-[#777]">Select a payslip to preview, print, or save as PDF.</p>
        </div>
        <div className="divide-y divide-[#242424]">
          {payslips.map((p) => (
            <button key={p.id} type="button" onClick={() => setSelected(p)} className={`flex w-full items-center justify-between px-5 py-4 text-left transition hover:bg-[#171717] ${selected?.id === p.id ? "bg-[#1a1f19]" : ""}`}>
              <div className="min-w-0">
                <div className="truncate font-medium text-white">{p.employee_name}</div>
                <div className="text-xs text-[#777]">{p.employee_code} · {p.period_name} · {p.payslip_number}</div>
              </div>
              <div className="ml-4 shrink-0 font-semibold text-[#76b36b]">{money(p.net_pay)}</div>
            </button>
          ))}
          {payslips.length === 0 && <div className="px-5 py-12 text-center text-sm text-[#777]">No payslips yet.</div>}
        </div>
      </Card>
      {selected ? <PayslipDocument payslip={selected} period={selectedPeriod} /> : <Card className="flex min-h-[520px] items-center justify-center border-[#2b2b2b] bg-[#121212] p-8 text-center text-sm text-[#777]">Select a payslip to preview and print.</Card>}
    </div>
  );
}

function ConfigurationPanel({rules,setRules,editingRule,setEditingRule,showModal,setShowModal,showVersions,setShowVersions,versionRuleId,setVersionRuleId}:{rules:PayrollRule[];setRules:(v:PayrollRule[])=>void;editingRule:PayrollRule|null;setEditingRule:(v:PayrollRule|null)=>void;showModal:boolean;setShowModal:(v:boolean)=>void;showVersions:boolean;setShowVersions:(v:boolean)=>void;versionRuleId:number|null;setVersionRuleId:(v:number|null)=>void}){const refresh=async()=>{const res=await payrollService.getRules();setRules(res.rules)};return <div className="space-y-4"><div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between"><div><h3 className="text-base font-semibold text-white">Dynamic payroll rules</h3><p className="text-[10px] text-[#777]">Rates, formulas, order and versions remain editable by the Admin.</p></div><Button className="h-8 px-3 text-xs" onClick={()=>{setEditingRule(null);setShowModal(true)}}><Plus className="h-3.5 w-3.5"/> Add Rule</Button></div><RuleTable rules={rules} onEdit={r=>{setEditingRule(r);setShowModal(true)}} onToggle={async r=>{await payrollService.toggleRule(r.id,!r.is_active);await refresh()}} onDelete={async r=>{if(confirm(`Delete rule "${r.name}"?`)){await payrollService.deleteRule(r.id);await refresh()}}} onShowVersions={r=>{setVersionRuleId(r.id);setShowVersions(true)}} />{showModal&&<RuleEditModal rule={editingRule} rules={rules} onClose={()=>setShowModal(false)} onSaved={()=>{setShowModal(false);void refresh();}}/>}{showVersions&&versionRuleId!==null&&<VersionHistoryModal ruleId={versionRuleId} onClose={()=>setShowVersions(false)}/>}<div className="grid gap-3 xl:grid-cols-2"><FormulaTester rules={rules}/><PreviewPanel rules={rules}/></div></div>}

function Field({label,children}:{label:string;children:ReactNode}){return <label className="block text-sm text-[#bdbdbd]">{label}{children}</label>}
const input="mt-1 w-full rounded-xl border border-[#2a2a2a] bg-[#0f0f0f] px-3 py-2.5 text-sm text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/20";
