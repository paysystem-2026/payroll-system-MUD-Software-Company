export interface PayrollRule {
  id: number;
  name: string;
  code: string;
  component_type: string;
  calc_type: string;
  side: string;
  rate: number | null;
  formula_expression: string | null;
  base_reference: string | null;
  is_taxable: boolean;
  is_pensionable: boolean;
  sort_order: number;
  effective_date: string;
  is_active: boolean;
  version: number;
}

export interface RuleListResponse { rules: PayrollRule[]; errors: string[]; }
export interface SaveRuleRequest {
  id?: number; name: string; code: string; component_type: string; calc_type: string; side: string;
  rate?: number | null; formula_expression?: string | null; base_reference?: string | null;
  is_taxable: boolean; is_pensionable: boolean; sort_order: number;
}
export interface SaveRuleResponse { success: boolean; message: string; rule_id: number | null; errors: string[]; }
export interface RuleVersion { id:number; rule_id:number; version:number; name:string; code:string; component_type:string; calc_type:string; side:string; rate:number|null; formula_expression:string|null; base_reference:string|null; is_taxable:boolean; is_pensionable:boolean; sort_order:number; effective_date:string; created_at:string; }
export interface VersionHistory { versions: RuleVersion[]; }
export interface CalcItem { code:string; name:string; component_type:string; calc_type:string; side:string; rate:number|null; formula:string|null; base_reference:string|null; amount:string; }
export interface CalcResult { items:CalcItem[]; gross_earnings:string; total_deductions:string; total_tax:string; net_pay:string; employer_contributions:string; errors:string[]; }
export interface FormulaTestResult { success:boolean; result:string|null; error:string|null; breakdown:CalcItem[]; }
export interface TestInput { code:string; value:number; }
export interface TestFormulaRequest { expression:string; inputs:TestInput[]; }

export interface PayrollPeriod {
  id:number; period_name:string; start_date:string; end_date:string; pay_date:string|null; status:string;
  config_version:number; record_count:number; total_gross:number; total_deductions:number; total_tax:number; total_net:number; employer_contributions:number;
}
export interface PayrollPeriodRequest { id?:number; period_name:string; start_date:string; end_date:string; pay_date?:string|null; }
export interface PayrollPeriodResponse { success:boolean; message:string; id:number|null; }
export interface PayrollRecord {
  id:number; employee_id:number; employee_code:string; employee_name:string; department_name:string|null; position_title:string|null;
  base_salary:number; gross_earnings:number; total_deductions:number; total_tax:number; net_pay:number; employer_contributions:number;
  status:string; config_version:number; calculation_snapshot:string|null;
}
export interface PayrollRunResponse { success:boolean; message:string; period_id:number; records:PayrollRecord[]; errors:string[]; }
export interface Loan {
  id:number; employee_id:number; employee_name:string; principal:number; interest_rate:number; total_amount:number;
  installment_amount:number; total_installments:number; paid_installments:number; remaining_amount:number; start_date:string; status:string;
}
export interface LoanRequest { id?:number; employee_id:number; principal:number; interest_rate:number; total_amount:number; installment_amount:number; total_installments:number; start_date:string; }
export interface Payslip {
  id:number; payroll_record_id:number; employee_id:number; employee_name:string; employee_code:string; period_name:string;
  payslip_number:string; net_pay:number; generated_at:string; calculation_snapshot:string|null;
}
