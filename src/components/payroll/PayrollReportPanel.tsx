import { useEffect, useMemo, useState } from "react";
import { FolderOpen, Printer, FileDown } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { PrintConfigModal } from "@/components/reports/PrintConfigModal";
import type { PayrollPeriod, PayrollRecord } from "@/types/payroll";
import { reportsService } from "@/services/reports";

const COLUMNS = [
  ["BASIC","BASIC SALARY"],["TRANSPORT","TRANSPORT"],["ACCOMMODATION","ACCOMMODATION"],
  ["TAXABLE_BASE","BASE IMPOSABLE"],["PAYE","PAYE (TPR)"],["BASE_RSSB","BASE RSSB"],
  ["PENSION_EMP","PENSION 6% EMP"],["PENSION_ER","PENSION 6% ER"],["PENSION_2","PENSION 2%"],
  ["PENSION_TOTAL","PENSION TOTAL"],["MATERNITY_EMP","MATERNITY 0.3% EMP"],["MATERNITY_ER","MATERNITY 0.3% ER"],
  ["MATERNITY_TOTAL","MATERNITY TOTAL"],["NET_SALARY","NET SALARY"],["CHBI","CHBI 0.5%"],
] as const;

const fmt=(v:number)=>new Intl.NumberFormat("en-RW",{minimumFractionDigits:0,maximumFractionDigits:0}).format(Number.isFinite(v)?v:0);
function mapOf(r:PayrollRecord): Map<string,number>{
  try{
    const s=r.calculation_snapshot?JSON.parse(r.calculation_snapshot):null;
    const m=new Map<string,number>((s?.items??[]).map((i:any):[string,number]=>[String(i.code??"").toUpperCase(),Number(i.amount)||0]));
    if(!m.has("BASE_RSSB")) m.set("BASE_RSSB",m.get("TAXABLE_BASE")||0);
    if(!m.has("PENSION_TOTAL")) m.set("PENSION_TOTAL",(m.get("PENSION_EMP")||0)+(m.get("PENSION_ER")||0)+(m.get("PENSION_2")||0));
    if(!m.has("MATERNITY_TOTAL")) m.set("MATERNITY_TOTAL",(m.get("MATERNITY_EMP")||0)+(m.get("MATERNITY_ER")||0));
    return m;
  }catch{return new Map<string,number>();}
}

export function PayrollReportPanel({period,records}:{period:PayrollPeriod|null;records:PayrollRecord[]}){
 const maps=useMemo(()=>records.map(mapOf),[records]);
 const [showPrint,setShowPrint]=useState(false);
 const [output,setOutput]=useState("");
 useEffect(()=>{void reportsService.getOutputDirectory().then(setOutput).catch(()=>{});},[records.length]);
 const totals=useMemo(()=>{const m=new Map<string,number>();for(const map of maps)for(const [code] of COLUMNS)m.set(code,(m.get(code)||0)+(map.get(code)||0));return m;},[maps]);
 const chooseOutput=async()=>{const selected=await open({directory:true,multiple:false,defaultPath:output||undefined,title:"Choose Payroll Report Output Folder"});if(!selected||Array.isArray(selected))return;const r=await reportsService.setOutputDirectory(selected);if(r.success)setOutput(selected);};
 const savePdf=async()=>{const r=await reportsService.exportPdf("payroll-detail",period?.id??null,period?Number(period.start_date.slice(0,4)):null,null,null,null,null);if(!r.success)throw new Error(r.message);};
 const print=async(settings:{printer:string|null;paper:string;orientation:string;copies:number})=>{const r=await reportsService.printReportConfigured("payroll-detail",period?.id??null,period?Number(period.start_date.slice(0,4)):null,null,null,null,null,settings.printer,settings.paper,settings.orientation,settings.copies);if(!r.success)throw new Error(r.message);};
 return <div className="space-y-3 animate-[fade-in_.25s_ease-out]">
  <div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between print-hide">
   <div className="min-w-0"><div className="flex items-center gap-2"><div className="rounded-lg bg-[#4a8b3f]/12 p-1.5 text-[#75ad6b]"><FileDown className="h-4 w-4"/></div><div><h3 className="text-base font-semibold text-white">Payroll Report</h3><p className="text-[11px] text-[#777]">Monthly payroll register · frozen calculation snapshot</p></div></div>
    <div className="mt-1 flex items-center gap-2 text-[10px] text-[#777]"><span className="truncate">Output: {output||"Default report folder"}</span><Button variant="ghost" className="h-7 px-2 text-[10px]" onClick={()=>void chooseOutput()}><FolderOpen className="h-3 w-3"/> Change</Button></div>
   </div>
   <div className="flex gap-2"><Button variant="secondary" className="h-8 px-3 text-xs" onClick={()=>void savePdf()} disabled={!records.length}><FileDown className="h-3.5 w-3.5"/> Save PDF</Button><Button className="h-8 px-3 text-xs" onClick={()=>setShowPrint(true)} disabled={!records.length}><Printer className="h-3.5 w-3.5"/> Print</Button></div>
  </div>
  <Card className="overflow-hidden border-[#2b2b2b] bg-[#121212] p-0 shadow-[0_14px_38px_rgba(0,0,0,.22)]">
   <div className="flex items-center justify-between border-b border-[#2b2b2b] px-4 py-3">
    <div className="flex items-center gap-3"><img src="/gorilla-doctors.jpeg" alt="Gorilla Doctors" className="h-9 w-9 object-contain"/><div><div className="text-xs font-bold tracking-wide text-white">GORILLA DOCTORS</div><div className="text-[9px] uppercase tracking-[.14em] text-[#777]">Payroll Register / Monthly Payroll Report</div></div></div>
    <div className="text-right text-[9px] text-[#888]"><div className="font-semibold text-white">{period?.period_name||"No period selected"}</div>{period&&<div>{period.start_date} → {period.end_date} · {records.length} employees</div>}</div>
   </div>
   {records.length===0?<div className="px-5 py-12 text-center text-xs text-[#777]">Calculate a payroll period to generate the register.</div>:
   <div className="max-h-[62vh] overflow-auto"><table className="w-full min-w-[1180px] table-fixed border-separate border-spacing-0 text-[9px] leading-snug"><colgroup><col style={{width:"34px"}}/><col style={{width:"82px"}}/><col style={{width:"150px"}}/>{COLUMNS.map(([code])=><col key={code} style={{width: code==="ACCOMMODATION"?"92px":code==="TAXABLE_BASE"?"94px":code.includes("MATERNITY")?"98px":code.includes("PENSION")?"88px":"82px"}}/>)}</colgroup><thead className="sticky top-0 z-10 bg-[#181818] text-[#b8b8b8]"><tr><th className="sticky left-0 z-20 border-r border-[#333] bg-[#181818] px-1.5 py-2.5 text-left align-top whitespace-normal break-words">No.</th><th className="sticky left-[30px] z-20 min-w-[85px] border-r border-[#333] bg-[#181818] px-1.5 py-2.5 text-left align-top whitespace-normal break-words">Code</th><th className="sticky left-[115px] z-20 min-w-[150px] border-r border-[#333] bg-[#181818] px-1.5 py-2.5 text-left align-top whitespace-normal break-words">Employee</th>{COLUMNS.map(([code,label])=><th key={code} className={`border-r border-[#333] px-2 py-2.5 text-right align-top whitespace-normal break-words ${code==="NET_SALARY"?"text-[#8bc180]":""}`}><span className="block font-bold">{code}</span><span className="mt-0.5 block whitespace-normal break-words text-[7px] font-normal leading-[1.15] opacity-70">{label}</span></th>)}</tr></thead>
   <tbody>{records.map((r,i)=><tr key={r.id} className="border-b border-[#242424] align-top transition-colors duration-150 hover:bg-[#191919]"><td className="sticky left-0 z-[1] border-r border-[#333] bg-[#121212] px-2 py-2.5 align-top tabular-nums">{i+1}</td><td className="sticky left-[30px] z-[1] border-r border-[#333] bg-[#121212] px-2 py-2.5 align-top font-semibold text-white break-words">{r.employee_code}</td><td className="sticky left-[115px] z-[1] border-r border-[#333] bg-[#121212] px-2 py-2"><div className="whitespace-normal break-words text-[#aaa] leading-snug">{r.employee_name}</div></td>{COLUMNS.map(([code])=><td key={code} className={`border-r border-[#252525] px-2 py-2.5 text-right align-top tabular-nums whitespace-normal break-words ${code==="NET_SALARY"?"font-bold text-[#7fba75]":"text-[#ddd]"}`}>{fmt(maps[i].get(code)||0)}</td>)}</tr>)}<tr className="border-t-2 border-[#4a8b3f] bg-[#1a1a1a] font-bold"><td className="sticky left-0 z-[1] border-r border-[#333] bg-[#1a1a1a] px-2 py-2"></td><td className="sticky left-[30px] z-[1] border-r border-[#333] bg-[#1a1a1a] px-2 py-2"></td><td className="sticky left-[115px] z-[1] border-r border-[#333] bg-[#1a1a1a] px-1.5 py-2 text-white">TOTAL</td>{COLUMNS.map(([code])=><td key={code} className={`border-r border-[#333] px-2 py-2.5 text-right align-top tabular-nums whitespace-normal break-words ${code==="NET_SALARY"?"text-[#8bc180]":"text-white"}`}>{fmt(totals.get(code)||0)}</td>)}</tr></tbody></table></div>}
  </Card>
  {showPrint&&<PrintConfigModal title="GORILLA DOCTORS Payroll Register / Monthly Payroll Report" initialOrientation="landscape" onClose={()=>setShowPrint(false)} onSavePdf={async()=>{await savePdf();setShowPrint(false);}} onPrint={async(settings)=>{await print(settings);setShowPrint(false);}}/>}
 </div>;
}
