import type { PayrollRule } from "@/types/payroll";
import { Card } from "@/components/ui/Card";
import { Pencil, Power, Trash2, History } from "lucide-react";

interface RuleTableProps {
  rules: PayrollRule[];
  onEdit: (rule: PayrollRule) => void;
  onToggle: (rule: PayrollRule) => void;
  onDelete: (rule: PayrollRule) => void;
  onShowVersions: (rule: PayrollRule) => void;
}

const TYPE_COLORS: Record<string, string> = {
  earning: "text-[#4a8b3f]",
  deduction: "text-[#e8a44c]",
  tax: "text-[#d96a5a]",
  contribution: "text-[#6a9fd9]",
};

const SIDE_LABEL: Record<string, string> = {
  employee: "Employee",
  employer: "Employer",
};

export function RuleTable({ rules, onEdit, onToggle, onDelete, onShowVersions }: RuleTableProps) {
  if (rules.length === 0) {
    return (
      <Card>
        <div className="py-12 text-center text-[13px] text-[#888888]">
          No payroll rules configured. Click "Add Rule" to create your first component.
        </div>
      </Card>
    );
  }

  return (
    <Card className="overflow-x-auto p-0">
      <table className="w-full">
        <thead>
          <tr className="border-b border-[#2e2e2e] text-left">
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Order</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Code</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Name</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Type</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Calc</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Side</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Rate/Base</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Status</th>
            <th className="px-4 py-3 text-[11px] font-semibold uppercase tracking-wider text-[#888888]">Actions</th>
          </tr>
        </thead>
        <tbody>
          {rules.map((rule) => (
            <tr key={rule.id} className="border-b border-[#222] hover:bg-[#1e1e1e]">
              <td className="px-4 py-3 text-[13px] text-[#888888]">{rule.sort_order}</td>
              <td className="px-4 py-3 text-[13px] font-mono font-medium text-[#e8e8e8]">{rule.code}</td>
              <td className="px-4 py-3 text-[13px] text-[#e8e8e8]">{rule.name}</td>
              <td className={`px-4 py-3 text-[13px] font-medium ${TYPE_COLORS[rule.component_type] ?? "text-[#e8e8e8]"}`}>
                {rule.component_type}
              </td>
              <td className="px-4 py-3 text-[13px] text-[#888888]">{rule.calc_type}</td>
              <td className="px-4 py-3 text-[13px] text-[#888888]">{SIDE_LABEL[rule.side] ?? rule.side}</td>
              <td className="px-4 py-3 text-[13px] text-[#888888]">
                {rule.calc_type === "percentage" && rule.rate !== null
                  ? `${rule.rate}% of ${rule.base_reference ?? "BASIC"}`
                  : rule.calc_type === "formula"
                  ? rule.formula_expression ?? "—"
                  : rule.rate !== null
                  ? String(rule.rate)
                  : "—"}
              </td>
              <td className="px-4 py-3">
                <span
                  className={`inline-block rounded-full px-2 py-0.5 text-[11px] font-medium ${
                    rule.is_active
                      ? "bg-[#1e3a1a] text-[#4a8b3f]"
                      : "bg-[#2a2a2a] text-[#888888]"
                  }`}
                >
                  {rule.is_active ? "Active" : "Inactive"}
                </span>
              </td>
              <td className="px-4 py-3">
                <div className="flex items-center gap-1">
                  <button onClick={() => onEdit(rule)} title="Edit" className="rounded p-1.5 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#e8e8e8]">
                    <Pencil className="h-4 w-4" />
                  </button>
                  <button onClick={() => onToggle(rule)} title={rule.is_active ? "Deactivate" : "Activate"} className="rounded p-1.5 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#e8e8e8]">
                    <Power className="h-4 w-4" />
                  </button>
                  <button onClick={() => onShowVersions(rule)} title="Version history" className="rounded p-1.5 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#e8e8e8]">
                    <History className="h-4 w-4" />
                  </button>
                  <button onClick={() => onDelete(rule)} title="Delete" className="rounded p-1.5 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#d96a5a]">
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}
