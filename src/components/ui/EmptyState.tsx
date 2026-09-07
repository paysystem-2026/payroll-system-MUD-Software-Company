import { LucideIcon } from "lucide-react";
import { ReactNode } from "react";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyState({ icon: Icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex min-h-[220px] flex-col items-center justify-center rounded-2xl border border-dashed border-[#303030] bg-[#111111]/60 px-6 py-12 text-center">
      <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-[#1f1f1f] text-[#4a8b3f] ring-1 ring-[#2d2d2d]">
        <Icon className="h-5 w-5" />
      </div>
      <h3 className="text-[14px] font-semibold text-white">{title}</h3>
      {description && <p className="mt-1.5 max-w-sm text-[12px] leading-5 text-[#777777]">{description}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
