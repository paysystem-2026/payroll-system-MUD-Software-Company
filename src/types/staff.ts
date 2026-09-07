export interface ContractType { id: number; name: string; code: string | null; description: string | null; is_active: boolean }

export interface ContractTypeRequest { id?: number; name: string; code?: string | null; description?: string | null }

export interface Employee {
  id: number;
  employee_code: string;
  first_name: string;
  last_name: string;
  gender: string | null;
  date_of_birth: string | null;
  national_id: string | null;
  phone: string | null;
  email: string | null;
  address: string | null;
  department_id: number | null;
  department_name: string | null;
  position_id: number | null;
  position_title: string | null;
  grade: string | null;
  hire_date: string | null;
  termination_date: string | null;
  employment_status: string;
  dependants: number;
  rssb_number: string | null;
  bank_name: string | null;
  bank_account: string | null;
  contract_type_id: number | null;
  contract_type_name: string | null;
  photo_path: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface SalaryRecord {
  id: number;
  employee_id: number;
  base_salary: number;
  effective_date: string;
  reason: string | null;
  created_at: string;
}

export interface Company { id:number; name:string; legal_name:string|null; website:string|null; tin_number:string|null; rssb_number:string|null; address:string|null; phone:string|null; email:string|null; tax_id:string|null; currency:string; logo_path:string|null }
export interface CompanyRequest { name:string; legal_name?:string|null; website?:string|null; tin_number?:string|null; rssb_number?:string|null; address?:string|null; phone?:string|null; email?:string|null; tax_id?:string|null; currency:string; logo_path?:string|null }
export interface Department {
  id: number;
  name: string;
  code: string | null;
}

export interface Position {
  id: number;
  title: string;
  code: string | null;
  department_id: number;
}

export interface SaveEmployeeRequest {
  id?: number;
  employee_code: string;
  first_name: string;
  last_name: string;
  gender?: string | null;
  date_of_birth?: string | null;
  national_id?: string | null;
  phone?: string | null;
  email?: string | null;
  address?: string | null;
  department_id?: number | null;
  position_id?: number | null;
  grade?: string | null;
  hire_date?: string | null;
  employment_status: string;
  dependants?: number | null;
  rssb_number?: string | null;
  bank_name?: string | null;
  bank_account?: string | null;
  contract_type_id?: number | null;
  photo_path?: string | null;
  base_salary?: number | null;
  salary_effective_date?: string | null;
  salary_reason?: string | null;
}

export interface AddSalaryRequest {
  employee_id: number;
  base_salary: number;
  effective_date: string;
  reason?: string | null;
}

export interface DepartmentRequest { id?:number; name:string; code?:string|null }
export interface PositionRequest { id?:number; department_id:number; title:string; code?:string|null }

export interface StaffResponse {
  success: boolean;
  message: string;
  id: number | null;
}

export interface EmployeePayrollOverride {
  id:number; employee_id:number; rule_id:number; rule_name:string; rule_code:string;
  override_type:"fixed"|"percentage"|"formula"; value:number|null; formula_expression:string|null;
  base_reference:string|null; effective_date:string; is_active:boolean;
}
export interface EmployeePayrollOverrideRequest {
  id?:number; employee_id:number; rule_id:number; override_type:"fixed"|"percentage"|"formula";
  value?:number|null; formula_expression?:string|null; base_reference?:string|null; effective_date:string; is_active?:boolean;
}
