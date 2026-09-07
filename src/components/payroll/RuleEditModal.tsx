import { useState, FormEvent } from "react";
import type { PayrollRule, SaveRuleRequest } from "@/types/payroll";
import { payrollService } from "@/services/payroll";
import { Button } from "@/components/ui/Button";
import { X } from "lucide-react";

interface RuleEditModalProps {
  rule: PayrollRule | null;
  rules: PayrollRule[];
  onClose: () => void;
  onSaved: () => void;
}

const COMPONENT_TYPES = ["earning", "deduction", "tax", "contribution"];
const CALC_TYPES = ["fixed", "percentage", "formula"];
const SIDES = ["employee", "employer"];

export function RuleEditModal({ rule, rules, onClose, onSaved }: RuleEditModalProps) {
  const [name, setName] = useState(rule?.name ?? "");
  const [code, setCode] = useState(rule?.code ?? "");
  const [componentType, setComponentType] = useState(rule?.component_type ?? "earning");
  const [calcType, setCalcType] = useState(rule?.calc_type ?? "fixed");
  const [side, setSide] = useState(rule?.side ?? "employee");
  const [rate, setRate] = useState(rule?.rate?.toString() ?? "");
  const [formula, setFormula] = useState(rule?.formula_expression ?? "");
  const [baseRef, setBaseRef] = useState(rule?.base_reference ?? "");
  const [isTaxable, setIsTaxable] = useState(rule?.is_taxable ?? false);
  const [isPensionable, setIsPensionable] = useState(rule?.is_pensionable ?? false);
  const [sortOrder, setSortOrder] = useState<string>(String(rule?.sort_order ?? (rules.length + 1)));
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const otherCodes = rules.filter((r) => r.id !== rule?.id).map((r) => r.code);

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");

    if (name.trim().length < 2) { setError("Name must be at least 2 characters"); return; }
    if (code.trim().length < 2) { setError("Code must be at least 2 characters"); return; }

    if (calcType === "percentage" && (!rate || isNaN(Number(rate)))) {
      setError("Percentage type requires a numeric rate"); return;
    }
    if (calcType === "formula" && !formula.trim()) {
      setError("Formula type requires an expression"); return;
    }

    const request: SaveRuleRequest = {
      id: rule?.id,
      name: name.trim(),
      code: code.trim().toUpperCase(),
      component_type: componentType,
      calc_type: calcType,
      side,
      rate: rate ? Number(rate) : null,
      formula_expression: calcType === "formula" ? formula.trim() : null,
      base_reference: baseRef || null,
      is_taxable: isTaxable,
      is_pensionable: isPensionable,
      sort_order: Number(sortOrder) || 0,
    };

    setSaving(true);
    try {
      const res = await payrollService.saveRule(request);
      if (!res.success && res.errors.length > 0) {
        setError(res.errors.join("; "));
      } else if (!res.success) {
        setError(res.message);
      } else {
        onSaved();
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-2xl border border-[#2e2e2e] bg-[#1a1a1a] p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-[16px] font-semibold text-[#e8e8e8]">
            {rule ? "Edit Rule" : "Add Rule"}
          </h2>
          <button onClick={onClose} className="rounded p-1 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#e8e8e8]">
            <X className="h-5 w-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <Field label="Name">
              <input value={name} onChange={(e) => setName(e.target.value)} className={inputCls} autoFocus />
            </Field>
            <Field label="Code">
              <input value={code} onChange={(e) => setCode(e.target.value.toUpperCase())} className={inputCls} placeholder="e.g. BASIC" />
            </Field>
          </div>

          <div className="grid grid-cols-3 gap-4">
            <Field label="Component Type">
              <select value={componentType} onChange={(e) => setComponentType(e.target.value)} className={inputCls}>
                {COMPONENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
              </select>
            </Field>
            <Field label="Calc Type">
              <select value={calcType} onChange={(e) => setCalcType(e.target.value)} className={inputCls}>
                {CALC_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
              </select>
            </Field>
            <Field label="Side">
              <select value={side} onChange={(e) => setSide(e.target.value)} className={inputCls}>
                {SIDES.map((s) => <option key={s} value={s}>{s}</option>)}
              </select>
            </Field>
          </div>

          {calcType === "percentage" && (
            <div className="grid grid-cols-2 gap-4">
              <Field label="Rate (%)">
                <input type="number" step="0.01" value={rate} onChange={(e) => setRate(e.target.value)} className={inputCls} placeholder="e.g. 6" />
              </Field>
              <Field label="Base Reference">
                <select value={baseRef} onChange={(e) => setBaseRef(e.target.value)} className={inputCls}>
                  <option value="">— Select —</option>
                  {otherCodes.map((c) => <option key={c} value={c}>{c}</option>)}
                </select>
              </Field>
            </div>
          )}

          {calcType === "formula" && (
            <Field label="Formula Expression">
              <textarea
                value={formula}
                onChange={(e) => setFormula(e.target.value)}
                className={`${inputCls} min-h-[80px] resize-y font-mono`}
                placeholder="e.g. BASIC + TRANSPORT + ACCOMMODATION"
              />
              <p className="mt-1 text-[11px] text-[#888888]">
                Reference other component codes by name. Use +, -, *, /, parentheses. e.g. TAXABLE_BASE * 0.1
              </p>
            </Field>
          )}

          {calcType === "fixed" && (
            <Field label="Default Value">
              <input type="number" step="0.01" value={rate} onChange={(e) => setRate(e.target.value)} className={inputCls} placeholder="0" />
            </Field>
          )}

          <div className="grid grid-cols-2 gap-4">
            <label className="flex items-center gap-2 text-[13px] text-[#e8e8e8]">
              <input type="checkbox" checked={isTaxable} onChange={(e) => setIsTaxable(e.target.checked)} className="accent-[#4a8b3f]" />
              Taxable
            </label>
            <label className="flex items-center gap-2 text-[13px] text-[#e8e8e8]">
              <input type="checkbox" checked={isPensionable} onChange={(e) => setIsPensionable(e.target.checked)} className="accent-[#4a8b3f]" />
              Pensionable
            </label>
          </div>

          <Field label="Sort Order">
            <input type="number" value={sortOrder} onChange={(e) => setSortOrder(e.target.value)} className={inputCls} />
          </Field>

          {error && <p className="rounded-lg border border-[#4a8b3f]/35 bg-[#4a8b3f]/10 px-3 py-2 text-[13px] text-white">{error}</p>}

          <div className="flex justify-end gap-2 pt-2">
            <Button variant="secondary" type="button" onClick={onClose}>Cancel</Button>
            <Button type="submit" disabled={saving}>{saving ? "Saving..." : "Save Rule"}</Button>
          </div>
        </form>
      </div>
    </div>
  );
}

const inputCls =
  "w-full rounded-lg border border-[#2e2e2e] bg-[#0d0d0d] px-3 py-2 text-[13px] text-[#e8e8e8] placeholder:text-[#888888] focus:outline-none focus:border-[#4a8b3f]";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="mb-1.5 block text-[12px] font-medium text-[#e8e8e8]">{label}</label>
      {children}
    </div>
  );
}
