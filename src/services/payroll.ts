import type {
  CalcResult, FormulaTestResult, Loan, LoanRequest, PayrollPeriod, PayrollPeriodRequest,
  PayrollPeriodResponse, PayrollRecord, PayrollRunResponse, Payslip,
  RuleListResponse, SaveRuleRequest, SaveRuleResponse, TestFormulaRequest, VersionHistory,
} from "@/types/payroll";
import { invokeCommand } from "@/services/tauri";

const response: SaveRuleResponse = { success:true, message:"Ready in preview mode.", rule_id:null, errors:[] };
const periodResponse: PayrollPeriodResponse = { success:true, message:"Ready in preview mode.", id:null };
const runResponse: PayrollRunResponse = { success:true, message:"Ready in preview mode.", period_id:0, records:[], errors:[] };

export const payrollService = {
  getRules: () => invokeCommand<RuleListResponse>("get_payroll_rules", undefined, { rules: [], errors: [] }),
  saveRule: (request: SaveRuleRequest) => invokeCommand<SaveRuleResponse>("save_payroll_rule", { request }, response),
  toggleRule: (ruleId:number,isActive:boolean) => invokeCommand<SaveRuleResponse>("toggle_payroll_rule", { ruleId, isActive }, response),
  deleteRule: (ruleId:number) => invokeCommand<SaveRuleResponse>("delete_payroll_rule", { ruleId }, response),
  getVersions: (ruleId:number) => invokeCommand<VersionHistory>("get_rule_versions", { ruleId }, { versions: [] } as VersionHistory),
  testFormula: (request:TestFormulaRequest) => invokeCommand<FormulaTestResult>("test_formula", { request }, { success:false,result:null,error:null,breakdown:[] }),
  previewCalculation: (basicSalary:number) => invokeCommand<CalcResult>("preview_calculation", { request:{ basic_salary:basicSalary } }, { items:[],gross_earnings:"0",total_deductions:"0",total_tax:"0",net_pay:"0",employer_contributions:"0",errors:[] }),
  validateRules: () => invokeCommand<string[]>("validate_rules", undefined, []),
  getPeriods: () => invokeCommand<PayrollPeriod[]>("get_payroll_periods", undefined, []),
  savePeriod: (request:PayrollPeriodRequest) => invokeCommand<PayrollPeriodResponse>("save_payroll_period", { request }, periodResponse),
  deletePeriod: (periodId:number) => invokeCommand<PayrollPeriodResponse>("delete_payroll_period", { periodId }, periodResponse),
  calculatePeriod: (periodId:number) => invokeCommand<PayrollRunResponse>("calculate_payroll_period", { periodId }, runResponse),
  getRecords: (periodId:number) => invokeCommand<PayrollRecord[]>("get_payroll_records", { periodId }, []),
  finalizePeriod: (periodId:number) => invokeCommand<PayrollRunResponse>("finalize_payroll_period", { periodId }, runResponse),
  getLoans: () => invokeCommand<Loan[]>("get_loans", undefined, []),
  saveLoan: (request:LoanRequest) => invokeCommand<PayrollPeriodResponse>("save_loan", { request }, periodResponse),
  getPayslips: () => invokeCommand<Payslip[]>("get_payslips", undefined, []),
};
