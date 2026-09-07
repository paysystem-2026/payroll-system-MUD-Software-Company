import type { NavItem } from "@/types";

export const NAV_ITEMS: NavItem[] = [
  { id: "dashboard", label: "Dashboard", icon: "LayoutDashboard", path: "/" },
  { id: "basic-data", label: "Basic Data", icon: "Database", path: "/basic-data" },
  { id: "staff", label: "Staff", icon: "Users", path: "/staff" },
  { id: "leaves", label: "Leaves", icon: "CalendarDays", path: "/leaves" },
  { id: "payroll", label: "Payments", icon: "Wallet", path: "/payroll" },
  { id: "reports", label: "Reports", icon: "FileText", path: "/reports" },
  { id: "reminders", label: "Reminders", icon: "Bell", path: "/reminders" },
  { id: "backup", label: "Backup & Restore", icon: "HardDriveDownload", path: "/backup" },
  { id: "lan-transfer", label: "LAN Transfer", icon: "Network", path: "/lan-transfer" },
  { id: "updates", label: "Updates", icon: "RefreshCw", path: "/updates" },
  { id: "administration", label: "Administration", icon: "ShieldCheck", path: "/administration" },
  { id: "settings", label: "Settings", icon: "Settings", path: "/settings" },
];
