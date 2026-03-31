import { Skeleton } from "@/components/ui/skeleton";

export default function NetworkLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-20" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-16" />
        </div>
        <Skeleton className="h-8 w-28 mb-2" />
        <Skeleton className="h-4 w-56" />
      </header>

      {/* Stats cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03] rounded-sm overflow-hidden mb-10">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="bg-black p-5">
            <Skeleton className="h-2.5 w-16 mb-3" />
            <Skeleton className="h-6 w-12 mb-1" />
            <Skeleton className="h-3 w-20" />
          </div>
        ))}
      </div>

      {/* Subsystems */}
      <div className="mb-10">
        <Skeleton className="h-3 w-24 mb-4" />
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-3 p-4 border border-white/[0.06] rounded-sm"
            >
              <Skeleton className="w-2 h-2 rounded-full shrink-0" />
              <div className="flex-1">
                <Skeleton className="h-3.5 w-24 mb-1" />
                <Skeleton className="h-2.5 w-40" />
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Peer list */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <Skeleton className="h-3 w-20" />
          <Skeleton className="h-8 w-full sm:w-56 rounded-md" />
        </div>
        <div className="border border-white/[0.06] rounded-sm overflow-hidden">
          {Array.from({ length: 5 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-4 px-4 py-3 border-b border-white/[0.04] last:border-0"
            >
              <Skeleton className="w-2 h-2 rounded-full shrink-0" />
              <Skeleton className="h-3 w-36 flex-1" />
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-12" />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
