import { LucideIcon } from "lucide-react";
import { PageHeader } from "@/components/ui/PageHeader";
import { Card } from "@/components/ui/Card";

interface PlaceholderPageProps {
  title: string;
  description: string;
  icon: LucideIcon;
}

export function PlaceholderPage({ title, description, icon: Icon }: PlaceholderPageProps) {
  return (
    <div>
      <PageHeader title={title} description={description} />
      <Card>
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <div className="mb-4 flex h-14 w-14 items-center justify-center rounded-full bg-[#242424]">
            <Icon className="h-6 w-6 text-[#888888]" />
          </div>
          <h3 className="text-[15px] font-medium text-[#e8e8e8]">{title}</h3>
          <p className="mt-1 max-w-sm text-[13px] text-[#888888]">
            This module is part of the approved architecture and will be implemented in a later phase.
          </p>
        </div>
      </Card>
    </div>
  );
}
