import { Skeleton } from "@/components/ui/skeleton";

export default function SettingsLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-3xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-16" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-16" />
        </div>
        <Skeleton className="h-8 w-28 mb-2" />
        <Skeleton className="h-4 w-48" />
      </header>

      {/* Node status */}
      <div className="mb-10 p-5 border border-white/[0.06] rounded-sm">
        <div className="flex items-center gap-3 mb-4">
          <Skeleton className="w-2 h-2 rounded-full" />
          <Skeleton className="h-3.5 w-24" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i}>
              <Skeleton className="h-2.5 w-14 mb-2" />
              <Skeleton className="h-4 w-20" />
            </div>
          ))}
        </div>
      </div>

      {/* Profile section */}
      <div className="mb-10">
        <Skeleton className="h-3 w-16 mb-4" />
        <div className="space-y-4 p-5 border border-white/[0.06] rounded-sm">
          <div>
            <Skeleton className="h-3 w-20 mb-2" />
            <Skeleton className="h-9 w-full rounded-md" />
          </div>
          <div>
            <Skeleton className="h-3 w-20 mb-2" />
            <Skeleton className="h-9 w-full rounded-md" />
          </div>
          <div>
            <Skeleton className="h-3 w-20 mb-2" />
            <Skeleton className="h-9 w-full rounded-md" />
          </div>
        </div>
      </div>

      {/* Appearance section */}
      <div className="mb-10">
        <Skeleton className="h-3 w-24 mb-4" />
        <div className="p-5 border border-white/[0.06] rounded-sm">
          <div className="flex items-center justify-between">
            <div>
              <Skeleton className="h-3.5 w-16 mb-1" />
              <Skeleton className="h-3 w-32" />
            </div>
            <Skeleton className="h-8 w-16 rounded-md" />
          </div>
        </div>
      </div>

      {/* Danger zone */}
      <div>
        <Skeleton className="h-3 w-24 mb-4" />
        <div className="p-5 border border-red-500/10 rounded-sm">
          <div className="flex items-center justify-between">
            <div>
              <Skeleton className="h-3.5 w-32 mb-1" />
              <Skeleton className="h-3 w-56" />
            </div>
            <Skeleton className="h-8 w-20 rounded-md" />
          </div>
        </div>
      </div>
    </div>
  );
}
