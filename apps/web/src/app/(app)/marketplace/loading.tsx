import { Skeleton } from "@/components/ui/skeleton";

export default function MarketplaceLoading() {
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
            <Skeleton className="h-4 w-56" />
          </div>
          <Skeleton className="h-9 w-28 rounded-md" />
        </div>
      </header>

      {/* Tab bar */}
      <div className="flex gap-4 mb-6 border-b border-white/[0.06] pb-3">
        <Skeleton className="h-5 w-16" />
        <Skeleton className="h-5 w-14" />
        <Skeleton className="h-5 w-16" />
        <Skeleton className="h-5 w-14" />
      </div>

      {/* Category filter */}
      <div className="flex gap-2 mb-8 overflow-x-auto">
        {Array.from({ length: 7 }).map((_, i) => (
          <Skeleton key={i} className="h-7 w-16 rounded-md shrink-0" />
        ))}
      </div>

      {/* Listing grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <div
            key={i}
            className="p-5 border border-white/[0.06] rounded-sm"
          >
            <div className="flex items-start justify-between mb-4">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-5 w-14 rounded-sm" />
            </div>
            <Skeleton className="h-3 w-full mb-1.5" />
            <Skeleton className="h-3 w-2/3 mb-4" />
            <div className="flex items-center justify-between">
              <Skeleton className="h-5 w-20" />
              <Skeleton className="h-3 w-16" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
