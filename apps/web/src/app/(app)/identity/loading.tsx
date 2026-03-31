import { Skeleton } from "@/components/ui/skeleton";

export default function IdentityLoading() {
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
        <Skeleton className="h-4 w-56" />
      </header>

      {/* Identity card */}
      <div className="p-6 border border-white/[0.06] rounded-sm mb-10">
        <div className="flex items-center gap-4 mb-6">
          <Skeleton className="w-14 h-14 rounded-full" />
          <div>
            <Skeleton className="h-5 w-32 mb-2" />
            <Skeleton className="h-3 w-48" />
          </div>
        </div>
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Skeleton className="w-4 h-4 rounded shrink-0" />
            <Skeleton className="h-3 w-64" />
          </div>
          <div className="flex items-center gap-2">
            <Skeleton className="w-4 h-4 rounded shrink-0" />
            <Skeleton className="h-3 w-40" />
          </div>
        </div>
      </div>

      {/* Reputation */}
      <div className="mb-10">
        <Skeleton className="h-3 w-20 mb-4" />
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="p-4 border border-white/[0.06] rounded-sm"
            >
              <Skeleton className="h-2.5 w-16 mb-2" />
              <Skeleton className="h-6 w-10 mb-1" />
              <Skeleton className="h-2 w-full rounded-full" />
            </div>
          ))}
        </div>
      </div>

      {/* Credentials */}
      <div>
        <Skeleton className="h-3 w-24 mb-4" />
        <div className="space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-4 p-4 border border-white/[0.06] rounded-sm"
            >
              <Skeleton className="w-8 h-8 rounded-md shrink-0" />
              <div className="flex-1">
                <Skeleton className="h-3.5 w-36 mb-1" />
                <Skeleton className="h-2.5 w-24" />
              </div>
              <Skeleton className="h-5 w-16 rounded-sm shrink-0" />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
