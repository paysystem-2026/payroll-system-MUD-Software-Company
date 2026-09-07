import { useMemo, useState } from "react";
import type { PayrollRule, FormulaTestResult, TestInput } from "@/types/payroll";
import { payrollService } from "@/services/payroll";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { FlaskConical, Plus, X, CheckCircle, XCircle, RotateCcw, Play } from "lucide-react";

interface FormulaTesterProps { rules: PayrollRule[]; }

export function FormulaTester({ rules }: FormulaTesterProps) {
  const activeRules = useMemo(() => rules.filter(r => r.is_active), [rules]);
  const [expression, setExpression] = useState("");
  const [inputs, setInputs] = useState<TestInput[]>([{ code: "BASIC", value: 100000 }]);
  const [result, setResult] = useState<FormulaTestResult | null>(null);
  const [testing, setTesting] = useState(false);

  const addInput = () => setInputs(v => [...v, { code: "", value: 0 }]);
  const removeInput = (idx:number) => setInputs(v => v.length > 1 ? v.filter((_,i)=>i!==idx) : v);
  const updateInput = (idx:number, field:"code"|"value", val:string) => setInputs(v => v.map((item,i) =>
    i===idx ? {...item, [field]: field==="code" ? val.toUpperCase().replace(/[^A-Z0-9_]/g,"") : (Number(val)||0)} : item
  ));
  const insertCode = (code:string) => setExpression(v => v ? `${v.trim()} ${code}` : code);
  const reset = () => { setExpression(""); setInputs([{code:"BASIC",value:100000}]); setResult(null); };

  const handleTest = async () => {
    const expr = expression.trim();
    if (!expr) return;
    setTesting(true);
    try {
      const res = await payrollService.testFormula({expression:expr,inputs:inputs.filter(i=>i.code.trim())});
      setResult(res);
    } catch(e) {
      setResult({success:false,result:null,error:e instanceof Error?e.message:String(e),breakdown:[]});
    } finally { setTesting(false); }
  };

  return <div className="space-y-3">
    <Card className="border-[#2b2b2b] bg-[#121212] p-4 transition-all duration-200 hover:border-[#353535]">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-2"><div className="rounded-lg bg-[#4a8b3f]/12 p-2 text-[#75ad6b]"><FlaskConical className="h-4 w-4"/></div><div><h3 className="text-sm font-semibold text-white">Formula Tester</h3><p className="text-[10px] text-[#777]">Test a formula safely before saving a payroll rule.</p></div></div>
        <button type="button" onClick={reset} className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[10px] text-[#888] transition hover:bg-[#242424] hover:text-white"><RotateCcw className="h-3 w-3"/> Reset</button>
      </div>

      <div className="mt-3">
        <label className="mb-1.5 block text-[11px] font-medium text-[#aaa]">Formula expression</label>
        <textarea value={expression} onChange={e=>setExpression(e.target.value)} onKeyDown={e=>{if((e.ctrlKey||e.metaKey)&&e.key==="Enter"){e.preventDefault();void handleTest();}}}
          className="min-h-[68px] w-full resize-y rounded-xl border border-[#303030] bg-[#0c0c0c] px-3 py-2.5 font-mono text-xs text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/15"
          placeholder="Example: BASIC + TRANSPORT + ACCOMMODATION" />
        <p className="mt-1 text-[9px] text-[#666]">Tip: press Ctrl + Enter to test.</p>
      </div>

      <div className="mt-3 rounded-xl border border-[#292929] bg-[#0f0f0f] p-3">
        <div className="mb-2 flex items-center justify-between"><div><p className="text-[11px] font-semibold text-white">Input values</p><p className="text-[9px] text-[#666]">Only variables used by your formula are needed.</p></div><button type="button" onClick={addInput} className="inline-flex items-center gap-1 rounded-lg border border-[#2d2d2d] px-2 py-1 text-[10px] text-[#7fba75] transition hover:border-[#4a8b3f] hover:bg-[#4a8b3f]/10"><Plus className="h-3 w-3"/> Add</button></div>
        <div className="space-y-1.5">{inputs.map((inp,idx)=><div key={idx} className="flex gap-1.5">
          <input value={inp.code} onChange={e=>updateInput(idx,"code",e.target.value)} placeholder="CODE" className="w-28 rounded-lg border border-[#2c2c2c] bg-[#0b0b0b] px-2.5 py-1.5 font-mono text-[11px] text-white outline-none focus:border-[#4a8b3f]"/>
          <input type="number" value={inp.value} onChange={e=>updateInput(idx,"value",e.target.value)} className="min-w-0 flex-1 rounded-lg border border-[#2c2c2c] bg-[#0b0b0b] px-2.5 py-1.5 text-[11px] text-white outline-none focus:border-[#4a8b3f]"/>
          <button type="button" onClick={()=>removeInput(idx)} aria-label="Remove input" className="rounded-lg px-2 text-[#666] transition hover:bg-[#242424] hover:text-white"><X className="h-3.5 w-3.5"/></button>
        </div>)}</div>
      </div>

      <div className="mt-3">
        <div className="mb-1.5 flex items-center justify-between"><label className="text-[10px] font-medium uppercase tracking-[.12em] text-[#666]">Available component codes</label><span className="text-[9px] text-[#555]">{activeRules.length} active</span></div>
        <div className="flex max-h-20 flex-wrap gap-1.5 overflow-auto">{activeRules.map(r=><button type="button" key={r.id} onClick={()=>insertCode(r.code)} className="rounded-md border border-[#292929] bg-[#1a1a1a] px-2 py-1 font-mono text-[10px] text-[#999] transition hover:-translate-y-px hover:border-[#4a8b3f]/60 hover:text-white">{r.code}</button>)}</div>
      </div>

      <div className="mt-3 flex justify-end"><Button onClick={()=>void handleTest()} disabled={testing||!expression.trim()} className="h-8 px-3 text-xs"><Play className="h-3.5 w-3.5"/>{testing?"Testing…":"Test formula"}</Button></div>
    </Card>

    {result && <Card className={`border-[#2b2b2b] bg-[#121212] p-4 animate-[fade-in_.2s_ease-out] ${result.success?"":"border-[#5a3a1e]"}`}>
      <div className="flex items-center justify-between gap-3"><div className="flex items-center gap-2">{result.success?<CheckCircle className="h-4 w-4 text-[#6fa765]"/>:<XCircle className="h-4 w-4 text-[#d0d0d0]"/>}<div><p className="text-[10px] uppercase tracking-[.12em] text-[#666]">Test result</p><p className={`text-sm font-semibold ${result.success?"text-[#7fba75]":"text-white"}`}>{result.success?result.result:"Formula could not be evaluated"}</p></div></div></div>
      {result.error&&<div className="mt-3 rounded-lg border border-[#5a3a1e] bg-[#21180f] px-3 py-2 text-[11px] text-[#e8b36b]">{result.error}</div>}
      {result.breakdown.length>0&&<div className="mt-3 grid gap-1.5 sm:grid-cols-2">{result.breakdown.map((item,i)=><div key={i} className="flex items-center justify-between rounded-lg bg-[#1b1b1b] px-3 py-2"><span className="font-mono text-[10px] text-[#888]">{item.code}</span><span className="text-[11px] font-semibold tabular-nums text-white">{item.amount}</span></div>)}</div>}
    </Card>}
  </div>;
}
