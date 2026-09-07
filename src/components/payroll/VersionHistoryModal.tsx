import { useState, useEffect } from "react";
import type { RuleVersion } from "@/types/payroll";
import { payrollService } from "@/services/payroll";
import { X, History } from "lucide-react";

interface VersionHistoryModalProps {
  ruleId: number;
  onClose: () => void;
}

export function VersionHistoryModal({ ruleId, onClose }: VersionHistoryModalProps) {
  const [versions, setVersions] = useState<RuleVersion[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    payrollService
      .getVersions(ruleId)
      .then((res) => setVersions(res.versions))
      .catch(() => setVersions([]))
      .finally(() => setLoading(false));
  }, [ruleId]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={onClose}>
      <div
        className="max-h-[80vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-[#2e2e2e] bg-[#1a1a1a] p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <History className="h-5 w-5 text-[#4a8b3f]" />
            <h2 className="text-[16px] font-semibold text-[#e8e8e8]">Version History</h2>
          </div>
          <button onClick={onClose} className="rounded p-1 text-[#888888] hover:bg-[#2e2e2e] hover:text-[#e8e8e8]">
            <X className="h-5 w-5" />
          </button>
        </div>

        {loading ? (
          <div className="py-8 text-center text-[13px] text-[#888888]">Loading...</div>
        ) : versions.length === 0 ? (
          <div className="py-8 text-center text-[13px] text-[#888888]">No version history found.</div>
        ) : (
          <div className="space-y-2">
            {versions.map((v) => (
              <div key={v.id} className="rounded-lg border border-[#2e2e2e] bg-[#242424] p-4">
                <div className="mb-2 flex items-center justify-between">
                  <span className="rounded-full bg-[#1e3a1a] px-2 py-0.5 text-[11px] font-medium text-[#4a8b3f]">
                    v{v.version}
                  </span>
                  <span className="text-[11px] text-[#888888]">{v.created_at}</span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-[12px]">
                  <div>
                    <span className="text-[#888888]">Name: </span>
                    <span className="text-[#e8e8e8]">{v.name}</span>
                  </div>
                  <div>
                    <span className="text-[#888888]">Code: </span>
                    <span className="font-mono text-[#e8e8e8]">{v.code}</span>
                  </div>
                  <div>
                    <span className="text-[#888888]">Type: </span>
                    <span className="text-[#e8e8e8]">{v.component_type}</span>
                  </div>
                  <div>
                    <span className="text-[#888888]">Calc: </span>
                    <span className="text-[#e8e8e8]">{v.calc_type}</span>
                  </div>
                  {v.rate !== null && (
                    <div>
                      <span className="text-[#888888]">Rate: </span>
                      <span className="text-[#e8e8e8]">{v.rate}</span>
                    </div>
                  )}
                  {v.formula_expression && (
                    <div className="col-span-2">
                      <span className="text-[#888888]">Formula: </span>
                      <span className="font-mono text-[#e8e8e8]">{v.formula_expression}</span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
