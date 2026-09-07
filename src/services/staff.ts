import type { Employee, SalaryRecord, Department, Position, Company, CompanyRequest, DepartmentRequest, PositionRequest, SaveEmployeeRequest, AddSalaryRequest, StaffResponse, ContractType, ContractTypeRequest, EmployeePayrollOverride, EmployeePayrollOverrideRequest } from "@/types/staff";
import { invokeCommand } from "@/services/tauri";

const response: StaffResponse = { success: false, message: "Desktop database is unavailable in browser preview.", id: null };
export const staffService = {
  getEmployees: () => invokeCommand<Employee[]>("get_employees", undefined, []),
  getEmployee: (employeeId: number) => invokeCommand<Employee | null>("get_employee", { employeeId }, null),
  saveEmployee: (request: SaveEmployeeRequest) => invokeCommand<StaffResponse>("save_employee", { request }, response),
  deleteEmployee: (employeeId: number) => invokeCommand<StaffResponse>("delete_employee", { employeeId }, response),
  getSalaryHistory: (employeeId: number) => invokeCommand<SalaryRecord[]>("get_salary_history", { employeeId }, []),
  addSalaryRecord: (request: AddSalaryRequest) => invokeCommand<StaffResponse>("add_salary_record", { request }, response),
  getDepartments: () => invokeCommand<Department[]>("get_departments", undefined, []),
  getPositions: (departmentId?: number) => invokeCommand<Position[]>("get_positions", { departmentId }, []),
  getCompany: () => invokeCommand<Company | null>("get_company", undefined, null),
  saveCompany: (request: CompanyRequest) => invokeCommand<StaffResponse>("save_company", { request }, response),
  saveDepartment: (request: DepartmentRequest) => invokeCommand<StaffResponse>("save_department", { request }, response),
  deleteDepartment: (id: number) => invokeCommand<StaffResponse>("delete_department", { id }, response),
  savePosition: (request: PositionRequest) => invokeCommand<StaffResponse>("save_position", { request }, response),
  deletePosition: (id: number) => invokeCommand<StaffResponse>("delete_position", { id }, response),
  getContractTypes: () => invokeCommand<ContractType[]>("get_contract_types", undefined, []),
  saveContractType: (request: ContractTypeRequest) => invokeCommand<StaffResponse>("save_contract_type", { request }, response),
  deleteContractType: (id: number) => invokeCommand<StaffResponse>("delete_contract_type", { id }, response),
  generateEmployeeCode: () => invokeCommand<string>("generate_employee_code", undefined, `EMP-${Date.now().toString(36).toUpperCase()}`),
  getEmployeePayrollOverrides: (employeeId:number) => invokeCommand<EmployeePayrollOverride[]>("get_employee_payroll_overrides", { employeeId }, []),
  saveEmployeePayrollOverride: (request:EmployeePayrollOverrideRequest) => invokeCommand<StaffResponse>("save_employee_payroll_override", { request }, response),
  deleteEmployeePayrollOverride: (id:number) => invokeCommand<StaffResponse>("delete_employee_payroll_override", { id }, response),
};
