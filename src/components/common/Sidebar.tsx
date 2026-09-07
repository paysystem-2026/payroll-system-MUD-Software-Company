import gorillaDoctorsLogo from "@/assets/logo/gorilla-doctors.jpeg";
import { NavLink } from "react-router-dom";
import {
  LayoutDashboard, Database, Users, CalendarDays, Wallet, FileText,
  Bell, HardDriveDownload, Network, RefreshCw, Settings,
  type LucideIcon,
} from "lucide-react";
import { NAV_ITEMS } from "@/constants/navigation";

const ICONS: Record<string, LucideIcon> = {
  LayoutDashboard, Database, Users, CalendarDays, Wallet, FileText,
  Bell, HardDriveDownload, Network, RefreshCw, Settings,
};

export function Sidebar() {
  return (
    <aside className="hidden w-[248px] shrink-0 flex-col border-r border-[#292929] bg-[#151515] lg:flex">
      <div className="flex h-[76px] items-center gap-3 border-b border-[#292929] px-5">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-white/95 shadow-[0_0_24px_rgba(74,139,63,0.18)] ring-1 ring-[#4a8b3f]/30">
          <img
            src={gorillaDoctorsLogo}
            alt="Gorilla Doctors"
            className="h-full w-full object-contain p-0.5"
          />
        </div>
        <div className="min-w-0">
          <p className="truncate text-[14px] font-semibold text-white">Payroll System</p>
          <p className="mt-0.5 text-[11px] text-[#777777]">Version 1.0</p>
        </div>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 py-4">
        <p className="mb-2 px-3 text-[10px] font-semibold uppercase tracking-[0.16em] text-[#5f5f5f]">
          Workspace
        </p>
        <div className="space-y-1">
          {NAV_ITEMS.map((item) => {
            const Icon = ICONS[item.icon];
            return (
              <NavLink
                key={item.id}
                to={item.path}
                end={item.id === "dashboard"}
                className={({ isActive }) =>
                  `group flex items-center gap-3 rounded-xl px-3.5 py-3 text-[13px] font-medium transition-all duration-200 ${
                    isActive
                      ? "bg-[#4a8b3f]/12 text-[#67ab5a] shadow-[inset_0_0_0_1px_rgba(74,139,63,0.22)]"
                      : "text-[#8b8b8b] hover:bg-[#1d1d1d] hover:text-[#f0f0f0]"
                  }`
                }
              >
                {Icon && (
                  <Icon className="h-[18px] w-[18px] shrink-0 transition-transform duration-200 group-hover:scale-105" />
                )}
                <span className="truncate">{item.label}</span>
              </NavLink>
            );
          })}
        </div>
      </nav>

      <div className="border-t border-[#292929] p-4">
        <div className="rounded-xl border border-[#292929] bg-[#101010] px-3 py-2.5">
          <p className="text-[11px] font-medium text-[#d9d9d9]">Offline desktop app</p>
          <p className="mt-0.5 text-[10px] text-[#666666]">Secure • Fast • Local</p>
        </div>
      </div>
    </aside>
  );
}
