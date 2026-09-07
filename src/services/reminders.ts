import { invokeCommand } from "@/services/tauri";
import type { Reminder, ReminderInput } from "@/types/reminders";
export const reminderService = {
  list(token:string){ return invokeCommand<Reminder[]>("get_reminders",{token},[]); },
  due(token:string){ return invokeCommand<Reminder[]>("get_due_reminders",{token},[]); },
  unreadDue(token:string){ return invokeCommand<Reminder[]>("get_unread_due_reminders",{token},[]); },
  markRead(token:string, reminderId:number){ return invokeCommand<Reminder>("mark_reminder_read",{token,reminderId}, {} as Reminder); },
  save(token:string, reminder:ReminderInput){ return invokeCommand<Reminder>("save_reminder",{token,reminder}, {} as Reminder); },
  complete(token:string, reminderId:number){ return invokeCommand<Reminder>("complete_reminder",{token,reminderId}, {} as Reminder); },
  snooze(token:string, reminderId:number, minutes:number){ return invokeCommand<Reminder>("snooze_reminder",{token,reminderId,minutes}, {} as Reminder); },
  remove(token:string, reminderId:number){ return invokeCommand<void>("delete_reminder",{token,reminderId}, undefined); },
};
