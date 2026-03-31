import { Skeleton } from "@/components/ui/skeleton";

export default function WalletLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-14" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-14" />
        </div>
        <div className="flex items-start justify-between gap-4">
          <div>
            <Skeleton className="h-8 w-28 mb-2" />
            <Skeleton className="h-4 w-52" />
          </div>
          <Skeleton className="h-9 w-24 rounded-md" />
        </div>
      </header>

      {/* Tab bar */}
      <div className="flex gap-4 sm:gap-8 mb-8">
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-16" />
        <Skeleton className="h-5 w-16" />
      </div>

      {/* Balance cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-10">
        {Array.from({ length: 3 }).map((_, i) => (
          <div
            key={i}
            className="p-6 border border-white/[0.06] rounded-sm"
          >
            <Skeleton className="h-2.5 w-12 mb-3" />
            <Skeleton className="h-8 w-24 mb-1" />
            <Skeleton className="h-3 w-16" />
          </div>
        ))}
      </div>

      {/* Transaction list */}
      <div>
        <Skeleton className="h-3 w-28 mb-4" />
        <div className="border border-white/[0.06] rounded-sm overflow-hidden">
          {Array.from({ length: 5 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-4 px-4 py-3.5 border-b border-white/[0.04] last:border-0"
            >
              <Skeleton className="w-8 h-8 rounded-md shrink-0" />
              <div className="flex-1">
                <Skeleton className="h-3.5 w-32 mb-1" />
                <Skeleton className="h-2.5 w-24" />
              </div>
              <div className="text-right">
                <Skeleton className="h-4 w-16 mb-1 ml-auto" />
                <Skeleton className="h-2.5 w-10 ml-auto" />
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
