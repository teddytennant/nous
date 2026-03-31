import { Skeleton } from "@/components/ui/skeleton";

export default function SocialLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-3xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-20" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-12" />
        </div>
        <Skeleton className="h-8 w-32 mb-2" />
        <Skeleton className="h-4 w-64" />
      </header>

      {/* Tab bar */}
      <div className="flex gap-4 mb-8 border-b border-white/[0.06] pb-3">
        <Skeleton className="h-5 w-20" />
        <Skeleton className="h-5 w-20" />
      </div>

      {/* Post composer */}
      <div className="border border-white/[0.06] rounded-sm p-4 mb-8">
        <Skeleton className="h-20 w-full rounded-sm mb-3" />
        <div className="flex justify-between items-center">
          <Skeleton className="h-3 w-16" />
          <Skeleton className="h-8 w-20 rounded-md" />
        </div>
      </div>

      {/* Feed skeleton */}
      <div className="space-y-0 border border-white/[0.06] rounded-sm overflow-hidden">
        {Array.from({ length: 5 }).map((_, i) => (
          <div
            key={i}
            className="p-4 border-b border-white/[0.04] last:border-0"
          >
            <div className="flex items-start gap-3">
              <Skeleton className="w-8 h-8 rounded-full shrink-0" />
              <div className="flex-1 space-y-2">
                <div className="flex items-center gap-2">
                  <Skeleton className="h-3 w-28" />
                  <Skeleton className="h-2.5 w-10" />
                </div>
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <div className="flex gap-4 pt-1">
                  <Skeleton className="h-3 w-10" />
                  <Skeleton className="h-3 w-10" />
                  <Skeleton className="h-3 w-10" />
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
