import { useEffect, useState, useCallback } from "react";
import {
  BarChart3, FileText, Users, CalendarDays, Landmark, TrendingUp, Download,
  Filter, RefreshCw, CheckCircle2, AlertCircle, ChevronRight, Printer, FileSpreadsheet, FileType,
} from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { reportsService } from "@/services/reports";
import { payrollService } from "@/services/payroll";
import { staffService } from "@/services/staff";
import { PrintConfigModal } from "@/components/reports/PrintConfigModal";
import type { PayrollPeriod } from "@/types/payroll";
import type { Department } from "@/types/staff";
import type {
  ReportSummary, ReportTotals, ReportData,
  PayrollReportRow, PayrollDetailRow, StaffReportRow,
  LeaveReportRow, LeaveSummaryRow, LoanReportRow, CumulativeReportRow,
} from "@/types/reports";

type ReportType = "summary" | "payroll" | "payroll-detail" | "staff" | "leaves" | "leave-summary" | "loans" | "cumulative";

const input = "rounded-lg border border-[#303030] bg-[#101010] px-3 py-2 text-[11px] text-white outline-none transition-all duration-200 focus:border-[#4a8b3f] focus:ring-1 focus:ring-[#4a8b3f]/40";
const tableHead = "border-b border-[#303030] bg-[#111111] px-3 py-2.5 text-left text-[9px] font-semibold uppercase tracking-[.09em] text-[#777] whitespace-normal break-words leading-3";
const tableCell = "px-3 py-2.5 align-top text-[11px] leading-4 break-words whitespace-normal";
const money = (n: number) => `RWF ${new Intl.NumberFormat("en-US", { maximumFractionDigits: 2, minimumFractionDigits: 2 }).format(n || 0)}`;
const num = (n: number) => new Intl.NumberFormat("en-US").format(n || 0);

export function ReportsPage() {
  const [activeReport, setActiveReport] = useState<ReportType>("summary");
  const [summary, setSummary] = useState<ReportSummary | null>(null);
  const [periods, setPeriods] = useState<PayrollPeriod[]>([]);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [showPrintConfig, setShowPrintConfig] = useState(false);
  const [outputDir, setOutputDir] = useState("");
  const [savingDir, setSavingDir] = useState(false);

  // Filters
  const [periodId, setPeriodId] = useState<number | null>(null);
  const [departmentId, setDepartmentId] = useState<number | null>(null);
  const [statusFilter, setStatusFilter] = useState<string | null>(null);
  const [yearFilter, setYearFilter] = useState<number | null>(new Date().getFullYear());
  const [startDate, setStartDate] = useState<string | null>(null);
  const [endDate, setEndDate] = useState<string | null>(null);

  // Report data
  const [payrollData, setPayrollData] = useState<ReportData<PayrollReportRow> | null>(null);
  const [detailData, setDetailData] = useState<ReportData<PayrollDetailRow> | null>(null);
  const [staffData, setStaffData] = useState<ReportData<StaffReportRow> | null>(null);
  const [leaveData, setLeaveData] = useState<ReportData<LeaveReportRow> | null>(null);
  const [leaveSummaryData, setLeaveSummaryData] = useState<ReportData<LeaveSummaryRow> | null>(null);
  const [loanData, setLoanData] = useState<ReportData<LoanReportRow> | null>(null);
  const [cumulativeData, setCumulativeData] = useState<ReportData<CumulativeReportRow> | null>(null);

  const loadSummary = useCallback(async () => {
    try { setSummary(await reportsService.getSummary()); } catch (e) { setError(e instanceof Error ? e.message : "Unable to load summary."); }
  }, []);

  const loadPeriods = useCallback(async () => {
    try { setPeriods(await payrollService.getPeriods()); } catch { /* ignore */ }
  }, []);
  const loadDepartments = useCallback(async () => {
    try { setDepartments(await staffService.getDepartments()); } catch { /* ignore */ }
  }, []);

  useEffect(() => { void loadSummary(); void loadPeriods(); void loadDepartments(); }, [loadSummary, loadPeriods, loadDepartments]);

  const runReport = useCallback(async (report: ReportType) => {
    setLoading(true); setError(""); setMessage("");
    try {
      switch (report) {
        case "summary": await loadSummary(); break;
        case "payroll": setPayrollData(await reportsService.getPayrollReport(periodId)); break;
        case "payroll-detail": setDetailData(await reportsService.getPayrollDetail(periodId)); break;
        case "staff": setStaffData(await reportsService.getStaffReport(departmentId, statusFilter)); break;
        case "leaves": setLeaveData(await reportsService.getLeaveReport(yearFilter, statusFilter)); break;
        case "leave-summary": {
          const y = yearFilter ?? new Date().getFullYear();
          setLeaveSummaryData(await reportsService.getLeaveSummary(y));
          setLeaveData(await reportsService.getLeaveReport(y, null));
          break;
        }
        case "loans": setLoanData(await reportsService.getLoanReport(statusFilter)); break;
        case "cumulative": setCumulativeData(await reportsService.getCumulativeReport(startDate, endDate)); break;
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unable to generate report.");
    } finally { setLoading(false); }
  }, [periodId, departmentId, statusFilter, yearFilter, startDate, endDate, loadSummary]);

  useEffect(() => { if (activeReport !== "summary") void runReport(activeReport); }, [activeReport, periodId, departmentId, statusFilter, yearFilter, startDate, endDate, runReport]);

  useEffect(() => { void reportsService.getOutputDirectory().then(setOutputDir).catch(() => {}); }, []);
  const saveOutputDir = async () => { setSavingDir(true); try { const r = await reportsService.setOutputDirectory(outputDir); if (r.success && r.file_path) setOutputDir(r.file_path); } finally { setSavingDir(false); } };

  const exportableReports: ReportType[] = ["summary", "payroll", "payroll-detail", "staff", "leaves", "leave-summary", "loans", "cumulative"];
  const reportKeyMap: Record<string, string> = {
    "summary": "summary", "payroll": "payroll", "payroll-detail": "payroll-detail", "staff": "staff",
    "leaves": "leaves", "leave-summary": "leave-summary", "loans": "loans", "cumulative": "cumulative",
  };

  const handleExport = async (format: "csv" | "html" | "pdf" | "xlsx" | "print") => {
    setError(""); setMessage("");
    const reportKey = reportKeyMap[activeReport];
    if (!reportKey) { setError("This report type does not support export."); return; }
    setLoading(true);
    try {
      let res;
      switch (format) {
        case "csv": res = await reportsService.exportCsv(reportKey, periodId, yearFilter, departmentId, statusFilter, startDate, endDate); break;
        case "html": res = await reportsService.exportHtml(reportKey, periodId, yearFilter, departmentId, statusFilter, startDate, endDate); break;
        case "pdf": res = await reportsService.exportPdf(reportKey, periodId, yearFilter, departmentId, statusFilter, startDate, endDate); break;
        case "xlsx": res = await reportsService.exportXlsx(reportKey, periodId, yearFilter, departmentId, statusFilter, startDate, endDate); break;
        case "print": setShowPrintConfig(true); setLoading(false); return;
      }
      if (res && res.success) setMessage(res.message + (res.file_path ? `: ${res.file_path}` : ""));
      else if (res) setError(res.message);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Export failed.");
    } finally { setLoading(false); }
  };

  const reports: { id: ReportType; label: string; icon: typeof FileText; description: string }[] = [
    { id: "summary", label: "Summary", icon: BarChart3, description: "Key metrics across the system" },
    { id: "payroll", label: "Payroll", icon: FileText, description: "Per-period payroll totals" },
    { id: "payroll-detail", label: "Payroll Detail", icon: Users, description: "Per-employee breakdown for a period" },
    { id: "staff", label: "Staff", icon: Users, description: "Employee roster and base salaries" },
    { id: "leaves", label: "Leaves", icon: CalendarDays, description: "Leave records by year and status" },
    { id: "leave-summary", label: "Leave Summary", icon: CalendarDays, description: "Leave balances per employee" },
    { id: "loans", label: "Loans", icon: Landmark, description: "Active and paid employee loans" },
    { id: "cumulative", label: "Cumulative", icon: TrendingUp, description: "Multi-period trend analysis" },
  ];

  return (
    <div className="pb-8 animate-[fade-in_.18s_ease-out]">
      <PageHeader title="Reports" description="Generate, preview, and export payroll, staff, leave, and loan reports." />

      {(message || error) && (
        <div className={`mb-5 flex items-center gap-2 rounded-xl border px-4 py-3 text-xs ${error ? "border-[#493b2b] bg-[#19150f] text-[#e4c58d]" : "border-[#294126] bg-[#142012] text-[#a8d9a0]"}`}>
          {error ? <AlertCircle className="h-4 w-4" /> : <CheckCircle2 className="h-4 w-4" />}
          {error || message}
        </div>
      )}

      {/* Output location */}
      <Card className="mb-4 p-3.5 transition-all duration-200 hover:border-[#353535]">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-end sm:justify-between">
          <div className="min-w-0 flex-1">
            <p className="text-[10px] font-semibold uppercase tracking-wider text-[#707070]">Report output location</p>
            <p className="mt-0.5 text-[10px] text-[#606060]">Choose the folder where generated PDF, Excel, and CSV reports are saved.</p>
            <input value={outputDir} onChange={e=>setOutputDir(e.target.value)} className="mt-1 w-full rounded-lg border border-[#303030] bg-[#101010] px-3 py-2 text-xs text-white outline-none focus:border-[#4a8b3f]" placeholder="/home/user/Documents/Payroll Reports" />
          </div>
          <Button variant="secondary" onClick={() => void saveOutputDir()} disabled={savingDir || !outputDir.trim()}>
            {savingDir ? "Saving…" : "Save location"}
          </Button>
        </div>
      </Card>

      {/* Report selector grid */}
      <div className="mb-5 grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
        {reports.map((r) => {
          const Icon = r.icon;
          const active = activeReport === r.id;
          return (
            <button key={r.id} onClick={() => setActiveReport(r.id)}
              className={`group rounded-xl border p-3 text-left transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-[#4a8b3f]/60 hover:bg-[#171717] ${active ? "border-[#4a8b3f] bg-[#142012] shadow-[0_8px_24px_rgba(74,139,63,.10)]" : "border-[#292929] bg-[#151515]"}`}>
              <div className="flex items-center justify-between">
                <Icon className={`h-5 w-5 ${active ? "text-[#67b85a]" : "text-[#707070]"}`} />
                {active && <ChevronRight className="h-4 w-4 text-[#67b85a]" />}
              </div>
              <p className="mt-2.5 text-xs font-semibold text-white">{r.label}</p>
              <p className="mt-1 text-[10px] leading-4 text-[#707070]">{r.description}</p>
            </button>
          );
        })}
      </div>

      {/* Filters */}
      {(activeReport === "payroll" || activeReport === "payroll-detail" || activeReport === "staff" || activeReport === "leaves" || activeReport === "leave-summary" || activeReport === "loans" || activeReport === "cumulative") && (
        <Card className="mb-4 p-3.5 transition-all duration-200 hover:border-[#353535]">
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wider text-[#707070]">
              <Filter className="h-4 w-4" /> Filters
            </div>
            {(activeReport === "payroll" || activeReport === "payroll-detail") && (
              <label className="block text-[11px] text-[#999]">
                Period
                <select className={`mt-1 ${input}`} value={periodId ?? ""} onChange={(e) => setPeriodId(e.target.value ? Number(e.target.value) : null)}>
                  <option value="">All periods</option>
                  {periods.map((p) => <option key={p.id} value={p.id}>{p.period_name}</option>)}
                </select>
              </label>
            )}
            {activeReport === "staff" && (
              <>
                <label className="block text-[11px] text-[#999]">
                  Department
                  <select className={`mt-1 ${input}`} value={departmentId ?? ""} onChange={(e) => setDepartmentId(e.target.value ? Number(e.target.value) : null)}>
                    <option value="">All departments</option>
                    {departments.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
                  </select>
                </label>
                <label className="block text-[11px] text-[#999]">
                  Status
                  <select className={`mt-1 ${input}`} value={statusFilter ?? ""} onChange={(e) => setStatusFilter(e.target.value || null)}>
                    <option value="">All statuses</option>
                    <option value="active">Active</option>
                    <option value="terminated">Terminated</option>
                    <option value="suspended">Suspended</option>
                  </select>
                </label>
              </>
            )}
            {(activeReport === "leaves" || activeReport === "leave-summary") && (
              <label className="block text-[11px] text-[#999]">
                Year
                <input type="number" className={`mt-1 w-24 ${input}`} value={yearFilter ?? ""} onChange={(e) => setYearFilter(e.target.value ? Number(e.target.value) : null)} />
              </label>
            )}
            {activeReport === "leaves" && (
              <label className="block text-[11px] text-[#999]">
                Status
                <select className={`mt-1 ${input}`} value={statusFilter ?? ""} onChange={(e) => setStatusFilter(e.target.value || null)}>
                  <option value="">All statuses</option>
                  <option value="pending">Pending</option>
                  <option value="approved">Approved</option>
                  <option value="rejected">Rejected</option>
                </select>
              </label>
            )}
            {activeReport === "loans" && (
              <label className="block text-[11px] text-[#999]">
                Status
                <select className={`mt-1 ${input}`} value={statusFilter ?? ""} onChange={(e) => setStatusFilter(e.target.value || null)}>
                  <option value="">All statuses</option>
                  <option value="active">Active</option>
                  <option value="paid">Paid</option>
                </select>
              </label>
            )}
            {activeReport === "cumulative" && (
              <>
                <label className="block text-[11px] text-[#999]">
                  From
                  <input type="date" className={`mt-1 ${input}`} value={startDate ?? ""} onChange={(e) => setStartDate(e.target.value || null)} />
                </label>
                <label className="block text-[11px] text-[#999]">
                  To
                  <input type="date" className={`mt-1 ${input}`} value={endDate ?? ""} onChange={(e) => setEndDate(e.target.value || null)} />
                </label>
              </>
            )}
            <Button variant="secondary" onClick={() => void runReport(activeReport)} disabled={loading}>
              <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              {loading ? "Generating…" : "Generate"}
            </Button>
            {exportableReports.includes(activeReport) && (
              <>
                <Button variant="ghost" onClick={() => void handleExport("csv")} disabled={loading}>
                  <Download className="h-4 w-4" /> CSV
                </Button>
                <Button variant="ghost" onClick={() => void handleExport("xlsx")} disabled={loading}>
                  <FileSpreadsheet className="h-4 w-4" /> Excel
                </Button>
                <Button variant="ghost" onClick={() => void handleExport("pdf")} disabled={loading}>
                  <FileType className="h-4 w-4" /> PDF
                </Button>
                <Button variant="ghost" onClick={() => void handleExport("print")} disabled={loading}>
                  <Printer className="h-4 w-4" /> Print
                </Button>
              </>
            )}
          </div>
        </Card>
      )}

      {/* Report content */}
      {activeReport === "summary" && <SummaryView summary={summary} loading={loading} />}
      {activeReport === "payroll" && <PayrollView data={payrollData} loading={loading} />}
      {activeReport === "payroll-detail" && <DetailView data={detailData} loading={loading} />}
      {activeReport === "staff" && <StaffView data={staffData} loading={loading} />}
      {activeReport === "leaves" && <LeaveView data={leaveData} loading={loading} />}
      {activeReport === "leave-summary" && <LeaveSummaryView data={leaveSummaryData} loading={loading} />}
      {activeReport === "loans" && <LoanView data={loanData} loading={loading} />}
      {activeReport === "cumulative" && <CumulativeView data={cumulativeData} loading={loading} />}
      {showPrintConfig && <PrintConfigModal
        title={reports.find(r=>r.id===activeReport)?.label || "Report"}
        initialOrientation={activeReport === "payroll" || activeReport === "payroll-detail" || activeReport === "loans" ? "landscape" : "portrait"}
        onClose={()=>setShowPrintConfig(false)}
        onSavePdf={async()=>{const key=reportKeyMap[activeReport];const r=await reportsService.exportPdf(key,periodId,yearFilter,departmentId,statusFilter,startDate,endDate);if(!r.success)throw new Error(r.message);setMessage(r.message+(r.file_path?`: ${r.file_path}`:""));}}
        onPrint={async(settings)=>{const key=reportKeyMap[activeReport];const r=await reportsService.printReportConfigured(key,periodId,yearFilter,departmentId,statusFilter,startDate,endDate,settings.printer,settings.paper,settings.orientation,settings.copies);if(!r.success)throw new Error(r.message);setMessage(r.message+(r.file_path?`: ${r.file_path}`:""));}}
      />}
    </div>
  );
}

// ── Summary cards ───────────────────────────────────────────────────

function SummaryView({ summary, loading }: { summary: ReportSummary | null; loading: boolean }) {
  if (loading && !summary) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!summary) return <EmptyState icon={BarChart3} title="No summary data" description="Report summary will appear here once data is available." />;
  const cards = [
    { label: "Total Employees", value: num(summary.total_employees), sub: `${summary.active_employees} active` },
    { label: "Departments", value: num(summary.total_departments) },
    { label: "Payroll Periods", value: num(summary.total_periods), sub: `${summary.closed_periods} closed` },
    { label: "Payroll Records", value: num(summary.total_payroll_runs) },
    { label: "Total Gross (all)", value: money(summary.total_gross_all) },
    { label: "Total Net (all)", value: money(summary.total_net_all) },
    { label: "Total Tax (all)", value: money(summary.total_tax_all) },
    { label: "Active Loans", value: num(summary.total_loans_active), sub: money(summary.total_loan_outstanding) + " outstanding" },
    { label: "Leave Records", value: num(summary.total_leave_records), sub: `${summary.pending_leaves} pending` },
  ];
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {cards.map((c) => (
        <Card key={c.label} className="p-4 transition-all duration-200 hover:-translate-y-0.5 hover:border-[#353535] animate-[fade-in_.18s_ease-out]">
          <p className="text-[10px] uppercase tracking-wider text-[#707070]">{c.label}</p>
          <p className="mt-1.5 text-xl font-semibold leading-6 text-white tabular-nums">{c.value}</p>
          {c.sub && <p className="mt-1 text-[11px] text-[#9ed394]">{c.sub}</p>}
        </Card>
      ))}
    </div>
  );
}

// ── Totals bar ──────────────────────────────────────────────────────

function TotalsBar({ totals }: { totals: ReportTotals | null }) {
  if (!totals) return null;
  const items = [
    { label: "Count", value: num(totals.count) },
    { label: "Gross", value: money(totals.total_gross) },
    { label: "Deductions", value: money(totals.total_deductions) },
    { label: "Tax", value: money(totals.total_tax) },
    { label: "Net", value: money(totals.total_net) },
    { label: "Employer", value: money(totals.employer_contributions) },
  ].filter((it) => it.value !== "RWF 0.00" || it.label === "Count");
  return (
    <div className="mb-4 grid gap-2 sm:grid-cols-3 xl:grid-cols-6">
      {items.map((it) => (
        <div key={it.label} className="rounded-lg border border-[#292929] bg-[#101010] p-2.5 transition-transform duration-200 hover:-translate-y-0.5">
          <p className="text-[10px] uppercase tracking-wider text-[#707070]">{it.label}</p>
          <p className="mt-1 text-[12px] font-semibold leading-4 text-white tabular-nums">{it.value}</p>
        </div>
      ))}
    </div>
  );
}

// ── Report-specific totals ─────────────────────────────────────────
function ReportKpis({ items }: { items: { label: string; value: string }[] }) {
  return (
    <div className="mb-4 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
      {items.map((item) => (
        <div key={item.label} className="rounded-lg border border-[#292929] bg-[#101010] px-3 py-2.5 transition-all duration-200 hover:-translate-y-0.5 hover:border-[#353535]">
          <p className="text-[9px] font-semibold uppercase tracking-[.09em] text-[#707070]">{item.label}</p>
          <p className="mt-1 text-[12px] font-semibold leading-4 text-white tabular-nums">{item.value}</p>
        </div>
      ))}
    </div>
  );
}

// ── Payroll report ──────────────────────────────────────────────────

function PayrollView({ data, loading }: { data: ReportData<PayrollReportRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={FileText} title="No payroll data" description="Run payroll for a period to see it here." />;
  return (
    <div>
      <TotalsBar totals={data.totals} />
      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="min-w-full table-auto text-[11px]">
            <thead>
              <tr className="border-b border-[#292929] bg-[#101010]">
                {["Period", "Start", "End", "Employees", "Gross", "Deductions", "Tax", "Net", "Employer"].map((h) => (
                  <th key={h} className={tableHead}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-[#1e1e1e]">
              {data.rows.map((r) => (
                <tr key={r.period_id} className="transition-colors duration-150 hover:bg-[#181818]">
                  <td className={`${tableCell} font-medium text-white`}>{r.period_name}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.start_date}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.end_date}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#aaa]`}>{r.employee_count}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{money(r.total_gross)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{money(r.total_deductions)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{money(r.total_tax)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 font-semibold text-[#9ed394]`}>{money(r.total_net)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#aaa]`}>{money(r.employer_contributions)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

// ── Payroll detail ──────────────────────────────────────────────────

function DetailView({ data, loading }: { data: ReportData<PayrollDetailRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={Users} title="No detail records" description="Select a period and generate to see the Gorilla payroll register." />;
  const allowed=["BASIC","TRANSPORT","ACCOMMODATION","TAXABLE_BASE","PAYE","PENSION_EMP","PENSION_ER","PENSION_2","MATERNITY_EMP","MATERNITY_ER","CHBI","LOAN_DED","OTHER_DED"];
  const labels:Record<string,string>={BASIC:"Basic Salary",TRANSPORT:"Transport",ACCOMMODATION:"Accommodation",TAXABLE_BASE:"Taxable Base",PAYE:"PAYE",PENSION_EMP:"Employee Pension",PENSION_ER:"Employer Pension",PENSION_2:"Pension 2%",MATERNITY_EMP:"Maternity Employee",MATERNITY_ER:"Maternity Employer",CHBI:"CHBI",LOAN_DED:"Loan / Advance",OTHER_DED:"Other Deductions"};
  const maps=data.rows.map(r=>{try{const v=r as PayrollDetailRow & { calculation_snapshot?: string };const snap=v.calculation_snapshot?JSON.parse(v.calculation_snapshot):null;return new Map<string, number>((snap?.items??[]).map((i:any): [string, number] => [String(i.code??"").toUpperCase(),Number(i.amount)||0]));}catch{return new Map<string,number>();}});
  const cols=allowed.filter(c=>maps.some(m=>Math.abs(m.get(c)||0)>0)); const fmt=(n:number)=>new Intl.NumberFormat("en-RW",{maximumFractionDigits:0}).format(n||0);
  return <div><TotalsBar totals={data.totals}/><Card className="overflow-hidden"><div className="overflow-x-auto"><table className="w-full min-w-[1180px] table-fixed text-[10px]"><thead><tr className="border-b border-[#292929] bg-[#101010]"><th className="px-2 py-2 text-left text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal">No.</th><th className="px-2 py-2 text-left text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal">Employee</th><th className="px-2 py-2 text-left text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal">Department</th><th className="px-2 py-2 text-left text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal">Position</th>{cols.map(c=><th key={c} className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3"><span className="block font-semibold">{c}</span><span className="text-[8px] text-[#666]">{labels[c]}</span></th>)}<th className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3">GROSS</th><th className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3">DEDUCTIONS</th><th className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3">NET PAY</th><th className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3">EMPLOYER</th></tr></thead><tbody>{data.rows.map((r,i)=><tr key={r.record_id} className="border-b border-[#202020] transition-colors duration-150 hover:bg-[#181818]"><td className="px-2.5 py-2 align-top leading-3 break-words whitespace-normal">{i+1}</td><td className="px-2.5 py-2 align-top font-medium leading-3 text-white break-words whitespace-normal"><div>{r.employee_code}</div><div>{r.employee_name}</div></td><td className="px-2.5 py-2 align-top text-[#aaa] leading-3 break-words whitespace-normal">{r.department_name||"—"}</td><td className="px-2.5 py-2 align-top text-[#aaa] leading-3 break-words whitespace-normal">{r.position_title||"—"}</td>{cols.map(c=><td key={c} className="px-2.5 py-2 text-right tabular-nums whitespace-nowrap pr-3 align-top">{fmt(maps[i].get(c)||0)}</td>)}<td className="px-2.5 py-2 text-right font-semibold tabular-nums whitespace-nowrap pr-3 align-top">{fmt(r.gross_earnings)}</td><td className="px-2.5 py-2 text-right font-semibold tabular-nums whitespace-nowrap pr-3 align-top">{fmt(r.total_deductions)}</td><td className="px-2.5 py-2 text-right font-bold text-[#9ed394] tabular-nums whitespace-nowrap pr-3 align-top">{fmt(r.net_pay)}</td><td className="px-2 py-2 text-right text-[9px] font-semibold uppercase leading-3 text-[#777] break-words whitespace-normal pr-3">{fmt(r.employer_contributions)}</td></tr>)}</tbody></table></div></Card></div>;
}

// ── Staff report ────────────────────────────────────────────────────

function StaffView({ data, loading }: { data: ReportData<StaffReportRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={Users} title="No staff records" description="Add employees to see them in this report." />;
  return (
    <div>
      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[1180px] table-fixed text-[10px]">
            <colgroup><col className="w-[90px]"/><col className="w-[170px]"/><col className="w-[70px]"/><col className="w-[150px]"/><col className="w-[150px]"/><col className="w-[75px]"/><col className="w-[90px]"/><col className="w-[100px]"/><col className="w-[125px]"/><col className="w-[70px]"/><col className="w-[130px]"/></colgroup>
            <thead>
              <tr className="border-b border-[#292929] bg-[#101010]">
                {["Code", "Name", "Gender", "Department", "Position", "Grade", "Status", "Hire Date", "Base Salary", "Dep.", "Contract"].map((h) => (
                  <th key={h} className={tableHead}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-[#1e1e1e]">
              {data.rows.map((r) => (
                <tr key={r.id} className="transition-colors duration-150 hover:bg-[#181818]">
                  <td className={`${tableCell} font-medium text-white`}>{r.employee_code}</td>
                  <td className={`${tableCell} text-white break-words whitespace-normal`}>{r.full_name}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.gender ?? "—"}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.department_name ?? "—"}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.position_title ?? "—"}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.grade ?? "—"}</td>
                  <td className={`${tableCell} break-words whitespace-normal`}><StatusBadge status={r.employment_status} /></td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.hire_date ?? "—"}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{money(r.base_salary)}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.dependants}</td>
                  <td className={`${tableCell} text-[#aaa] break-words whitespace-normal`}>{r.contract_type_name ?? "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
      <div className="mt-4">
        <ReportKpis items={[
          { label: "Total Staff", value: num(data.rows.length) },
          { label: "Total Basic Salaries", value: money(data.rows.reduce((sum, r) => sum + (r.base_salary || 0), 0)) },
          { label: "Total Males", value: num(data.rows.filter(r => (r.gender ?? "").toLowerCase() === "male").length) },
          { label: "Total Females", value: num(data.rows.filter(r => (r.gender ?? "").toLowerCase() === "female").length) },
        ]} />
      </div>
    </div>
  );
}

// ── Leave report ────────────────────────────────────────────────────

function LeaveView({ data, loading }: { data: ReportData<LeaveReportRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={CalendarDays} title="No leave records" description="Leave requests will appear here once recorded." />;
  return (
    <div>
      <Card className="overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[980px] table-fixed text-[10px]">
          <colgroup><col className="w-[85px]"/><col className="w-[180px]"/><col className="w-[145px]"/><col className="w-[105px]"/><col className="w-[105px]"/><col className="w-[70px]"/><col className="w-[95px]"/><col className="w-[190px]"/></colgroup>
          <thead>
            <tr className="border-b border-[#292929] bg-[#101010]">
              {["Code", "Name", "Leave Type", "Start", "End", "Days", "Status", "Reason"].map((h) => (
                <th key={h} className={tableHead}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-[#1e1e1e]">
            {data.rows.map((r) => (
              <tr key={r.id} className="transition-colors duration-150 hover:bg-[#181818]">
                <td className={`${tableCell} font-medium text-white`}>{r.employee_code}</td>
                <td className={`${tableCell} text-white`}>{r.employee_name}</td>
                <td className={`${tableCell} text-[#aaa]`}>{r.leave_type_name}</td>
                <td className={`${tableCell} text-[#aaa]`}>{r.start_date}</td>
                <td className={`${tableCell} text-[#aaa]`}>{r.end_date}</td>
                <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{r.days.toFixed(1)}</td>
                <td className={tableCell}><StatusBadge status={r.status} /></td>
                <td className={`${tableCell} text-[#777]`}>{r.reason ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      </Card>
      <div className="mt-4">
        <ReportKpis items={[
          { label: "Total Employee", value: num(new Set(data.rows.map(r => r.employee_code)).size) },
          { label: "Total Approved Leaves", value: num(data.rows.filter(r => r.status.toLowerCase() === "approved").length) },
          { label: "Total Rejected", value: num(data.rows.filter(r => r.status.toLowerCase() === "rejected").length) },
        ]} />
      </div>
    </div>
  );
}

// ── Leave summary ───────────────────────────────────────────────────

function LeaveSummaryView({ data, loading }: { data: ReportData<LeaveSummaryRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={CalendarDays} title="No leave balances" description="Leave balances will appear here once employees and leave types exist." />;
  return (
    <div>
      <Card className="overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[780px] table-fixed text-[10px]">
          <colgroup><col className="w-[90px]"/><col className="w-[190px]"/><col className="w-[170px]"/><col className="w-[105px]"/><col className="w-[105px]"/><col className="w-[110px]"/></colgroup>
          <thead>
            <tr className="border-b border-[#292929] bg-[#101010]">
              {["Code", "Name", "Leave Type", "Entitled", "Used", "Remaining"].map((h) => (
                <th key={h} className={tableHead}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-[#1e1e1e]">
            {data.rows.map((r, i) => (
              <tr key={`${r.employee_id}-${r.leave_type_name}-${i}`} className="transition-colors duration-150 hover:bg-[#181818]">
                <td className={`${tableCell} font-medium text-white`}>{r.employee_code}</td>
                <td className={`${tableCell} text-white`}>{r.employee_name}</td>
                <td className={`${tableCell} text-[#aaa]`}>{r.leave_type_name}</td>
                <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{r.entitled.toFixed(1)}</td>
                <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{r.used.toFixed(1)}</td>
                <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 font-semibold text-[#9ed394]`}>{r.remaining.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      </Card>
      <div className="mt-4">
        <ReportKpis items={[
          { label: "Total Employees", value: num(new Set(data.rows.map(r => r.employee_id)).size) },
          { label: "Total Leave Type", value: num(new Set(data.rows.map(r => r.leave_type_name)).size) },
        ]} />
      </div>
    </div>
  );
}

// ── Loan report ─────────────────────────────────────────────────────

function LoanView({ data, loading }: { data: ReportData<LoanReportRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={Landmark} title="No loan records" description="Employee loans will appear here once recorded." />;
  return (
    <div>
      <TotalsBar totals={data.totals} />
      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="min-w-full table-auto text-[11px]">
            <thead>
              <tr className="border-b border-[#292929] bg-[#101010]">
                {["Code", "Name", "Principal", "Rate", "Total", "Installment", "Total Inst.", "Paid", "Remaining", "Start", "Status"].map((h) => (
                  <th key={h} className={tableHead}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-[#1e1e1e]">
              {data.rows.map((r) => (
                <tr key={r.id} className="transition-colors duration-150 hover:bg-[#181818]">
                  <td className={`${tableCell} font-medium text-white`}>{r.employee_code}</td>
                  <td className={`${tableCell} text-white`}>{r.employee_name}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{money(r.principal)}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.interest_rate.toFixed(2)}%</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{money(r.total_amount)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{money(r.installment_amount)}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.total_installments}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.paid_installments}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 font-semibold text-[#9ed394]`}>{money(r.remaining_amount)}</td>
                  <td className={`${tableCell} text-[#aaa]`}>{r.start_date}</td>
                  <td className={tableCell}><StatusBadge status={r.status} /></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

// ── Cumulative report ──────────────────────────────────────────────

function CumulativeView({ data, loading }: { data: ReportData<CumulativeReportRow> | null; loading: boolean }) {
  if (loading && !data) return <Card className="p-12 text-center"><RefreshCw className="mx-auto h-7 w-7 animate-spin text-[#4a8b3f]" /></Card>;
  if (!data || data.rows.length === 0) return <EmptyState icon={TrendingUp} title="No cumulative data" description="Run payroll across multiple periods to see trends." />;
  return (
    <div>
      <TotalsBar totals={data.totals} />
      <Card className="overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full min-w-[900px] table-fixed text-[10px]">
            <colgroup><col className="w-[180px]"/><col className="w-[100px]"/><col className="w-[135px]"/><col className="w-[135px]"/><col className="w-[125px]"/><col className="w-[135px]"/><col className="w-[135px]"/><col className="w-[135px]"/></colgroup>
            <thead>
              <tr className="border-b border-[#292929] bg-[#101010]">
                {["Period", "Employees", "Gross", "Deductions", "Tax", "Net", "Employer", "Loan Ded."].map((h) => (
                  <th key={h} className={tableHead}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-[#1e1e1e]">
              {data.rows.map((r, i) => (
                <tr key={i} className="transition-colors duration-150 hover:bg-[#181818]">
                  <td className={`${tableCell} font-medium text-white`}>{r.period_name}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#aaa]`}>{r.employee_count}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-white`}>{money(r.total_gross)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{money(r.total_deductions)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#e4c58d]`}>{money(r.total_tax)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 font-semibold text-[#9ed394]`}>{money(r.total_net)}</td>
                  <td className={`${tableCell} text-right tabular-nums whitespace-nowrap pr-4 text-[#aaa]`}>{money(r.employer_contributions)}</td>
                  <td className={`${tableCell} text-[#e4c58d]`}>{money(r.loan_deductions)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
    </div>
  );
}

// ── Status badge ────────────────────────────────────────────────────

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    active: "border-[#294126] bg-[#142012] text-[#9ed394]",
    approved: "border-[#294126] bg-[#142012] text-[#9ed394]",
    calculated: "border-[#294126] bg-[#142012] text-[#9ed394]",
    finalized: "border-[#294126] bg-[#142012] text-[#9ed394]",
    closed: "border-[#294126] bg-[#142012] text-[#9ed394]",
    paid: "border-[#294126] bg-[#142012] text-[#9ed394]",
    pending: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
    processing: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
    rejected: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
    terminated: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
    suspended: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
    cancelled: "border-[#493b2b] bg-[#19150f] text-[#e4c58d]",
  };
  const cls = styles[status] ?? "border-[#303030] bg-[#1a1a1a] text-[#aaa]";
  return <span className={`inline-flex rounded-full border px-2.5 py-1 text-[9px] font-semibold ${cls}`}>{status}</span>;
}
