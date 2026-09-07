import { invokeCommand } from "@/services/tauri";
import type { Notification } from "@/types/notifications";
export const notificationService = {
  list(token:string){ return invokeCommand<Notification[]>("get_notifications", {token}, []); },
  markRead(token:string, notificationId:number){ return invokeCommand<void>("mark_notification_read", {token, notificationId}, undefined); },
  markAllRead(token:string){ return invokeCommand<void>("mark_all_notifications_read", {token}, undefined); },
};
