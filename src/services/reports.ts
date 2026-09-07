import { invokeCommand } from "@/services/tauri";
import type {
  ReportSummary, ReportData, PayrollReportRow, PayrollDetailRow,
  StaffReportRow, LeaveReportRow, LeaveSummaryRow, LoanReportRow,
  CumulativeReportRow, ExportResult,
} from "@/types/reports";

export interface PrinterInfo { name: string; is_default: boolean; }

const emptySummary: ReportSummary = {
  total_employees: 0, active_employees: 0, total_departments: 0, total_periods: 0,
  closed_periods: 0, total_payroll_runs: 0, total_gross_all: 0, total_net_all: 0,
  total_tax_all: 0, total_loans_active: 0, total_loan_outstanding: 0,
  total_leave_records: 0, pending_leaves: 0,
};
const emptyExport: ExportResult = { success: false, message: "Preview mode", file_path: null };
function getToken(): string { return (typeof window !== "undefined" && sessionStorage.getItem("payroll_session_token")) || ""; }

export const reportsService = {
  getSummary: () => invokeCommand<ReportSummary>("get_report_summary", { token: getToken() }, emptySummary),
  getPayrollReport: (periodId: number | null) => invokeCommand<ReportData<PayrollReportRow>>("get_payroll_report", { token: getToken(), periodId }, { rows: [], totals: null, generated_at: "" }),
  getPayrollDetail: (periodId: number | null) => invokeCommand<ReportData<PayrollDetailRow>>("get_payroll_detail_report", { token: getToken(), periodId }, { rows: [], totals: null, generated_at: "" }),
  getStaffReport: (departmentId: number | null, status: string | null) => invokeCommand<ReportData<StaffReportRow>>("get_staff_report", { token: getToken(), departmentId, status }, { rows: [], totals: null, generated_at: "" }),
  getLeaveReport: (year: number | null, status: string | null) => invokeCommand<ReportData<LeaveReportRow>>("get_leave_report", { token: getToken(), year, status }, { rows: [], totals: null, generated_at: "" }),
  getLeaveSummary: (year: number) => invokeCommand<ReportData<LeaveSummaryRow>>("get_leave_summary_report", { token: getToken(), year }, { rows: [], totals: null, generated_at: "" }),
  getLoanReport: (status: string | null) => invokeCommand<ReportData<LoanReportRow>>("get_loan_report", { token: getToken(), status }, { rows: [], totals: null, generated_at: "" }),
  getCumulativeReport: (startDate: string | null, endDate: string | null) => invokeCommand<ReportData<CumulativeReportRow>>("get_cumulative_report", { token: getToken(), startDate, endDate }, { rows: [], totals: null, generated_at: "" }),
  exportCsv: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null = null, status: string | null = null, startDate: string | null = null, endDate: string | null = null) => invokeCommand<ExportResult>("export_report_csv", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate }, emptyExport),
  exportHtml: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null = null, status: string | null = null, startDate: string | null = null, endDate: string | null = null) => invokeCommand<ExportResult>("export_report_html", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate }, emptyExport),
  exportPdf: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null = null, status: string | null = null, startDate: string | null = null, endDate: string | null = null) => invokeCommand<ExportResult>("export_report_pdf", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate }, emptyExport),
  exportXlsx: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null = null, status: string | null = null, startDate: string | null = null, endDate: string | null = null) => invokeCommand<ExportResult>("export_report_xlsx", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate }, emptyExport),
  printReport: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null = null, status: string | null = null, startDate: string | null = null, endDate: string | null) => invokeCommand<ExportResult>("print_report", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate }, emptyExport),
  printReportConfigured: (reportType: string, periodId: number | null, year: number | null, departmentId: number | null, status: string | null, startDate: string | null, endDate: string | null, printer: string | null, paper: string, orientation: string, copies: number) => invokeCommand<ExportResult>("print_report_configured", { token: getToken(), reportType, periodId, year, departmentId, status, startDate, endDate, printer, paper, orientation, copies }, emptyExport),
  getPrinters: () => invokeCommand<PrinterInfo[]>("get_report_printers", { token: getToken() }, []),
  getOutputDirectory: () => invokeCommand<string>("get_report_output_dir", { token: getToken() }, ""),
  setOutputDirectory: (path: string) => invokeCommand<ExportResult>("set_report_output_dir", { token: getToken(), path }, emptyExport),
  exportPayslipPdf: (payrollRecordId: number) => invokeCommand<ExportResult>("export_payslip_pdf", { token: getToken(), payrollRecordId }, emptyExport),
  printPayslip: (payrollRecordId: number) => invokeCommand<ExportResult>("print_payslip", { token: getToken(), payrollRecordId }, emptyExport),
  printPayslipConfigured: (payrollRecordId: number, printer: string | null, paper: string, orientation: string, copies: number) => invokeCommand<ExportResult>("print_payslip_configured", { token: getToken(), payrollRecordId, printer, paper, orientation, copies }, emptyExport),
};
