import { Skeleton } from "@/components/ui/skeleton";

export default function GovernanceLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-14" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-24" />
        </div>
        <div className="flex items-start justify-between gap-4">
          <div>
            <Skeleton className="h-8 w-36 mb-2" />
            <Skeleton className="h-4 w-60" />
          </div>
          <Skeleton className="h-9 w-32 rounded-md" />
        </div>
      </header>

      {/* Tab bar */}
      <div className="flex gap-4 mb-8 border-b border-white/[0.06] pb-3">
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-14" />
        <Skeleton className="h-5 w-24" />
      </div>

      {/* Analytics placeholder — 2x2 chart grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-10">
        {Array.from({ length: 4 }).map((_, i) => (
          <div
            key={i}
            className="p-6 border border-white/[0.06] rounded-sm"
          >
            <Skeleton className="h-3 w-28 mb-4" />
            <Skeleton className="h-40 w-full rounded-sm" />
          </div>
        ))}
      </div>

      {/* Proposal list */}
      <div>
        <Skeleton className="h-3 w-32 mb-4" />
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="p-5 border border-white/[0.06] rounded-sm"
            >
              <div className="flex items-start justify-between gap-4 mb-3">
                <div className="flex-1">
                  <Skeleton className="h-4 w-48 mb-2" />
                  <Skeleton className="h-3 w-full" />
                </div>
                <Skeleton className="h-6 w-16 rounded-sm shrink-0" />
              </div>
              <div className="flex gap-6">
                <Skeleton className="h-2.5 w-20" />
                <Skeleton className="h-2.5 w-16" />
                <Skeleton className="h-2.5 w-24" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
