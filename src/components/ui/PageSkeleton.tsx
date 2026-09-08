export function PageSkeleton({ variant = "dashboard" }: { variant?: "dashboard" | "table" | "form" | "report" }) {
  const blocks = variant === "table" ? 7 : variant === "form" ? 6 : variant === "report" ? 8 : 5;
  return (
    <div className="space-y-4" aria-busy="true" aria-label="Loading page">
      <div className="space-y-2">
        <div className="skeleton h-6 w-40" />
        <div className="skeleton h-3 w-72 max-w-[70%]" />
      </div>
      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: Math.min(blocks, 4) }).map((_, i) => <div key={i} className="skeleton-card h-24" />)}
      </div>
      <div className="rounded-2xl border border-[#242424] bg-[#111] p-4">
        <div className="mb-4 flex items-center justify-between gap-3">
          <div className="skeleton h-4 w-32" />
          <div className="skeleton h-8 w-24 rounded-lg" />
        </div>
        {Array.from({ length: blocks }).map((_, i) => (
          <div key={i} className="flex items-center gap-3 border-t border-[#1f1f1f] py-3">
            <div className="skeleton h-8 w-8 rounded-lg" />
            <div className="min-w-0 flex-1 space-y-2">
              <div className={`skeleton h-3 ${i % 3 === 0 ? "w-3/4" : "w-1/2"}`} />
              <div className="skeleton h-2.5 w-1/3" />
            </div>
            <div className="skeleton h-7 w-16 rounded-lg" />
          </div>
        ))}
      </div>
    </div>
  );
}

export function InlineSkeleton({ rows = 4 }: { rows?: number }) {
  return (
    <div className="space-y-2" aria-busy="true">
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="flex items-center gap-3 rounded-xl border border-[#242424] bg-[#111] p-3">
          <div className="skeleton h-8 w-8 rounded-lg" />
          <div className="flex-1 space-y-2"><div className="skeleton h-3 w-2/3" /><div className="skeleton h-2.5 w-1/3" /></div>
          <div className="skeleton h-7 w-14 rounded-lg" />
        </div>
      ))}
    </div>
  );
}
