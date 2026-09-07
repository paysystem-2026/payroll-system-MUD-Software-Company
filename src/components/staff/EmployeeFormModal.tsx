import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import type { ContractType, Department, Employee, Position, SaveEmployeeRequest } from "@/types/staff";
import { staffService } from "@/services/staff";
import { Button } from "@/components/ui/Button";
import { Camera, CalendarDays, RefreshCw, UserRound, X } from "lucide-react";

interface EmployeeFormModalProps { employee: Employee | null; onClose: () => void; onSaved: () => void; }
const STATUSES = ["active", "inactive", "suspended", "terminated"];
const GENDERS = ["", "male", "female", "other"];
const inputCls = "w-full rounded-xl border border-[#303030] bg-[#0f0f0f] px-3.5 py-2.5 text-[13px] text-white outline-none transition focus:border-[#4a8b3f] focus:ring-2 focus:ring-[#4a8b3f]/20";

export function EmployeeFormModal({ employee, onClose, onSaved }: EmployeeFormModalProps) {
  const [departments, setDepartments] = useState<Department[]>([]);
  const [positions, setPositions] = useState<Position[]>([]);
  const [contractTypes, setContractTypes] = useState<ContractType[]>([]);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [employeeCode, setEmployeeCode] = useState(employee?.employee_code ?? "");
  const [firstName, setFirstName] = useState(employee?.first_name ?? "");
  const [lastName, setLastName] = useState(employee?.last_name ?? "");
  const [gender, setGender] = useState(employee?.gender ?? "");
  const [dateOfBirth, setDateOfBirth] = useState(employee?.date_of_birth ?? "");
  const [nationalId, setNationalId] = useState(employee?.national_id ?? "");
  const [phone, setPhone] = useState(employee?.phone ?? "");
  const [email, setEmail] = useState(employee?.email ?? "");
  const [address, setAddress] = useState(employee?.address ?? "");
  const [departmentId, setDepartmentId] = useState(employee?.department_id?.toString() ?? "");
  const [positionId, setPositionId] = useState(employee?.position_id?.toString() ?? "");
  const [contractTypeId, setContractTypeId] = useState(employee?.contract_type_id?.toString() ?? "");
  const [grade, setGrade] = useState(employee?.grade ?? "");
  const [hireDate, setHireDate] = useState(employee?.hire_date ?? "");
  const [employmentStatus, setEmploymentStatus] = useState(employee?.employment_status ?? "active");
  const [dependants, setDependants] = useState(employee?.dependants?.toString() ?? "0");
  const [rssbNumber, setRssbNumber] = useState(employee?.rssb_number ?? "");
  const [bankName, setBankName] = useState(employee?.bank_name ?? "");
  const [bankAccount, setBankAccount] = useState(employee?.bank_account ?? "");
  const [photoPath, setPhotoPath] = useState(employee?.photo_path ?? "");
  const [baseSalary, setBaseSalary] = useState("");
  const [salaryEffectiveDate, setSalaryEffectiveDate] = useState("");
  const [salaryReason, setSalaryReason] = useState("");

  useEffect(() => {
    Promise.all([staffService.getDepartments(), staffService.getContractTypes()]).then(([d, c]) => { setDepartments(d); setContractTypes(c); }).catch(() => {});
  }, []);
  useEffect(() => {
    const deptId = departmentId ? Number(departmentId) : undefined;
    staffService.getPositions(deptId).then(setPositions).catch(() => setPositions([]));
  }, [departmentId]);
  useEffect(() => {
    if (!employee) staffService.generateEmployeeCode().then(setEmployeeCode).catch(() => setEmployeeCode(`EMP-${Date.now().toString().slice(-6)}`));
  }, [employee]);
  
  const onPhotoChange = (file?: File) => {
    if (!file) return;
    if (file.size > 2 * 1024 * 1024) { setError("Employee photo must be 2 MB or smaller."); return; }
    const reader = new FileReader();
    reader.onload = () => setPhotoPath(String(reader.result || ""));
    reader.readAsDataURL(file);
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault(); setError("");
    if (!firstName.trim()) return setError("First name is required.");
    if (!lastName.trim()) return setError("Last name is required.");
    if (!employee && (!baseSalary || Number(baseSalary) <= 0)) return setError("Initial basic salary is required for new employees.");
    const request: SaveEmployeeRequest = {
      id: employee?.id, employee_code: employeeCode.trim(), first_name: firstName.trim(), last_name: lastName.trim(),
      gender: gender || null, date_of_birth: dateOfBirth || null, national_id: nationalId || null, phone: phone || null, email: email || null,
      address: address || null, department_id: departmentId ? Number(departmentId) : null, position_id: positionId ? Number(positionId) : null,
      contract_type_id: contractTypeId ? Number(contractTypeId) : null, grade: grade || null, hire_date: hireDate || null,
      employment_status: employmentStatus, dependants: Number(dependants) || 0, rssb_number: rssbNumber || null,
      bank_name: bankName || null, bank_account: bankAccount || null, photo_path: photoPath || null,
      base_salary: baseSalary ? Number(baseSalary) : null, salary_effective_date: salaryEffectiveDate || null, salary_reason: salaryReason || null,
    };
    setSaving(true);
    try { const res = await staffService.saveEmployee(request); if (!res.success) setError(res.message); else onSaved(); }
    catch (err) { setError(err instanceof Error ? err.message : String(err)); }
    finally { setSaving(false); }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm" onClick={onClose}>
      <div className="max-h-[94vh] w-full max-w-3xl overflow-y-auto rounded-[28px] border border-[#2d2d2d] bg-[#171717] shadow-2xl" onClick={(e) => e.stopPropagation()}>
        <div className="sticky top-0 z-10 flex items-center justify-between border-b border-[#282828] bg-[#171717]/95 px-6 py-5 backdrop-blur">
          <div><div className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#68a85f]"><UserRound className="h-3.5 w-3.5" /> Staff Records</div><h2 className="mt-1 text-[20px] font-semibold text-white">{employee ? "Edit Employee" : "Register Employee"}</h2></div>
          <button onClick={onClose} className="rounded-xl p-2 text-[#777] transition hover:bg-[#242424] hover:text-white"><X className="h-5 w-5" /></button>
        </div>
        <form onSubmit={handleSubmit} className="space-y-6 p-6">
          <Section title="Employee Identity">
            <div className="grid gap-5 md:grid-cols-[120px_1fr]">
              <div className="flex flex-col items-center gap-2"><div className="grid h-24 w-24 place-items-center overflow-hidden rounded-2xl border border-[#303030] bg-[#0f0f0f]">{photoPath ? <img src={photoPath} alt="Employee" className="h-full w-full object-cover" /> : <Camera className="h-7 w-7 text-[#4a8b3f]" />}</div><label className="cursor-pointer text-[11px] font-semibold text-[#6aa660] hover:text-white"><input type="file" accept="image/*" className="hidden" onChange={(e) => onPhotoChange(e.target.files?.[0])} />Add photo</label></div>
              <div className="grid grid-cols-2 gap-4">
                <Field label="Employee Code"><div className="flex gap-2"><input value={employeeCode} readOnly className={`${inputCls} cursor-not-allowed text-[#bdbdbd]`} /><button type="button" title="Generate code" onClick={() => staffService.generateEmployeeCode().then(setEmployeeCode)} className="grid w-11 shrink-0 place-items-center rounded-xl border border-[#303030] bg-[#111] text-[#68a85f] hover:border-[#4a8b3f]"><RefreshCw className="h-4 w-4" /></button></div></Field>
                <Field label="National ID"><input value={nationalId} onChange={(e) => setNationalId(e.target.value)} className={inputCls} /></Field>
                <Field label="First Name *"><input value={firstName} onChange={(e) => setFirstName(e.target.value)} className={inputCls} autoFocus /></Field>
                <Field label="Last Name *"><input value={lastName} onChange={(e) => setLastName(e.target.value)} className={inputCls} /></Field>
                <Field label="Gender"><select value={gender} onChange={(e) => setGender(e.target.value)} className={inputCls}>{GENDERS.map((g) => <option key={g} value={g}>{g || "Select"}</option>)}</select></Field>
                <Field label="Date of Birth"><div className="relative"><CalendarDays className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#5f8f59]" /><input type="date" value={dateOfBirth} onChange={(e) => setDateOfBirth(e.target.value)} className={`${inputCls} pl-10 [color-scheme:dark]`} /></div></Field>
              </div>
            </div>
          </Section>

          <Section title="Employment">
            <div className="grid grid-cols-2 gap-4">
              <Field label="Department"><select value={departmentId} onChange={(e) => { setDepartmentId(e.target.value); setPositionId(""); }} className={inputCls}><option value="">Select department</option>{departments.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}</select></Field>
              <Field label="Position"><select value={positionId} onChange={(e) => setPositionId(e.target.value)} disabled={!departmentId} className={`${inputCls} disabled:opacity-50`}><option value="">{departmentId ? "Select position" : "Select department first"}</option>{positions.map((p) => <option key={p.id} value={p.id}>{p.title}</option>)}</select></Field>
              <Field label="Contract Type"><select value={contractTypeId} onChange={(e) => setContractTypeId(e.target.value)} className={inputCls}><option value="">Select contract type</option>{contractTypes.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select></Field>
              <Field label="Grade"><input value={grade} onChange={(e) => setGrade(e.target.value)} className={inputCls} /></Field>
              <Field label="Date of Start"><div className="relative"><CalendarDays className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#5f8f59]" /><input type="date" value={hireDate} onChange={(e) => setHireDate(e.target.value)} className={`${inputCls} pl-10 [color-scheme:dark]`} /></div></Field>
              <Field label="Employment Status"><select value={employmentStatus} onChange={(e) => setEmploymentStatus(e.target.value)} className={inputCls}>{STATUSES.map((s) => <option key={s} value={s}>{s}</option>)}</select></Field>
              <Field label="Dependants"><input type="number" min="0" value={dependants} onChange={(e) => setDependants(e.target.value)} className={inputCls} /></Field>
            </div>
          </Section>

          <Section title="RSSB & Banking"><div className="grid grid-cols-2 gap-4"><Field label="RSSB Number"><input value={rssbNumber} onChange={(e) => setRssbNumber(e.target.value)} className={inputCls} /></Field><Field label="Bank Name"><input value={bankName} onChange={(e) => setBankName(e.target.value)} className={inputCls} /></Field><Field label="Account Number"><input value={bankAccount} onChange={(e) => setBankAccount(e.target.value)} className={inputCls} /></Field></div></Section>
          <Section title="Contact"><div className="grid grid-cols-2 gap-4"><Field label="Phone"><input value={phone} onChange={(e) => setPhone(e.target.value)} className={inputCls} /></Field><Field label="Email"><input type="email" value={email} onChange={(e) => setEmail(e.target.value)} className={inputCls} /></Field><div className="col-span-2"><Field label="Address"><input value={address} onChange={(e) => setAddress(e.target.value)} className={inputCls} /></Field></div></div></Section>
          <Section title="Salary Information"><div className="grid grid-cols-3 gap-4"><Field label={employee ? "New Basic Salary" : "Basic Salary *"}><input type="number" step="0.01" min="0" value={baseSalary} onChange={(e) => setBaseSalary(e.target.value)} className={inputCls} placeholder="0.00" /></Field><Field label="Salary Effective Date"><div className="relative"><CalendarDays className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[#5f8f59]" /><input type="date" value={salaryEffectiveDate} onChange={(e) => setSalaryEffectiveDate(e.target.value)} className={`${inputCls} pl-10 [color-scheme:dark]`} /></div></Field><Field label="Reason"><input value={salaryReason} onChange={(e) => setSalaryReason(e.target.value)} className={inputCls} placeholder="Promotion / adjustment" /></Field></div></Section>
          {error && <div className="rounded-xl border border-[#333] bg-[#202020] px-4 py-3 text-[12px] text-white">{error}</div>}
          <div className="flex justify-end gap-3 border-t border-[#282828] pt-5"><Button variant="secondary" type="button" onClick={onClose}>Cancel</Button><Button type="submit" disabled={saving}>{saving ? "Saving…" : "Save Employee"}</Button></div>
        </form>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) { return <section><h3 className="mb-3 text-[11px] font-semibold uppercase tracking-[0.12em] text-[#69a860]">{title}</h3>{children}</section>; }
function Field({ label, children }: { label: string; children: ReactNode }) { return <label className="block"><span className="mb-1.5 block text-[11px] font-medium text-[#9b9b9b]">{label}</span>{children}</label>; }
