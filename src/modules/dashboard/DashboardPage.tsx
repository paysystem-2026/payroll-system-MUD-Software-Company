import { useEffect, useState } from "react";
import {
  ArrowRight,
  Building2,
  CalendarDays,
  CheckCircle2,
  Circle,
  Clock3,
  FileCog,
  Plus,
  Users,
  BriefcaseBusiness,
} from "lucide-react";
import { useNavigate } from "react-router-dom";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { staffService } from "@/services/staff";
import { leaveService } from "@/types/leaves";

interface DashboardData {
  employees: number;
  active: number;
  departments: number;
  positions: number;
  leaveRecords: number;
  leaveBalances: number;
}

const EMPTY_DATA: DashboardData = {
  employees: 0,
  active: 0,
  departments: 0,
  positions: 0,
  leaveRecords: 0,
  leaveBalances: 0,
};

export function DashboardPage() {
  const nav = useNavigate();
  const [data, setData] = useState<DashboardData>(EMPTY_DATA);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;
    const year = new Date().getFullYear();

    Promise.all([
      staffService.getEmployees(),
      staffService.getDepartments(),
      staffService.getPositions(),
      leaveService.getRecords(year),
      leaveService.getBalances(year),
    ])
      .then(([employees, departments, positions, leaveRecords, leaveBalances]) => {
        if (!mounted) return;
        setData({
          employees: employees.length,
          active: employees.filter((employee) => employee.employment_status === "active").length,
          departments: departments.length,
          positions: positions.length,
          leaveRecords: leaveRecords.length,
          leaveBalances: leaveBalances.length,
        });
      })
      .catch(() => {
        if (mounted) setData(EMPTY_DATA);
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });

    return () => {
      mounted = false;
    };
  }, []);

  const statCards = [
    { label: "Total Employees", value: data.employees, icon: Users },
    { label: "Total Departments", value: data.departments, icon: Building2 },
    { label: "Active Employees", value: data.active, icon: CheckCircle2 },
    { label: "Positions", value: data.positions, icon: BriefcaseBusiness },
  ];

  return (
    <div className="space-y-5 pb-8">
      <section className="dashboard-reveal relative overflow-hidden rounded-[26px] border border-white/10 bg-[#0b0b0b] px-5 py-5 shadow-[0_24px_60px_rgba(0,0,0,0.34)] lg:px-6 lg:py-6">
        <div className="pointer-events-none absolute -right-20 -top-24 h-72 w-72 rounded-full bg-[#4a8b3f]/20 blur-3xl" />
        <div className="pointer-events-none absolute bottom-0 left-1/3 h-24 w-64 rounded-full bg-white/[0.03] blur-3xl" />
        <div className="relative flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
          <div>
            <div className="mb-2 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.16em] text-[#67b85a]">
              <Circle className="h-2.5 w-2.5 fill-current" />
              Payroll System V1
            </div>
            <h1 className="text-[28px] font-semibold tracking-[-0.04em] text-white sm:text-[32px]">Dashboard</h1>
            <p className="mt-2 max-w-2xl text-[13px] leading-6 text-white/55">
              A clean overview of your people, organisation and leave activity.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <div className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.04] px-3.5 py-2 text-[11px] font-semibold text-white/65">
              <span className="h-2 w-2 rounded-full bg-[#4a8b3f] shadow-[0_0_14px_rgba(74,139,63,0.75)]" />
              Phase 1 Ready
            </div>
            <Button onClick={() => nav("/staff")}>
              <Plus className="h-4 w-4" />
              Register Employee
            </Button>
          </div>
        </div>
      </section>

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {statCards.map(({ label, value, icon: Icon }, index) => (
          <div key={label} className="dashboard-reveal" style={{ animationDelay: `${index * 70}ms` }}>
            <MetricCard icon={Icon} label={label} value={value} loading={loading} />
          </div>
        ))}
      </section>

      <section className="grid gap-3 xl:grid-cols-[1.35fr_1fr]">
        <div className="dashboard-reveal rounded-[20px] border border-white/10 bg-white/[0.035] p-4 shadow-[0_18px_45px_rgba(0,0,0,0.2)]" style={{ animationDelay: "300ms" }}>
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.14em] text-[#67b85a]">
                <CalendarDays className="h-4 w-4" />
                Leave overview
              </div>
              <h2 className="mt-2 text-[16px] font-semibold text-white">Current year activity</h2>
              <p className="mt-1 text-[12px] text-white/45">Records and available employee balances.</p>
            </div>
            <Button variant="ghost" onClick={() => nav("/leaves")}>
              Open Leaves
              <ArrowRight className="h-4 w-4" />
            </Button>
          </div>

          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            <StatTile label="Leave Records" value={data.leaveRecords} icon={Clock3} />
            <StatTile label="Tracked Balances" value={data.leaveBalances} icon={Users} />
          </div>
        </div>

        <div className="dashboard-reveal rounded-[20px] border border-white/10 bg-[#0b0b0b] p-4 shadow-[0_18px_45px_rgba(0,0,0,0.2)]" style={{ animationDelay: "370ms" }}>
          <div className="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.14em] text-[#67b85a]">
            <CheckCircle2 className="h-4 w-4" />
            Essentials
          </div>
          <h2 className="mt-2 text-[16px] font-semibold text-white">Core setup status</h2>
          <div className="mt-4 space-y-2">
            <StatusRow label="Employee records" ready={data.employees > 0} />
            <StatusRow label="Departments" ready={data.departments > 0} />
            <StatusRow label="Positions" ready={data.positions > 0} />
            <StatusRow label="Leave management" ready />
          </div>
        </div>
      </section>

      <section className="dashboard-reveal" style={{ animationDelay: "440ms" }}>
        <div className="mb-3 flex items-end justify-between">
          <div>
            <div className="flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.14em] text-[#67b85a]">
              <ArrowRight className="h-4 w-4" />
              Quick actions
            </div>
            <h2 className="mt-2 text-[16px] font-semibold text-white">Move quickly</h2>
          </div>
        </div>

        <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
          <ActionCard icon={Users} title="Staff Records" description="Register and manage employees." onClick={() => nav("/staff")} />
          <ActionCard icon={Building2} title="Basic Data" description="Company, departments and positions." onClick={() => nav("/basic-data")} />
          <ActionCard icon={FileCog} title="Payments / Payroll" description="Open payroll and calculation tools." onClick={() => nav("/payroll")} />
          <ActionCard icon={CalendarDays} title="Leaves" description="Manage records and balances." onClick={() => nav("/leaves")} />
        </div>
      </section>
    </div>
  );
}

function MetricCard({
  icon: Icon,
  label,
  value,
  loading,
}: {
  icon: typeof Users;
  label: string;
  value: number;
  loading: boolean;
}) {
  return (
    <Card className="group relative overflow-hidden border-white/10 bg-white/[0.035] p-4 shadow-[0_16px_36px_rgba(0,0,0,0.18)] transition-all duration-300 hover:-translate-y-1 hover:border-[#4a8b3f]/45 hover:bg-white/[0.05]">
      <div className="pointer-events-none absolute -right-6 -top-6 h-20 w-20 rounded-full bg-[#4a8b3f]/10 blur-2xl transition-transform duration-500 group-hover:scale-125" />
      <div className="relative flex items-start justify-between gap-4">
        <div className="flex h-9 w-9 items-center justify-center rounded-2xl border border-[#4a8b3f]/25 bg-[#4a8b3f]/10 text-[#67b85a] shadow-[0_8px_24px_rgba(74,139,63,0.14)]">
          <Icon className="h-4.5 w-4.5" />
        </div>
        <div className="mt-1 h-2 w-2 rounded-full bg-[#4a8b3f] shadow-[0_0_14px_rgba(74,139,63,0.7)]" />
      </div>
      <div className="relative mt-3">
        <div className="text-[26px] font-semibold tracking-[-0.04em] text-white">
          {loading ? <span className="inline-block h-8 w-16 animate-pulse rounded-md bg-white/10" /> : value}
        </div>
        <div className="mt-1 text-[12px] font-medium text-white/45">{label}</div>
      </div>
    </Card>
  );
}

function StatTile({
  label,
  value,
  icon: Icon,
}: {
  label: string;
  value: number;
  icon: typeof Users;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-black/30 p-3 transition-colors hover:border-[#4a8b3f]/30">
      <div className="flex items-center justify-between gap-4">
        <span className="text-[11px] font-medium uppercase tracking-[0.08em] text-white/40">{label}</span>
        <Icon className="h-4 w-4 text-[#67b85a]" />
      </div>
      <div className="mt-1.5 text-[22px] font-semibold tracking-[-0.03em] text-white">{value}</div>
    </div>
  );
}

function StatusRow({ label, ready }: { label: string; ready: boolean }) {
  return (
    <div className="flex items-center justify-between gap-4 rounded-2xl border border-white/8 bg-white/[0.025] px-3.5 py-2.5">
      <span className="text-[12px] text-white/65">{label}</span>
      <span className="inline-flex items-center gap-2 text-[11px] font-semibold text-white/55">
        <span className={`h-2 w-2 rounded-full ${ready ? "bg-[#4a8b3f] shadow-[0_0_12px_rgba(74,139,63,0.6)]" : "bg-white/20"}`} />
        {ready ? "Ready" : "Not configured"}
      </span>
    </div>
  );
}

function ActionCard({
  icon: Icon,
  title,
  description,
  onClick,
}: {
  icon: typeof Users;
  title: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="group rounded-2xl border border-white/10 bg-white/[0.03] p-3.5 text-left transition-all duration-300 hover:-translate-y-1 hover:border-[#4a8b3f]/45 hover:bg-white/[0.05]"
    >
      <div className="flex items-start justify-between gap-4">
        <div className="flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-black/40 text-[#67b85a] transition-all duration-300 group-hover:border-[#4a8b3f]/30 group-hover:bg-[#4a8b3f]/10">
          <Icon className="h-4.5 w-4.5" />
        </div>
        <ArrowRight className="h-4 w-4 text-white/25 transition-all duration-300 group-hover:translate-x-1 group-hover:text-[#67b85a]" />
      </div>
      <div className="mt-3">
        <h3 className="text-[13px] font-semibold text-white">{title}</h3>
        <p className="mt-1 text-[12px] leading-5 text-white/45">{description}</p>
      </div>
    </button>
  );
}
