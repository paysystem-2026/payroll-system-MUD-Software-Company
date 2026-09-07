import { useMemo, useState } from "react";
import type { PayrollRule, CalcResult } from "@/types/payroll";
import { payrollService } from "@/services/payroll";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { Calculator, AlertCircle, RotateCcw } from "lucide-react";

interface PreviewPanelProps { rules: PayrollRule[]; }

const money=(s:string)=>{const n=Number(s);return Number.isFinite(n)?new Intl.NumberFormat("en-RW",{minimumFractionDigits:0,maximumFractionDigits:2}).format(n):s;};

export function PreviewPanel({rules}:PreviewPanelProps){
 const active=useMemo(()=>rules.filter(r=>r.is_active),[rules]);
 const [basicSalary,setBasicSalary]=useState("100000");
 const [result,setResult]=useState<CalcResult|null>(null);
 const [loading,setLoading]=useState(false);
 const preview=async()=>{const value=Number(basicSalary);if(!Number.isFinite(value)||value<0)return;setLoading(true);try{setResult(await payrollService.previewCalculation(value));}catch(e){setResult({items:[],gross_earnings:"0",total_deductions:"0",total_tax:"0",net_pay:"0",employer_contributions:"0",errors:[e instanceof Error?e.message:String(e)]});}finally{setLoading(false);}};
 const reset=()=>{setBasicSalary("100000");setResult(null);};
 return <div className="space-y-3">
  <Card className="border-[#2b2b2b] bg-[#121212] p-4 transition-all duration-200 hover:border-[#353535]">
   <div className="flex items-start justify-between gap-3"><div className="flex items-center gap-2"><div className="rounded-lg bg-[#4a8b3f]/12 p-2 text-[#75ad6b]"><Calculator className="h-4 w-4"/></div><div><h3 className="text-sm font-semibold text-white">Calculation Preview</h3><p className="text-[10px] text-[#777]">Run the active payroll rules against a sample salary.</p></div></div><button type="button" onClick={reset} className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] text-[#888] transition hover:bg-[#242424] hover:text-white"><RotateCcw className="h-3 w-3"/> Reset</button></div>
   <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-end"><div className="flex-1"><label className="mb-1.5 block text-[11px] font-medium text-[#aaa]">Sample basic salary (RWF)</label><input type="number" min="0" step="0.01" value={basicSalary} onChange={e=>setBasicSalary(e.target.value)} onKeyDown={e=>{if(e.key==="Enter")void preview();}} className="w-full rounded-xl border border-[#303030] bg-[#0c0c0c] px-3 py-2 text-xs text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/15" placeholder="100000"/></div><Button onClick={()=>void preview()} disabled={loading||!basicSalary} className="h-8 px-3 text-xs">{loading?"Calculating…":"Preview calculation"}</Button></div>
   <div className="mt-2 flex items-center justify-between text-[9px] text-[#666]"><span>{active.length} active rules</span><span>Calculation is executed by the Rust payroll engine.</span></div>
  </Card>
  {result&&<Card className="border-[#2b2b2b] bg-[#121212] p-4 animate-[fade-in_.2s_ease-out]">
   {result.errors.length>0&&<div className="mb-3 flex items-start gap-2 rounded-lg border border-[#5a3a1e] bg-[#21180f] px-3 py-2.5 text-[11px] text-[#e8b36b]"><AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0"/><div>{result.errors.map((e,i)=><p key={i}>{e}</p>)}</div></div>}
   <div className="mb-2 flex items-center justify-between"><h4 className="text-[10px] font-semibold uppercase tracking-[.12em] text-[#777]">Calculation breakdown</h4><span className="text-[9px] text-[#555]">{result.items.length} components</span></div>
   <div className="overflow-hidden rounded-xl border border-[#292929]"><div className="max-h-64 overflow-auto divide-y divide-[#242424]">{result.items.map(item=><div key={item.code} className={`flex items-center justify-between gap-3 px-3 py-2 transition-colors hover:bg-[#191919] ${item.code==="NET_SALARY"?"bg-[#172116]":""}`}><div className="min-w-0"><span className="font-mono text-[10px] font-semibold text-white">{item.code}</span><span className="ml-2 truncate text-[10px] text-[#777]">{item.name}</span>{item.side==="employer"&&<span className="ml-2 rounded bg-[#4a8b3f]/10 px-1.5 py-0.5 text-[8px] text-[#7fba75]">employer</span>}</div><span className={`shrink-0 text-[11px] font-semibold tabular-nums ${item.code==="NET_SALARY"?"text-[#7fba75]":"text-[#ddd]"}`}>{money(item.amount)}</span></div>)}</div></div>
   <div className="mt-3 grid grid-cols-2 gap-1.5 sm:grid-cols-4"><Summary label="Gross / base" value={money(result.gross_earnings)}/><Summary label="Tax" value={money(result.total_tax)}/><Summary label="Deductions" value={money(result.total_deductions)}/><Summary label="Employer" value={money(result.employer_contributions)}/></div>
   <div className="mt-2 flex items-center justify-between rounded-xl border border-[#4a8b3f]/40 bg-[#4a8b3f]/10 px-3 py-2.5"><span className="text-xs font-semibold text-white">Net Salary</span><span className="text-sm font-bold tabular-nums text-[#86bd7d]">{money(result.net_pay)} RWF</span></div>
  </Card>}
 </div>;
}
function Summary({label,value}:{label:string;value:string}){return <div className="rounded-lg bg-[#1a1a1a] px-2.5 py-2"><div className="text-[8px] uppercase tracking-wide text-[#666]">{label}</div><div className="mt-0.5 text-[10px] font-semibold tabular-nums text-[#ddd]">{value}</div></div>}
