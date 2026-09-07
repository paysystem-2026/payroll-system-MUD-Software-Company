import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Building2, FileCog, Layers3, MapPin, Pencil, Plus, Trash2, BriefcaseBusiness, Globe2, Hash, Landmark, Phone, Mail, Save } from "lucide-react";
import { Card } from "@/components/ui/Card";
import { PageHeader } from "@/components/ui/PageHeader";
import { Button } from "@/components/ui/Button";
import { staffService } from "@/services/staff";
import type { Company, ContractType, Department, Position } from "@/types/staff";
import { payrollService } from "@/services/payroll";

type Tab = "company" | "departments" | "positions" | "contracts" | "payroll";

type DepartmentForm = { id?: number; name: string; code: string };
type PositionForm = { id?: number; department_id: number; title: string; code: string };
type ContractForm = { id?: number; name: string; code: string; description: string };

const input = "w-full rounded-xl border border-[#303030] bg-[#111111] px-3.5 py-2.5 text-[13px] text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/20 placeholder:text-[#666]";
const select = `${input} appearance-none`;

export function BasicDataPage() {
  const navigate = useNavigate();
  const [tab, setTab] = useState<Tab>("company");
  const [company, setCompany] = useState<Company | null>(null);
  const [departments, setDepartments] = useState<Department[]>([]);
  const [positions, setPositions] = useState<Position[]>([]);
  const [contractTypes, setContractTypes] = useState<ContractType[]>([]);
  const [rules, setRules] = useState(0);
  const [loading, setLoading] = useState(true);
  const [savingCompany, setSavingCompany] = useState(false);
  const [companySaved, setCompanySaved] = useState(false);
  const [deptModal, setDeptModal] = useState(false);
  const [positionModal, setPositionModal] = useState(false);
  const [deptForm, setDeptForm] = useState<DepartmentForm>({ name: "", code: "" });
  const [positionForm, setPositionForm] = useState<PositionForm>({ department_id: 0, title: "", code: "" });
  const [contractModal, setContractModal] = useState(false);
  const [contractForm, setContractForm] = useState<ContractForm>({ name: "", code: "", description: "" });

  const load = async () => {
    setLoading(true);
    try {
      const [c, d, p, r, contracts] = await Promise.all([
        staffService.getCompany(),
        staffService.getDepartments(),
        staffService.getPositions(),
        payrollService.getRules(),
        staffService.getContractTypes(),
      ]);
      setCompany(c);
      setDepartments(d);
      setPositions(p);
      setRules(r.rules.length);
      setContractTypes(contracts);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { load(); }, []);

  const departmentName = useMemo(() => new Map(departments.map(d => [d.id, d.name])), [departments]);
  const positionsByDepartment = useMemo(() => {
    const counts = new Map<number, number>();
    for (const position of positions) counts.set(position.department_id, (counts.get(position.department_id) ?? 0) + 1);
    return counts;
  }, [positions]);

  const updateCompany = (patch: Partial<Company>) => {
    setCompany(prev => ({
      id: prev?.id ?? 0,
      name: prev?.name ?? "",
      legal_name: prev?.legal_name ?? "",
      website: prev?.website ?? "",
      tin_number: prev?.tin_number ?? "",
      rssb_number: prev?.rssb_number ?? "",
      address: prev?.address ?? "",
      phone: prev?.phone ?? "",
      email: prev?.email ?? "",
      tax_id: prev?.tax_id ?? null,
      currency: prev?.currency ?? "RWF",
      logo_path: prev?.logo_path ?? null,
      ...patch,
    }));
    setCompanySaved(false);
  };

  const saveCompany = async () => {
    if (!company?.name.trim()) return;
    setSavingCompany(true);
    try {
      await staffService.saveCompany({
        name: company.name.trim(),
        legal_name: company.legal_name || null,
        website: company.website || null,
        tin_number: company.tin_number || null,
        rssb_number: company.rssb_number || null,
        address: company.address || null,
        phone: company.phone || null,
        email: company.email || null,
        tax_id: company.tax_id || null,
        currency: company.currency || "RWF",
        logo_path: company.logo_path || null,
      });
      await load();
      setCompanySaved(true);
    } finally {
      setSavingCompany(false);
    }
  };

  const openAddDepartment = () => {
    setDeptForm({ name: "", code: "" });
    setDeptModal(true);
  };

  const editDepartment = (department: Department) => {
    setDeptForm({ id: department.id, name: department.name, code: department.code ?? "" });
    setDeptModal(true);
  };

  const saveDepartment = async () => {
    if (!deptForm.name.trim()) return;
    await staffService.saveDepartment({ id: deptForm.id, name: deptForm.name.trim(), code: deptForm.code.trim() || null });
    setDeptModal(false);
    await load();
  };

  const openAddPosition = () => {
    setPositionForm({ department_id: departments[0]?.id ?? 0, title: "", code: "" });
    setPositionModal(true);
  };

  const editPosition = (position: Position) => {
    setPositionForm({ id: position.id, department_id: position.department_id, title: position.title, code: position.code ?? "" });
    setPositionModal(true);
  };

  const openAddContract = () => { setContractForm({ name: "", code: "", description: "" }); setContractModal(true); };
  const editContract = (contract: ContractType) => { setContractForm({ id: contract.id, name: contract.name, code: contract.code ?? "", description: contract.description ?? "" }); setContractModal(true); };
  const saveContract = async () => { if (!contractForm.name.trim()) return; await staffService.saveContractType({ id: contractForm.id, name: contractForm.name.trim(), code: contractForm.code.trim() || null, description: contractForm.description.trim() || null }); setContractModal(false); await load(); };

  const savePosition = async () => {
    if (!positionForm.title.trim() || !positionForm.department_id) return;
    await staffService.savePosition({
      id: positionForm.id,
      department_id: positionForm.department_id,
      title: positionForm.title.trim(),
      code: positionForm.code.trim() || null,
    });
    setPositionModal(false);
    await load();
  };

  return (
    <div className="basic-data-page space-y-6">
      <PageHeader title="Basic Data" description="Manage the company profile and core payroll setup." />

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <TabButton active={tab === "company"} onClick={() => setTab("company")} icon={Building2} label="Company" />
        <TabButton active={tab === "departments"} onClick={() => setTab("departments")} icon={Layers3} label="Departments" />
        <TabButton active={tab === "positions"} onClick={() => setTab("positions")} icon={BriefcaseBusiness} label="Positions" />
        <TabButton active={tab === "contracts"} onClick={() => setTab("contracts")} icon={FileCog} label="Contract Types" />
        <TabButton active={tab === "payroll"} onClick={() => setTab("payroll")} icon={FileCog} label="Payroll Configuration" />
      </div>

      {loading ? (
        <Card className="p-10 text-center text-[13px] text-[#888]">Loading basic data…</Card>
      ) : tab === "company" ? (
        <Card className="basic-data-panel p-6">
          <SectionTitle icon={Building2} title="Company profile" subtitle="Keep the organization details used across payroll and reports." />
          <div className="mt-6 grid gap-4 md:grid-cols-2">
            <Field label="Company name" icon={Building2}><input className={input} value={company?.name ?? ""} onChange={e => updateCompany({ name: e.target.value })} placeholder="Company name" /></Field>
            <Field label="Legal name"><input className={input} value={company?.legal_name ?? ""} onChange={e => updateCompany({ legal_name: e.target.value })} placeholder="Registered legal name" /></Field>
            <Field label="Website" icon={Globe2}><input className={input} value={company?.website ?? ""} onChange={e => updateCompany({ website: e.target.value })} placeholder="https://example.com" /></Field>
            <Field label="TIN number" icon={Hash}><input className={input} value={company?.tin_number ?? ""} onChange={e => updateCompany({ tin_number: e.target.value })} placeholder="Tax identification number" /></Field>
            <Field label="RSSB number" icon={Landmark}><input className={input} value={company?.rssb_number ?? ""} onChange={e => updateCompany({ rssb_number: e.target.value })} placeholder="Company RSSB number" /></Field>
            <Field label="Address" icon={MapPin}><input className={input} value={company?.address ?? ""} onChange={e => updateCompany({ address: e.target.value })} placeholder="Street / city / country" /></Field>
            <Field label="Phone" icon={Phone}><input className={input} value={company?.phone ?? ""} onChange={e => updateCompany({ phone: e.target.value })} placeholder="Phone number" /></Field>
            <Field label="Email" icon={Mail}><input className={input} value={company?.email ?? ""} onChange={e => updateCompany({ email: e.target.value })} placeholder="Email address" /></Field>
          </div>
          <div className="mt-6 flex items-center justify-between gap-4 border-t border-[#262626] pt-5">
            <p className="text-[12px] text-[#747474]">These details are stored locally and reused by the application.</p>
            <Button onClick={saveCompany} disabled={savingCompany || !company?.name.trim()}>
              <Save className="h-4 w-4" /> {savingCompany ? "Saving…" : companySaved ? "Saved" : "Save Company"}
            </Button>
          </div>
        </Card>
      ) : tab === "departments" ? (
        <Card className="basic-data-panel p-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <SectionTitle icon={Layers3} title="Departments" subtitle={`${departments.length} configured`} />
            <Button onClick={openAddDepartment}><Plus className="h-4 w-4" /> Add Department</Button>
          </div>
          <div className="mt-6 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {departments.length === 0 ? <EmptyPanel label="No departments yet" action="Add Department" onClick={openAddDepartment} /> : departments.map(d => (
              <div key={d.id} className="basic-data-item group rounded-2xl border border-[#292929] bg-[#111111] p-4 transition-all duration-300 hover:-translate-y-1 hover:border-[#4a8b3f]/60">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2"><span className="grid h-9 w-9 place-items-center rounded-xl bg-[#4a8b3f]/15 text-[#68a85f]"><Layers3 className="h-4 w-4" /></span><div><h3 className="truncate text-[14px] font-semibold text-white">{d.name}</h3><p className="text-[11px] text-[#777]">{d.code || "No code"}</p></div></div>
                  </div>
                  <div className="flex gap-1 opacity-70 transition-all duration-200 group-hover:opacity-100"><IconButton label="Edit" onClick={() => editDepartment(d)}><Pencil className="h-3.5 w-3.5" /></IconButton><IconButton label="Delete" onClick={async () => { await staffService.deleteDepartment(d.id); await load(); }}><Trash2 className="h-3.5 w-3.5" /></IconButton></div>
                </div>
                <div className="mt-4 flex items-center justify-between border-t border-[#242424] pt-3 text-[11px] text-[#777]">
                  <span>{positionsByDepartment.get(d.id) ?? 0} positions</span><span className="text-[#68a85f]">Active</span>
                </div>
              </div>
            ))}
          </div>
        </Card>
      ) : tab === "contracts" ? (
        <Card className="basic-data-panel p-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <SectionTitle icon={FileCog} title="Contract Types" subtitle={`${contractTypes.length} configured`} />
            <Button onClick={openAddContract}><Plus className="h-4 w-4" /> Add Contract Type</Button>
          </div>
          <div className="mt-6 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {contractTypes.length === 0 ? <EmptyPanel label="No contract types yet" action="Add Contract Type" onClick={openAddContract} /> : contractTypes.map(c => (
              <div key={c.id} className="basic-data-item group rounded-2xl border border-[#292929] bg-[#111111] p-4 transition-all duration-300 hover:-translate-y-1 hover:border-[#4a8b3f]/60">
                <div className="flex items-start justify-between gap-3"><div><h3 className="text-[14px] font-semibold text-white">{c.name}</h3><p className="mt-1 text-[11px] text-[#777]">{c.code || "No code"}</p>{c.description && <p className="mt-3 text-[12px] leading-5 text-[#888]">{c.description}</p>}</div><div className="flex gap-1 opacity-75 transition-opacity duration-200 hover:opacity-100"><IconButton label="Edit" onClick={() => editContract(c)}><Pencil className="h-3.5 w-3.5" /></IconButton><IconButton label="Delete" onClick={async () => { await staffService.deleteContractType(c.id); await load(); }}><Trash2 className="h-3.5 w-3.5" /></IconButton></div></div>
              </div>
            ))}
          </div>
        </Card>
      ) : tab === "positions" ? (
        <Card className="basic-data-panel p-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <SectionTitle icon={BriefcaseBusiness} title="Positions" subtitle={`${positions.length} configured`} />
            <Button onClick={openAddPosition} disabled={!departments.length}><Plus className="h-4 w-4" /> Add Position</Button>
          </div>
          <div className="mt-6 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {positions.length === 0 ? <EmptyPanel label={departments.length ? "No positions yet" : "Create a department first"} action={departments.length ? "Add Position" : "Go to Departments"} onClick={departments.length ? openAddPosition : () => setTab("departments")} /> : positions.map(p => (
              <div key={p.id} className="basic-data-item group rounded-2xl border border-[#292929] bg-[#111111] p-4 transition-all duration-300 hover:-translate-y-1 hover:border-[#4a8b3f]/60">
                <div className="flex items-start justify-between gap-3"><div className="flex min-w-0 items-center gap-3"><span className="grid h-9 w-9 place-items-center rounded-xl bg-[#4a8b3f]/15 text-[#68a85f]"><BriefcaseBusiness className="h-4 w-4" /></span><div className="min-w-0"><h3 className="truncate text-[14px] font-semibold text-white">{p.title}</h3><p className="truncate text-[11px] text-[#777]">{departmentName.get(p.department_id) ?? "Unassigned"} · {p.code || "No code"}</p></div></div><div className="flex gap-1 opacity-70 transition-all duration-200 group-hover:opacity-100"><IconButton label="Edit" onClick={() => editPosition(p)}><Pencil className="h-3.5 w-3.5" /></IconButton><IconButton label="Delete" onClick={async () => { await staffService.deletePosition(p.id); await load(); }}><Trash2 className="h-3.5 w-3.5" /></IconButton></div></div>
              </div>
            ))}
          </div>
        </Card>
      ) : (
        <Card className="basic-data-panel p-6">
          <SectionTitle icon={FileCog} title="Payroll Configuration" subtitle="Dynamic rules, formulas, calculation order, and version history." />
          <div className="mt-6 grid gap-4 md:grid-cols-3">
            <StatCard label="Configured rules" value={rules} />
            <StatCard label="Calculation mode" value="Dynamic" accent />
            <StatCard label="History" value="Versioned" accent />
          </div>
          <div className="mt-6 flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-[#292929] bg-[#111111] p-5">
            <div><h3 className="text-[14px] font-semibold text-white">Manage payroll rules</h3><p className="mt-1 text-[12px] text-[#777]">Edit rates, formulas, order, and rule versions from the existing configuration screen.</p></div>
            <Button onClick={() => navigate("/payroll")}><FileCog className="h-4 w-4" /> Open Configuration</Button>
          </div>
        </Card>
      )}

      {deptModal && <Modal title={deptForm.id ? "Edit Department" : "Add Department"} onClose={() => setDeptModal(false)} onSave={saveDepartment}><div className="space-y-4"><Field label="Department name"><input autoFocus className={input} value={deptForm.name} onChange={e => setDeptForm(v => ({ ...v, name: e.target.value }))} placeholder="e.g. Finance" /></Field><Field label="Code"><input className={input} value={deptForm.code} onChange={e => setDeptForm(v => ({ ...v, code: e.target.value }))} placeholder="e.g. FIN" /></Field></div></Modal>}
      {contractModal && <Modal title={contractForm.id ? "Edit Contract Type" : "Add Contract Type"} onClose={() => setContractModal(false)} onSave={saveContract}><div className="space-y-4"><Field label="Contract type name"><input autoFocus className={input} value={contractForm.name} onChange={e => setContractForm(v => ({ ...v, name: e.target.value }))} placeholder="e.g. Permanent" /></Field><Field label="Code"><input className={input} value={contractForm.code} onChange={e => setContractForm(v => ({ ...v, code: e.target.value }))} placeholder="e.g. PERM" /></Field><Field label="Description"><textarea className={`${input} min-h-24`} value={contractForm.description} onChange={e => setContractForm(v => ({ ...v, description: e.target.value }))} /></Field></div></Modal>}
      {positionModal && <Modal title={positionForm.id ? "Edit Position" : "Add Position"} onClose={() => setPositionModal(false)} onSave={savePosition}><div className="space-y-4"><Field label="Department"><select autoFocus className={select} value={positionForm.department_id || ""} onChange={e => setPositionForm(v => ({ ...v, department_id: Number(e.target.value) }))}><option value="">Select department</option>{departments.map(d => <option value={d.id} key={d.id}>{d.name}</option>)}</select></Field><Field label="Position title"><input className={input} value={positionForm.title} onChange={e => setPositionForm(v => ({ ...v, title: e.target.value }))} placeholder="e.g. Accountant" /></Field><Field label="Code"><input className={input} value={positionForm.code} onChange={e => setPositionForm(v => ({ ...v, code: e.target.value }))} placeholder="e.g. ACC" /></Field></div></Modal>}
    </div>
  );
}

function SectionTitle({ icon: Icon, title, subtitle }: { icon: typeof Building2; title: string; subtitle: string }) {
  return <div className="basic-data-section-title flex items-center gap-3"><span className="grid h-10 w-10 place-items-center rounded-xl border border-[#303030] bg-[#4a8b3f]/10 text-[#68a85f]"><Icon className="h-4.5 w-4.5" /></span><div><h2 className="text-[16px] font-semibold text-white">{title}</h2><p className="text-[12px] text-[#777]">{subtitle}</p></div></div>;
}

function Field({ label, icon: Icon, children }: { label: string; icon?: typeof Building2; children: ReactNode }) {
  return <label className="block"><span className="mb-1.5 flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-[0.08em] text-[#8a8a8a]">{Icon && <Icon className="h-3.5 w-3.5 text-[#68a85f]" />}{label}</span>{children}</label>;
}

function TabButton({ active, onClick, icon: Icon, label }: { active: boolean; onClick: () => void; icon: typeof Building2; label: string }) {
  return <button type="button" onClick={onClick} className={`basic-data-tab flex items-center justify-between rounded-2xl border px-4 py-3.5 text-left transition-all duration-300 ${active ? "border-[#4a8b3f]/70 bg-[#4a8b3f]/12 text-white shadow-[0_10px_30px_rgba(74,139,63,0.12)]" : "border-[#292929] bg-[#141414] text-[#8b8b8b] hover:border-[#3a3a3a] hover:bg-[#191919] hover:text-white"}`}><span className="flex items-center gap-2.5"><Icon className={`h-4 w-4 ${active ? "text-[#74b56a]" : "text-[#666]"}`} />{label}</span><span className={`h-2 w-2 rounded-full ${active ? "bg-[#4a8b3f] shadow-[0_0_12px_rgba(74,139,63,0.65)]" : "bg-[#333]"}`} /></button>;
}

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: ReactNode }) {
  return <button type="button" aria-label={label} title={label} onClick={onClick} className="grid h-8 w-8 place-items-center rounded-lg border border-[#2a2a2a] bg-[#151515] text-[#8b8b8b] transition hover:border-[#4a8b3f]/60 hover:text-white">{children}</button>;
}

function EmptyPanel({ label, action, onClick }: { label: string; action: string; onClick: () => void }) {
  return <div className="md:col-span-2 xl:col-span-3 rounded-2xl border border-dashed border-[#303030] bg-[#111111] p-10 text-center"><p className="text-[13px] font-medium text-white">{label}</p><button type="button" onClick={onClick} className="mt-3 text-[12px] font-semibold text-[#70ae68] transition hover:text-white">{action} →</button></div>;
}

function StatCard({ label, value, accent = false }: { label: string; value: string | number; accent?: boolean }) {
  return <div className="basic-data-stat rounded-2xl border border-[#292929] bg-[#111111] p-4 transition-all duration-300"><div className="text-[11px] uppercase tracking-[0.08em] text-[#777]">{label}</div><div className={`mt-2 text-[22px] font-semibold ${accent ? "text-[#76b46f]" : "text-white"}`}>{value}</div></div>;
}

function Modal({ title, onClose, onSave, children }: { title: string; onClose: () => void; onSave: () => Promise<void>; children: ReactNode }) {
  const [saving, setSaving] = useState(false);
  const save = async () => { setSaving(true); try { await onSave(); } finally { setSaving(false); } };
  return <div className="fixed inset-0 z-50 grid place-items-center bg-black/70 p-4 backdrop-blur-sm"><div className="w-full max-w-md rounded-3xl border border-[#2e2e2e] bg-[#101010] p-6 shadow-[0_25px_80px_rgba(0,0,0,0.45)]"><div className="mb-5 flex items-center justify-between"><h3 className="text-[17px] font-semibold text-white">{title}</h3><button type="button" onClick={onClose} className="text-[#777] transition hover:text-white">×</button></div>{children}<div className="mt-6 flex justify-end gap-2"><Button variant="ghost" onClick={onClose}>Cancel</Button><Button onClick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</Button></div></div></div>;
}
