import { invokeCommand } from "@/services/tauri";
export interface LeaveType { id:number; name:string; code:string; default_days:number; is_paid:boolean; carry_forward:boolean; is_active:boolean }
export interface LeaveRecord { id:number; employee_id:number; employee_name:string; leave_type_id:number; leave_type_name:string; start_date:string; end_date:string; days:number; reason:string|null; status:string; created_at:string }
export interface LeaveBalance { employee_id:number; employee_name:string; leave_type_id:number; leave_type_name:string; year:number; entitled:number; used:number; remaining:number }
export interface LeaveResponse { success:boolean; message:string; id:number|null }
const response: LeaveResponse = { success:false, message:"Desktop database is unavailable in browser preview.", id:null };
export const leaveService = {
 getTypes:()=>invokeCommand<LeaveType[]>("get_leave_types", undefined, []),
 saveType:(request:unknown)=>invokeCommand<LeaveResponse>("save_leave_type",{request},response),
 deleteType:(id:number)=>invokeCommand<LeaveResponse>("delete_leave_type",{id},response),
 getRecords:(year:number)=>invokeCommand<LeaveRecord[]>("get_leave_records",{year},[]),
 saveRecord:(request:unknown)=>invokeCommand<LeaveResponse>("save_leave_record",{request},response),
 updateStatus:(leaveId:number,status:string)=>invokeCommand<LeaveResponse>("update_leave_status",{leaveId,status},response),
 getBalances:(year:number)=>invokeCommand<LeaveBalance[]>("get_leave_balances",{year},[]),
};
