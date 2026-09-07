import { Activity } from "lucide-react";
import type { PageHeaderProps } from "@/types";

export function PageHeader({ title, description }: PageHeaderProps) {
  return (
    <div className="mb-6 flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
      <div>
        <div className="mb-1.5 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-[#5fa453]">
          <Activity className="h-3.5 w-3.5" />
          Payroll System V1
        </div>
        <h1 className="text-[24px] font-semibold tracking-tight text-white">{title}</h1>
        {description && <p className="mt-1.5 max-w-3xl text-[13px] leading-6 text-[#777777]">{description}</p>}
      </div>
    </div>
  );
}
