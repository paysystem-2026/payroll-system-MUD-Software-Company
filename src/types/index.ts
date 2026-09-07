export type ModuleId =
  | "dashboard"
  | "basic-data"
  | "staff"
  | "leaves"
  | "payroll"
  | "reports"
  | "reminders"
  | "backup"
  | "lan-transfer"
  | "updates"
  | "administration"
  | "settings";

export interface NavItem {
  id: ModuleId;
  label: string;
  icon: string;
  path: string;
}

export interface PageHeaderProps {
  title: string;
  description?: string;
}
export type { LanDevice } from "./lanTransfer";
