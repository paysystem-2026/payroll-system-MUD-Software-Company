import { Search, Bell, UserCircle, Lock, LogOut, Users, CalendarDays, Wallet, Database, BarChart3, RefreshCw, ShieldCheck, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { staffService } from "@/services/staff";
import { searchService, type GlobalSearchResult } from "@/services/search";
import { notificationService } from "@/services/notifications";
import { settingsService } from "@/services/settings";
import { useAuthContext } from "@/stores/authContext";
import type { Notification } from "@/types/notifications";
import type { Employee } from "@/types/staff";

interface HeaderProps {
  username: string | null;
  onLock: () => void;
  onLogout: () => void;
}

export function Header({ username, onLock, onLogout }: HeaderProps) {
  const navigate = useNavigate();
  const location = useLocation();
  const auth = useAuthContext();
  const token = auth.token;
  const [query, setQuery] = useState("");
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [globalResults, setGlobalResults] = useState<GlobalSearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [open, setOpen] = useState(false);
  const [notificationsOpen, setNotificationsOpen] = useState(false);
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [notificationBusy, setNotificationBusy] = useState<number | null>(null);
  const [notificationPollingSeconds, setNotificationPollingSeconds] = useState(15);
  const searchRef = useRef<HTMLDivElement>(null);

  // Close the search whenever the user clicks anywhere outside it (sidebar, page content,
  // toolbar, etc.) or when navigation changes. This prevents the dropdown from lingering
  // after the user abandons the search.
  useEffect(() => {
    if (!open) return;
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (target && !searchRef.current?.contains(target)) {
        setOpen(false);
        setQuery("");
      }
    };
    document.addEventListener("pointerdown", handlePointerDown, true);
    return () => document.removeEventListener("pointerdown", handlePointerDown, true);
  }, [open]);

  useEffect(() => {
    setOpen(false);
    setQuery("");
  }, [location.pathname, location.search]);

  // Do not load the full employee table during every app startup. Employee
  // search is lazy: fetch it only when the user actually starts searching.
  useEffect(() => {
    if (query.trim().length < 2 || employees.length > 0) return;
    let active = true;
    void staffService.getEmployees().then((items) => { if (active) setEmployees(items); }).catch(() => { if (active) setEmployees([]); });
    return () => { active = false; };
  }, [query, employees.length]);

  useEffect(() => {
    const q = query.trim();
    if (q.length < 2 || !token) { setGlobalResults([]); setSearching(false); return; }
    let active = true;
    setSearching(true);
    const timer = window.setTimeout(() => {
      void searchService.search(token, q, 40).then((items) => {
        if (active) setGlobalResults(Array.isArray(items) ? items : []);
      }).catch(() => { if (active) setGlobalResults([]); })
        .finally(() => { if (active) setSearching(false); });
    }, 220);
    return () => { active = false; window.clearTimeout(timer); };
  }, [query, token]);

  const loadNotifications = async () => {
    if (!token) return;
    try {
      const items = await notificationService.list(token);
      setNotifications(Array.isArray(items) ? items.filter(Boolean).slice(0, 30) : []);
    } catch {
      setNotifications([]);
    }
  };

  useEffect(() => {
    let active = true;
    void settingsService.getReminderSettings().then((s) => {
      if (active) setNotificationPollingSeconds(Math.max(5, s.polling_interval_seconds));
    }).catch(() => undefined);
    return () => { active = false; };
  }, [token]);

  useEffect(() => {
    if (!token) { setNotifications([]); return; }
    void loadNotifications();
    const timer = window.setInterval(() => void loadNotifications(), notificationPollingSeconds * 1000);
    const onFocus = () => void loadNotifications();
    window.addEventListener("focus", onFocus);
    return () => { window.clearInterval(timer); window.removeEventListener("focus", onFocus); };
  }, [token, notificationPollingSeconds]);

  const openNotification = async (notification: Notification) => {
    if (!token || notificationBusy === notification.id) return;
    setNotificationBusy(notification.id);
    try {
      await notificationService.markRead(token, notification.id);
      setNotifications(current => current.filter(item => item.id !== notification.id));
      setNotificationsOpen(false);
      navigate(notification.routeQuery ? `${notification.route}?${notification.routeQuery}` : notification.route);
    } finally {
      setNotificationBusy(null);
    }
  };

  const markAllNotificationsRead = async () => {
    if (!token || notifications.length === 0) return;
    setNotificationBusy(0);
    try {
      await notificationService.markAllRead(token);
      setNotifications([]);
    } finally {
      setNotificationBusy(null);
    }
  };

  const pages = useMemo(() => [
    { label: "Dashboard", path: "/", icon: BarChart3, keywords: "home dashboard overview" },
    { label: "Basic Data", path: "/basic-data", icon: Database, keywords: "company departments positions payroll configuration" },
    { label: "Staff Records", path: "/staff", icon: Users, keywords: "employees staff worker records" },
    { label: "Leaves", path: "/leaves", icon: CalendarDays, keywords: "leave vacation balance history" },
    { label: "Payments / Payroll", path: "/payroll", icon: Wallet, keywords: "pay payroll salary payslip" },
    { label: "Reports", path: "/reports", icon: BarChart3, keywords: "reports export pdf excel csv print" },
    { label: "Reminders", path: "/reminders", icon: Bell, keywords: "reminders notifications due alerts" },
    { label: "Backup & Restore", path: "/backup", icon: Database, keywords: "backup restore database recovery" },
    { label: "LAN Transfer", path: "/lan-transfer", icon: Database, keywords: "lan transfer devices network" },
    { label: "Updates", path: "/updates", icon: RefreshCw, keywords: "updates version updater" },
    { label: "Administration", path: "/administration", icon: ShieldCheck, keywords: "administration audit security system" },
    { label: "Settings", path: "/settings", icon: Database, keywords: "settings configuration language currency" },
  ], []);

  const matches = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return { pages: pages.slice(0, 4), employees: [] as Employee[], data: [] as GlobalSearchResult[] };
    const pageMatches = pages.filter((p) => `${p.label} ${p.keywords}`.toLowerCase().includes(q));
    const employeeMatches = employees.filter((e) => `${e.employee_code} ${e.first_name} ${e.last_name} ${e.phone ?? ""} ${e.email ?? ""}`.toLowerCase().includes(q)).slice(0, 6);
    const employeeIds = new Set(employeeMatches.map((e) => e.id));
    const data = globalResults.filter((r) => !(r.table === "employees" && r.record_id != null && employeeIds.has(r.record_id))).slice(0, 20);
    return { pages: pageMatches.slice(0, 5), employees: employeeMatches, data };
  }, [employees, globalResults, pages, query]);

  const choosePage = (path: string) => { setQuery(""); setOpen(false); navigate(path); };
  const chooseEmployee = (employee: Employee) => {
    setQuery("");
    setOpen(false);
    // Pass both route state and a URL parameter so the result opens reliably
    // even when the Staff page is already mounted.
    const nonce = Date.now().toString();
    navigate(`/staff?employee=${encodeURIComponent(employee.id)}&searchNonce=${nonce}`, {
      state: { employeeId: employee.id, searchNonce: nonce },
    });
  };

  const chooseResult = (result: GlobalSearchResult) => {
    setQuery("");
    setOpen(false);
    if (result.table === "employees" && result.record_id != null) {
      const nonce = Date.now().toString();
      navigate(`/staff?employee=${encodeURIComponent(result.record_id)}&searchNonce=${nonce}`, { state: { employeeId: result.record_id, searchNonce: nonce } });
      return;
    }
    navigate(`${result.route}?search=${encodeURIComponent(result.matched_value || query.trim())}`);
  };

  return (
    <header className="relative z-[80] flex h-[76px] shrink-0 items-center justify-between border-b border-[#292929] bg-[#111111]/95 px-5 backdrop-blur lg:px-6">
      <div className="relative z-[2] flex min-w-0 flex-1 items-center gap-4">
        <div ref={searchRef} className="relative w-full max-w-[420px]">
          <div className="relative z-[2] flex h-10 items-center gap-2.5 rounded-xl border border-[#2a2a2a] bg-[#171717] px-3.5 transition-all duration-200 focus-within:border-[#4a8b3f]/60 focus-within:bg-[#1a1a1a]">
            <Search className="h-4 w-4 shrink-0 text-[#6f6f6f]" />
            <input
              value={query}
              onChange={(event) => { setQuery(event.target.value); setOpen(true); }}
              onFocus={() => setOpen(true)}
              onKeyDown={(event) => { if (event.key === "Escape") { setOpen(false); setQuery(""); } }}
              type="text"
              placeholder="Search pages, employees, codes..."
              className="w-full bg-transparent text-[13px] text-white placeholder:text-[#666666] focus:outline-none"
              aria-label="Global search"
            />
            {query && <button type="button" aria-label="Clear search" onPointerDown={(event) => event.stopPropagation()} onClick={() => { setQuery(""); setOpen(true); }} className="grid h-7 w-7 shrink-0 place-items-center rounded-lg text-[#777] transition hover:bg-[#242424] hover:text-white"><X className="h-3.5 w-3.5" /></button>}
          </div>
          {open && (query.trim() || matches.employees.length || matches.pages.length || matches.data.length) && (
            <div className="absolute left-0 right-0 top-[48px] z-[100] max-h-[min(70vh,620px)] overflow-y-auto overscroll-contain rounded-2xl border border-[#2d2d2d] bg-[#141414] p-2 shadow-[0_24px_60px_rgba(0,0,0,0.45)] animate-[slide-up_.18s_ease-out]">
              <p className="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[#6d6d6d]">Quick search</p>
              {matches.pages.map((item) => { const Icon = item.icon; return <button key={item.path} type="button" onClick={() => choosePage(item.path)} className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm text-[#dddddd] transition hover:bg-[#1d1d1d] hover:text-white"><Icon className="h-4 w-4 text-[#68a85f]" /><span>{item.label}</span><span className="ml-auto text-[10px] text-[#666]">Page</span></button>; })}
              {matches.employees.length > 0 && <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-[#6d6d6d]">Employees</p>}
              {matches.employees.map((employee) => <button key={employee.id} type="button" onClick={() => chooseEmployee(employee)} className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition hover:bg-[#1d1d1d]"><div className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-[#4a8b3f]/12 text-[#6aac60]"><Users className="h-4 w-4" /></div><div className="min-w-0"><p className="truncate text-sm font-medium text-white">{employee.first_name} {employee.last_name}</p><p className="text-[10px] text-[#737373]">{employee.employee_code}</p></div></button>)}
              {matches.data.length > 0 && <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-[#6d6d6d]">All system data</p>}
              {matches.data.map((result, index) => <button key={`${result.table}-${result.record_id ?? "x"}-${result.matched_field}-${index}`} type="button" onClick={() => chooseResult(result)} className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition hover:bg-[#1d1d1d]"><div className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-[#4a8b3f]/10 text-[#75ae6d]"><Search className="h-4 w-4" /></div><div className="min-w-0 flex-1"><p className="truncate text-[12px] font-medium text-white">{result.title}</p><p className="truncate text-[10px] text-[#737373]">{result.subtitle}</p></div><span className="shrink-0 max-w-[110px] truncate text-[9px] uppercase tracking-wide text-[#555]">{result.table.replace(/_/g, " ")}</span></button>)}
              {searching && query.trim().length >= 2 && <p className="px-3 py-2 text-center text-[10px] text-[#666]">Searching all system data…</p>}
              {query.trim() && !searching && matches.pages.length === 0 && matches.employees.length === 0 && matches.data.length === 0 && <p className="px-3 py-4 text-center text-sm text-[#777]">No matching data found.</p>}
            </div>
          )}
        </div>
      </div>

      <div className="relative z-[2] ml-4 flex items-center gap-1.5">
        <div className="relative">
          <button type="button" onClick={() => { setNotificationsOpen(v => !v); if (!notificationsOpen) void loadNotifications(); }} title="Notifications" aria-label={`Notifications${notifications.length ? `, ${notifications.length} unread` : ""}`} className="relative rounded-xl p-2.5 text-[#7d7d7d] transition-all duration-200 hover:bg-[#1d1d1d] hover:text-[#67ab5a]">
            <Bell className="h-[18px] w-[18px]" />
            {notifications.length > 0 && <span className="absolute right-1 top-1 grid min-h-[15px] min-w-[15px] place-items-center rounded-full bg-[#4a8b3f] px-1 text-[8px] font-bold text-white ring-2 ring-[#111111]">{notifications.length > 99 ? "99+" : notifications.length}</span>}
          </button>
          {notificationsOpen && <div className="absolute right-0 top-[48px] z-[100] w-[360px] overflow-hidden rounded-2xl border border-[#2d2d2d] bg-[#141414] shadow-[0_24px_60px_rgba(0,0,0,0.55)]">
            <div className="flex items-center justify-between border-b border-[#292929] px-4 py-3">
              <div><p className="text-sm font-semibold text-white">Notifications</p><p className="text-[10px] text-[#6f6f6f]">System events and due reminders</p></div>
              <div className="flex items-center gap-2"><span className="rounded-full bg-[#4a8b3f]/15 px-2 py-1 text-[10px] font-semibold text-[#79b672]">{notifications.length} unread</span>{notifications.length > 0 && <button type="button" onClick={() => void markAllNotificationsRead()} disabled={notificationBusy !== null} className="text-[10px] font-semibold text-[#79b672] hover:text-white disabled:opacity-50">Mark all read</button>}</div>
            </div>
            {notifications.length === 0 ? <div className="px-4 py-9 text-center"><Bell className="mx-auto h-6 w-6 text-[#4a8b3f]"/><p className="mt-2 text-xs text-[#777]">No unread notifications.</p></div> : <div className="max-h-[420px] overflow-y-auto overscroll-contain p-2">{notifications.map(item => <button key={`${item.id}-${item.kind}`} type="button" disabled={notificationBusy === item.id} onClick={() => void openNotification(item)} className="flex w-full items-start gap-3 rounded-xl px-3 py-3 text-left transition hover:bg-[#1d1d1d] disabled:opacity-60">
              <div className="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-[#4a8b3f]/12 text-[#6aac60]"><Bell className="h-4 w-4"/></div>
              <div className="min-w-0 flex-1"><p className="truncate text-[12px] font-semibold text-white">{item.title}</p><p className="mt-0.5 line-clamp-2 text-[10px] text-[#8a8a8a]">{item.message}</p><div className="mt-1 flex items-center gap-2"><span className="text-[9px] uppercase tracking-wider text-[#5f5f5f]">{item.kind}</span><span className="text-[9px] text-[#4a4a4a]">{item.createdAt}</span></div></div>
            </button>)}</div>}
          </div>}
        </div>
        <button type="button" onClick={onLock} title="Lock session" className="rounded-xl p-2.5 text-[#7d7d7d] transition-all duration-200 hover:bg-[#1d1d1d] hover:text-[#67ab5a]"><Lock className="h-[18px] w-[18px]" /></button>
        <button type="button" onClick={onLogout} title="Sign out" className="rounded-xl p-2.5 text-[#7d7d7d] transition-all duration-200 hover:bg-[#1d1d1d] hover:text-[#67ab5a]"><LogOut className="h-[18px] w-[18px]" /></button>
        <div className="ml-2 flex items-center gap-2.5 border-l border-[#292929] pl-4"><div className="flex h-9 w-9 items-center justify-center rounded-xl bg-[#1f1f1f] ring-1 ring-[#2c2c2c]"><UserCircle className="h-5 w-5 text-[#8f8f8f]" /></div><div className="hidden sm:block"><p className="text-[12px] font-semibold text-white">{username || "Administrator"}</p><p className="mt-0.5 text-[10px] text-[#6f6f6f]">Local account</p></div></div>
      </div>
    </header>
  );
}
