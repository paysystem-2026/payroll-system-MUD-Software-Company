export interface ReportSummary {
  total_employees: number;
  active_employees: number;
  total_departments: number;
  total_periods: number;
  closed_periods: number;
  total_payroll_runs: number;
  total_gross_all: number;
  total_net_all: number;
  total_tax_all: number;
  total_loans_active: number;
  total_loan_outstanding: number;
  total_leave_records: number;
  pending_leaves: number;
}

export interface ReportTotals {
  total_gross: number;
  total_deductions: number;
  total_tax: number;
  total_net: number;
  employer_contributions: number;
  count: number;
}

export interface ReportData<T> {
  rows: T[];
  totals: ReportTotals | null;
  generated_at: string;
}

export interface PayrollReportRow {
  period_id: number;
  period_name: string;
  start_date: string;
  end_date: string;
  employee_count: number;
  total_gross: number;
  total_deductions: number;
  total_tax: number;
  total_net: number;
  employer_contributions: number;
}

export interface PayrollDetailRow {
  record_id: number;
  employee_id: number;
  employee_code: string;
  employee_name: string;
  department_name: string | null;
  position_title: string | null;
  base_salary: number;
  gross_earnings: number;
  total_deductions: number;
  total_tax: number;
  net_pay: number;
  employer_contributions: number;
  status: string;
  period_name: string;
  calculation_snapshot?: string | null;
}

export interface StaffReportRow {
  id: number;
  employee_code: string;
  full_name: string;
  gender: string | null;
  department_name: string | null;
  position_title: string | null;
  grade: string | null;
  employment_status: string;
  hire_date: string | null;
  base_salary: number;
  dependants: number;
  contract_type_name: string | null;
}

export interface LeaveReportRow {
  id: number;
  employee_code: string;
  employee_name: string;
  leave_type_name: string;
  start_date: string;
  end_date: string;
  days: number;
  status: string;
  reason: string | null;
}

export interface LeaveSummaryRow {
  employee_id: number;
  employee_code: string;
  employee_name: string;
  leave_type_name: string;
  entitled: number;
  used: number;
  remaining: number;
}

export interface LoanReportRow {
  id: number;
  employee_code: string;
  employee_name: string;
  principal: number;
  interest_rate: number;
  total_amount: number;
  installment_amount: number;
  total_installments: number;
  paid_installments: number;
  remaining_amount: number;
  start_date: string;
  status: string;
}

export interface CumulativeReportRow {
  period_name: string;
  employee_count: number;
  total_gross: number;
  total_deductions: number;
  total_tax: number;
  total_net: number;
  employer_contributions: number;
  loan_deductions: number;
}

export interface ExportResult {
  success: boolean;
  message: string;
  file_path: string | null;
}

export type ReportType = "payroll" | "payroll-detail" | "staff" | "leaves" | "leave-summary" | "loans" | "cumulative";
