import { useEffect, useMemo, useState } from "react";
import { Download, Printer } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { staffService } from "@/services/staff";
import { leaveService } from "@/types/leaves";
import { reportsService } from "@/services/reports";
import { PrintConfigModal } from "@/components/reports/PrintConfigModal";
import type { Employee, Company } from "@/types/staff";
import type { LeaveBalance } from "@/types/leaves";
import type { Payslip, PayrollPeriod } from "@/types/payroll";

const money = (value: number) => new Intl.NumberFormat("en-RW", {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
}).format(Number.isFinite(value) ? value : 0);

const text = (value: unknown, fallback = "—") => {
  const s = String(value ?? "").trim();
  return s || fallback;
};

function snapshotOf(payslip: Payslip): any {
  try { return payslip.calculation_snapshot ? JSON.parse(payslip.calculation_snapshot) : null; }
  catch { return null; }
}

function itemMap(snapshot: any): Map<string, any> {
  return new Map((snapshot?.items ?? []).map((item: any) => [String(item.code ?? "").toUpperCase(), item]));
}

function amount(items: Map<string, any>, code: string): number {
  const value = Number(items.get(code)?.amount ?? 0);
  return Number.isFinite(value) ? value : 0;
}

function sumEmployeeEarnings(items: any[]): number {
  return items
    .filter((item) => item.side === "employee" && item.component_type === "earning" && !["TAXABLE_BASE", "NET_SALARY"].includes(String(item.code).toUpperCase()))
    .reduce((sum, item) => sum + (Number(item.amount) || 0), 0);
}

function sumEmployeeDeductions(items: any[]): number {
  return items
    .filter((item) => item.side === "employee" && ["tax", "deduction", "contribution"].includes(item.component_type))
    .filter((item) => !["TOTAL_DED"].includes(String(item.code).toUpperCase()))
    .reduce((sum, item) => sum + (Number(item.amount) || 0), 0);
}

function Row({ label, value, strong = false }: { label: string; value: string; strong?: boolean }) {
  return (
    <div className={`flex items-baseline justify-between gap-4 py-1.5 ${strong ? "font-semibold" : ""}`}>
      <span>{label}</span>
      <span className="tabular-nums text-right">{value}</span>
    </div>
  );
}

export function PayslipDocument({ payslip, period }: { payslip: Payslip; period: PayrollPeriod | null }) {
  const [employee, setEmployee] = useState<Employee | null>(null);
  const [company, setCompany] = useState<Company | null>(null);
  const [balance, setBalance] = useState<LeaveBalance | null>(null);
  const [loading, setLoading] = useState(true);
  const [showPrintConfig, setShowPrintConfig] = useState(false);
  const snapshot = useMemo(() => snapshotOf(payslip), [payslip]);
  const items = useMemo(() => itemMap(snapshot), [snapshot]);
  const rawItems = snapshot?.items ?? [];

  useEffect(() => {
    let active = true;
    setLoading(true);
    void Promise.all([
      staffService.getEmployee(payslip.employee_id),
      staffService.getCompany(),
      period ? leaveService.getBalances(Number(period.start_date?.slice(0, 4) || new Date().getFullYear())) : Promise.resolve([] as LeaveBalance[]),
    ]).then(([emp, companyData, balances]) => {
      if (!active) return;
      setEmployee(emp);
      setCompany(companyData);
      const annual = (balances as LeaveBalance[]).find((b) =>
        Number(b.employee_id) === Number(payslip.employee_id) && /annual|vacation/i.test(b.leave_type_name)
      ) ?? (balances as LeaveBalance[]).find((b) => Number(b.employee_id) === Number(payslip.employee_id));
      setBalance(annual ?? null);
    }).finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [payslip.employee_id, period?.id]);

  const basic = amount(items, "BASIC");
  const transport = amount(items, "TRANSPORT");
  const accommodation = amount(items, "ACCOMMODATION");
  const otherPayment = Math.max(0, sumEmployeeEarnings(rawItems) - basic - transport - accommodation);
  const gross = basic + transport + accommodation + otherPayment;
  const paye = amount(items, "PAYE");
  const pension = amount(items, "PENSION_EMP");
  const maternity = amount(items, "MATERNITY_EMP");
  const loan = amount(items, "LOAN_DED");
  const otherDeduction = Math.max(0, sumEmployeeDeductions(rawItems) - paye - pension - maternity - loan);
  const deductions = paye + pension + maternity + loan + otherDeduction;
  const net = gross - deductions;
  const year = Number(period?.start_date?.slice(0, 4) || new Date().getFullYear());
  const attendance = snapshot?.attendance ?? null;
  const paidDays = Number(attendance?.paid_days ?? 0);
  const scheduledDays = Number(attendance?.scheduled_days ?? 0);
  const titleMonth = period?.pay_date
    ? new Date(`${period.pay_date}T00:00:00`).toLocaleDateString("en-US", { month: "long", year: "numeric" }).toUpperCase()
    : text(payslip.period_name, "PAYROLL").toUpperCase();

  return (
    <Card className="print-target overflow-hidden border-[#2b2b2b] bg-white p-0 text-black shadow-2xl">
      <div className="p-7 sm:p-9">
        <div className="mb-2 flex items-start justify-between gap-4 print-hide">
          <div>
            <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-black/45">Payslip document</p>
            <p className="mt-1 text-xs text-black/55">Print or use your system print dialog to choose “Save as PDF”.</p>
          </div>
          <div className="flex gap-2">
            <Button type="button" onClick={() => setShowPrintConfig(true)}><Printer className="h-4 w-4" /> Print</Button>
            <Button type="button" variant="secondary" onClick={() => setShowPrintConfig(true)}><Download className="h-4 w-4" /> Save PDF</Button>
          </div>
        </div>

        <div className="border border-black px-5 py-4 font-serif text-[12px] leading-[1.45]">
          <div className="flex items-start gap-4">
            <img src="/gorilla-doctors.jpeg" alt="Gorilla Doctors" className="h-12 w-12 object-contain" />
            <div className="flex-1 text-center">
              <div className="text-[16px] font-bold tracking-wide">PAYSLIP - {titleMonth}</div>
              {company?.name && <div className="mt-0.5 text-[10px] uppercase tracking-wide text-black/60">{company.name}</div>}
            </div>
            <div className="w-12" />
          </div>

          <div className="mt-3 grid grid-cols-2 gap-x-8 gap-y-1 border-b border-black pb-3">
            <InfoLine label="Employee Code" value={payslip.employee_code} />
            <InfoLine label="Employee Names" value={text(employee ? `${employee.first_name} ${employee.last_name}` : payslip.employee_name)} />
            <InfoLine label="Position" value={text(employee?.position_title)} />
            <InfoLine label="RSSB Number" value={text(employee?.rssb_number)} />
            <InfoLine label="Date of start" value={text(employee?.hire_date)} />
            <InfoLine label="Department" value={text(employee?.department_name)} />
            <InfoLine label="Grade" value={text(employee?.grade)} />
            <InfoLine label="# of Dependant" value={String(employee?.dependants ?? 0)} />
          </div>

          <div className="border-b border-black py-2 font-medium">
            Number of days you are paid: <span className="font-semibold">{paidDays || "—"} out of {scheduledDays || "—"}</span>
          </div>

          <div className="mt-3 border-b border-black pb-2">
            <Row label="Basic Salary:" value={money(basic)} />
            <Row label="Transport allowance:" value={money(transport)} />
            <Row label="Housing Allowance:" value={money(accommodation)} />
            <Row label="Other payment:" value={otherPayment ? money(otherPayment) : "-"} />
            <Row label="Gross Salary:" value={money(gross)} strong />
          </div>

          <div className="mt-3 border-b border-black pb-2">
            <Row label="Professional Tax:" value={paye ? money(paye) : "-"} />
            <Row label="Caisse Sociale(6%):" value={pension ? money(pension) : "-"} />
            <Row label="Medical insurance agency:" value={maternity ? money(maternity) : "-"} />
            <Row label="Other deductions:" value={otherDeduction ? money(otherDeduction) : "-"} />
            <Row label="Total deductions" value={money(deductions)} strong />
            <Row label="Net remuneration:" value={money(net)} strong />
            <div className="mt-1 text-[11px] italic">(A net of {money(net)} FRW)</div>
          </div>

          <div className="mt-3 grid grid-cols-2 gap-x-8 gap-y-1 border-b border-black pb-3">
            <InfoLine label="Paid at" value={text(employee?.bank_name)} />
            <InfoLine label="Account No" value={text(employee?.bank_account)} />
            <InfoLine label="Entitlement to annual leave" value={balance ? String(balance.entitled) : "—"} />
            <InfoLine label={`Leaves consumed from the beginning of the year ${year}`} value={balance ? String(balance.used) : "—"} />
          </div>

          <div className="mt-4 min-h-[70px] border-b border-black pb-3">
            <span className="underline">Other deductions:</span>
            <span className="ml-24">{loan ? `Loan / advance: ${money(loan)}` : "None"}</span>
          </div>

          <div className="grid grid-cols-2 gap-10 pt-6 text-[12px]">
            <div className="min-h-[70px]">
              <div>For MGVP</div>
              <div className="mt-8 border-t border-black/40 pt-1 text-[10px] text-black/55">Authorized signature</div>
            </div>
            <div className="min-h-[70px] text-right">
              <div>For acceptance &amp; reception</div>
              <div className="mt-8 border-t border-black/40 pt-1 text-[10px] text-black/55">{text(employee ? `${employee.first_name} ${employee.last_name}` : payslip.employee_name)}</div>
            </div>
          </div>
        </div>

        {loading && <p className="mt-2 text-center text-[10px] text-black/40 print-hide">Loading employee and leave details…</p>}
      </div>
      {showPrintConfig && <PrintConfigModal
        title={`PAYSLIP - ${titleMonth}`} initialOrientation="portrait" onClose={() => setShowPrintConfig(false)}
        onSavePdf={async () => { const r = await reportsService.exportPayslipPdf(payslip.payroll_record_id); if (!r.success) throw new Error(r.message); }}
        onPrint={async (settings) => { const r = await reportsService.printPayslipConfigured(payslip.payroll_record_id, settings.printer, settings.paper, settings.orientation, settings.copies); if (!r.success) throw new Error(r.message); }}
      />}
    </Card>
  );
}

function InfoLine({ label, value }: { label: string; value: string }) {
  return <div><span className="italic">{label}:</span> <span>{value}</span></div>;
}
