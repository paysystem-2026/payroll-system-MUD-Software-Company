export type ReminderType = "general" | "payroll" | "leave" | "loan" | "backup" | "update";
export type ReminderRecurrence = "none" | "daily" | "weekly" | "monthly";
export interface Reminder {
  id:number; title:string; message:string|null; reminder_type:ReminderType; due_date:string;
  recurrence:ReminderRecurrence; is_completed:boolean; snoozed_until:string|null; completed_at:string|null; read_at:string|null;
  created_at:string; updated_at:string;
}
export interface ReminderInput { id?:number; title:string; message?:string|null; reminderType:ReminderType; dueDate:string; recurrence:ReminderRecurrence; }
