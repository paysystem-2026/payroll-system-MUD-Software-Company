import { InlineSkeleton } from "@/components/ui/PageSkeleton";
import { useState, useEffect, useMemo, useCallback, type ReactNode } from "react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { staffService } from "@/services/staff";
import type { Employee } from "@/types/staff";
import { EmployeeFormModal } from "@/components/staff/EmployeeFormModal";
import { EmployeeProfile } from "@/components/staff/EmployeeProfile";
import { Search, Plus, Eye, Pencil, Trash2, ChevronLeft, ChevronRight, Users, BriefcaseBusiness, FileBadge2, UserPlus } from "lucide-react";
import { useLocation, useNavigate, useSearchParams } from "react-router-dom";

const PAGE_SIZE = 10;
const STATUS_COLORS: Record<string, string> = {
  active: "bg-[#1e3a1a] text-[#4a8b3f]",
  inactive: "bg-[#242424] text-[#c9c9c9]",
  terminated: "bg-[#242424] text-[#c9c9c9]",
  suspended: "bg-[#242424] text-[#c9c9c9]",
};

export function StaffPage() {
  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [sortBy, setSortBy] = useState<"name" | "code" | "department">("name");
  const [page, setPage] = useState(0);
  const [showForm, setShowForm] = useState(false);
  const [editingEmployee, setEditingEmployee] = useState<Employee | null>(null);
  const [profileEmployee, setProfileEmployee] = useState<Employee | null>(null);
  const [searchParams] = useSearchParams();
  const location = useLocation();
  const navigate = useNavigate();

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const [data, departments, positions, contractTypes] = await Promise.all([staffService.getEmployees(), staffService.getDepartments(), staffService.getPositions(), staffService.getContractTypes()]);
      const departmentMap = new Map(departments.map((d) => [d.id, d.name]));
      const positionMap = new Map(positions.map((p) => [p.id, p.title]));
      const contractMap = new Map(contractTypes.map((c) => [c.id, c.name]));
      setEmployees(data.map((e) => ({ ...e, department_name: e.department_name ?? (e.department_id ? departmentMap.get(e.department_id) ?? null : null), position_title: e.position_title ?? (e.position_id ? positionMap.get(e.position_id) ?? null : null), contract_type_name: e.contract_type_name ?? (e.contract_type_id ? contractMap.get(e.contract_type_id) ?? null : null) })));
    } catch {
      setEmployees([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const employeeQuery = searchParams.get("employee") ?? "";
  const searchNonceQuery = searchParams.get("searchNonce") ?? "";

  useEffect(() => {
    const state = location.state as { employeeId?: number; searchNonce?: string | number } | null;
    const queryId = Number(employeeQuery);
    const stateId = Number(state?.employeeId ?? 0);
    const employeeId = queryId || stateId;
    const requestNonce = searchNonceQuery || String(state?.searchNonce ?? "");

    if (!employeeId || !employees.length) return;
    const match = employees.find((employee) => Number(employee.id) === employeeId);
    if (!match) return;

    // The nonce makes every global-search open a distinct navigation event,
    // including reopening the same employee after a previous search.
    if (requestNonce) setProfileEmployee(match);
  }, [employees, employeeQuery, searchNonceQuery, location.key, location.state]);

  const filtered = useMemo(() => {
    let result = [...employees];

    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(
        (e) =>
          e.first_name.toLowerCase().includes(q) ||
          e.last_name.toLowerCase().includes(q) ||
          e.employee_code.toLowerCase().includes(q) ||
          (e.phone ?? "").toLowerCase().includes(q) ||
          (e.email ?? "").toLowerCase().includes(q)
      );
    }

    if (statusFilter !== "all") {
      result = result.filter((e) => e.employment_status === statusFilter);
    }

    result.sort((a, b) => {
      if (sortBy === "name") {
        return `${a.first_name} ${a.last_name}`.localeCompare(`${b.first_name} ${b.last_name}`);
      }
      if (sortBy === "code") {
        return a.employee_code.localeCompare(b.employee_code);
      }
      return (a.department_name ?? "").localeCompare(b.department_name ?? "");
    });

    return result;
  }, [employees, search, statusFilter, sortBy]);

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const currentPage = Math.min(page, totalPages - 1);
  const paged = filtered.slice(currentPage * PAGE_SIZE, (currentPage + 1) * PAGE_SIZE);

  const handleAdd = () => {
    setEditingEmployee(null);
    setShowForm(true);
  };

  const handleEdit = (emp: Employee) => {
    setEditingEmployee(emp);
    setShowForm(true);
  };

  const handleDelete = async (emp: Employee) => {
    if (confirm(`Delete employee "${emp.first_name} ${emp.last_name}"? This cannot be undone.`)) {
      await staffService.deleteEmployee(emp.id);
      refresh();
    }
  };

  const handleFormSaved = async () => {
    setShowForm(false);
    setEditingEmployee(null);
    await refresh();
  };

  const handleProfileClose = async () => {
    setProfileEmployee(null);
    // Remove the deep-link state/query so closing the profile does not reopen it.
    navigate("/staff", { replace: true, state: null });
    await refresh();
  };

  if (profileEmployee) {
    return <EmployeeProfile employee={profileEmployee} onClose={handleProfileClose} />;
  }

  return (
    <div className="animate-[fadeIn_.35s_ease-out]">
      <PageHeader
        title="Staff Records"
        description="Employee records, profiles, salary information, and employment history"
      />

      <div className="mb-4 grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
        <MiniStat icon={<Users className="h-4 w-4" />} label="Employees" value={employees.length} />
        <MiniStat icon={<UserPlus className="h-4 w-4" />} label="Active" value={employees.filter((e) => e.employment_status === "active").length} />
        <MiniStat icon={<BriefcaseBusiness className="h-4 w-4" />} label="Departments" value={new Set(employees.map((e) => e.department_id).filter(Boolean)).size} />
        <MiniStat icon={<FileBadge2 className="h-4 w-4" />} label="Contracts" value={new Set(employees.map((e) => e.contract_type_id).filter(Boolean)).size} />
      </div>

      <div className="mb-3.5 flex flex-wrap items-center gap-2.5">
        <div className="flex h-9 w-64 items-center gap-2 rounded-lg border border-[#2e2e2e] bg-[#171717] px-3 transition-all duration-200 focus-within:border-[#4a8b3f]/60 focus-within:shadow-[0_0_0_3px_rgba(74,139,63,.08)]">
          <Search className="h-4 w-4 text-[#888888]" />
          <input
            value={search}
            onChange={(e) => { setSearch(e.target.value); setPage(0); }}
            placeholder="Search employees..."
            className="w-full bg-transparent text-[13px] text-[#e8e8e8] placeholder:text-[#888888] focus:outline-none"
          />
        </div>

        <select
          value={statusFilter}
          onChange={(e) => { setStatusFilter(e.target.value); setPage(0); }}
          className="h-9 rounded-lg border border-[#2e2e2e] bg-[#171717] px-3 text-[12px] text-[#e8e8e8] outline-none transition-all duration-200 focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/10"
        >
          <option value="all">All Statuses</option>
          <option value="active">Active</option>
          <option value="inactive">Inactive</option>
          <option value="suspended">Suspended</option>
          <option value="terminated">Terminated</option>
        </select>

        <select
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as "name" | "code" | "department")}
          className="h-9 rounded-lg border border-[#2e2e2e] bg-[#171717] px-3 text-[12px] text-[#e8e8e8] outline-none transition-all duration-200 focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/10"
        >
          <option value="name">Sort by Name</option>
          <option value="code">Sort by Code</option>
          <option value="department">Sort by Department</option>
        </select>

        <div className="flex-1" />

        <Button onClick={handleAdd}>
          <Plus className="h-4 w-4" /> Add Employee
        </Button>
      </div>

      {loading ? (
        <InlineSkeleton rows={7} />
      ) : paged.length === 0 ? (
        <Card>
          <div className="py-12 text-center">
            <p className="text-[13px] text-[#888888]">No employees found.</p>
            <Button onClick={handleAdd} className="mt-4">
              <Plus className="h-4 w-4" /> Add First Employee
            </Button>
          </div>
        </Card>
      ) : (
        <>
          <Card className="staff-table-card overflow-hidden border-[#292929] bg-[#131313] p-0 shadow-[0_12px_36px_rgba(0,0,0,.22)]">
            <table className="w-full">
              <thead className="bg-[#161616]">
                <tr className="border-b border-[#2e2e2e] text-left">
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Code</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Name</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Department</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Position</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Contract</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Status</th>
                  <th className="px-3.5 py-2.5 text-[10px] font-semibold uppercase tracking-[0.1em] text-[#7f7f7f]">Actions</th>
                </tr>
              </thead>
              <tbody>
                {paged.map((emp) => (
                  <tr key={emp.id} className="staff-row-in border-b border-[#202020] transition-all duration-200 hover:bg-[#191919]">
                    <td className="px-3.5 py-2.5 font-mono text-[12px] font-medium text-[#dedede]">{emp.employee_code}</td>
                    <td className="px-3.5 py-2.5">
                      <div className="flex min-w-0 items-center gap-2.5">
                        <div className="h-8 w-8 shrink-0 overflow-hidden rounded-full border border-[#303030] bg-[#202020] shadow-inner">
                          {emp.photo_path ? (
                            <img src={emp.photo_path} alt={`${emp.first_name} ${emp.last_name}`} className="h-full w-full object-cover transition-transform duration-300 hover:scale-105" />
                          ) : (
                            <div className="grid h-full w-full place-items-center text-[11px] font-semibold text-[#4a8b3f]">
                              {(emp.first_name?.[0] ?? "").toUpperCase()}{(emp.last_name?.[0] ?? "").toUpperCase()}
                            </div>
                          )}
                        </div>
                        <div className="min-w-0">
                          <p className="truncate text-[12.5px] font-medium text-[#ededed]">{emp.first_name} {emp.last_name}</p>
                          <p className="truncate text-[10px] text-[#6f6f6f]">{emp.email ?? emp.phone ?? "Employee"}</p>
                        </div>
                      </div>
                    </td>
                    <td className="px-3.5 py-2.5 text-[12px] text-[#8f8f8f]">{emp.department_name ?? "—"}</td>
                    <td className="px-3.5 py-2.5 text-[12px] text-[#8f8f8f]">{emp.position_title ?? "—"}</td>
                    <td className="px-3.5 py-2.5 text-[12px]">
                      <span className={emp.contract_type_name ? "inline-flex items-center rounded-md border border-[#2e4a2b] bg-[#172016] px-2 py-0.5 text-[11px] text-[#b8d7b2]" : "text-[11px] text-[#6f6f6f]"}>
                        {emp.contract_type_name ?? "Not assigned"}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <span className={`inline-block rounded-full px-2 py-0.5 text-[10px] font-medium ${STATUS_COLORS[emp.employment_status] ?? STATUS_COLORS.inactive}`}>
                        {emp.employment_status}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-1">
                        <button onClick={() => setProfileEmployee(emp)} title="View profile" className="rounded-md p-1.5 text-[#777] transition-all duration-200 hover:bg-[#242424] hover:text-[#e8e8e8] hover:-translate-y-px">
                          <Eye className="h-4 w-4" />
                        </button>
                        <button onClick={() => handleEdit(emp)} title="Edit" className="rounded-md p-1.5 text-[#777] transition-all duration-200 hover:bg-[#242424] hover:text-[#e8e8e8] hover:-translate-y-px">
                          <Pencil className="h-4 w-4" />
                        </button>
                        <button onClick={() => handleDelete(emp)} title="Delete" className="rounded-md p-1.5 text-[#777] transition-all duration-200 hover:bg-[#242424] hover:text-white hover:-translate-y-px">
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </Card>

          <div className="mt-3 flex items-center justify-between">
            <span className="text-[12px] text-[#888888]">
              {filtered.length} employee{filtered.length !== 1 ? "s" : ""} · Page {currentPage + 1} of {totalPages}
            </span>
            <div className="flex items-center gap-2">
              <Button variant="secondary" disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>
                <ChevronLeft className="h-4 w-4" /> Prev
              </Button>
              <Button variant="secondary" disabled={currentPage >= totalPages - 1} onClick={() => setPage(currentPage + 1)}>
                Next <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </>
      )}

      {showForm && (
        <EmployeeFormModal
          employee={editingEmployee}
          onClose={() => { setShowForm(false); setEditingEmployee(null); }}
          onSaved={handleFormSaved}
        />
      )}
    </div>
  );
}

function MiniStat({ icon, label, value }: { icon: ReactNode; label: string; value: number }) {
  return (
    <Card className="staff-stat group border-[#292929] bg-[#151515] p-3 transition-all duration-300 hover:-translate-y-0.5 hover:border-[#4a8b3f]/50 hover:shadow-[0_10px_26px_rgba(74,139,63,.10)]">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.12em] text-[#8f8f8f]">{label}</p>
          <p className="mt-0.5 text-[20px] font-semibold tracking-tight text-white">{value}</p>
        </div>
        <div className="grid h-9 w-9 place-items-center rounded-xl border border-[#315e2a] bg-[#172316] text-[#4a8b3f] transition-transform duration-300 group-hover:scale-105">{icon}</div>
      </div>
    </Card>
  );
}
