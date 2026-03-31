import { Skeleton } from "@/components/ui/skeleton";

export default function DashboardLoading() {
  return (
    <div className="p-6 sm:p-8 max-w-5xl">
      {/* Welcome header */}
      <header className="mb-12">
        <Skeleton className="h-3 w-40 mb-3" />
        <Skeleton className="h-9 w-72 mb-2" />
        <Skeleton className="h-4 w-56" />
      </header>

      {/* Stats grid */}
      <section className="mb-12">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03] rounded-sm overflow-hidden">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="bg-black p-5 sm:p-6">
              <div className="flex items-center gap-2 mb-3">
                <Skeleton className="w-3 h-3 rounded" />
                <Skeleton className="h-2.5 w-14" />
              </div>
              <Skeleton className="h-7 w-16 mb-1" />
              <Skeleton className="h-3 w-24" />
            </div>
          ))}
        </div>
      </section>

      {/* Quick Actions */}
      <section className="mb-12">
        <Skeleton className="h-3 w-24 mb-6" />
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-4 p-4 border border-white/[0.06] rounded-sm"
            >
              <Skeleton className="w-10 h-10 rounded-md shrink-0" />
              <div className="flex-1">
                <Skeleton className="h-4 w-24 mb-1.5" />
                <Skeleton className="h-3 w-32" />
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Two-column: Feed + Subsystems */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-12">
        <section>
          <div className="flex items-center justify-between mb-6">
            <Skeleton className="h-3 w-28" />
            <Skeleton className="h-3 w-16" />
          </div>
          <div className="border border-white/[0.06] rounded-sm overflow-hidden">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="flex items-start gap-3 py-3 px-4">
                <Skeleton className="w-7 h-7 rounded-full shrink-0" />
                <div className="flex-1 space-y-2">
                  <Skeleton className="h-3 w-28" />
                  <Skeleton className="h-4 w-full" />
                </div>
                <Skeleton className="h-3 w-12 shrink-0" />
              </div>
            ))}
          </div>
        </section>
        <section>
          <Skeleton className="h-3 w-24 mb-6" />
          <div className="border border-white/[0.06] rounded-sm overflow-hidden">
            {Array.from({ length: 4 }).map((_, i) => (
              <div
                key={i}
                className="flex items-center gap-3 py-3 px-4 border-b border-white/[0.04] last:border-0"
              >
                <Skeleton className="w-2 h-2 rounded-full shrink-0" />
                <Skeleton className="h-3.5 w-24 flex-1" />
                <Skeleton className="h-3 w-10 shrink-0" />
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
