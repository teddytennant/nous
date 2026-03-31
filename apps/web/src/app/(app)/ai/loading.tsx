import { Skeleton } from "@/components/ui/skeleton";

export default function AILoading() {
  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-20" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-6" />
        </div>
        <div className="flex items-start justify-between gap-4">
          <div>
            <Skeleton className="h-8 w-10 mb-2" />
            <Skeleton className="h-4 w-48" />
          </div>
          <Skeleton className="h-9 w-28 rounded-md" />
        </div>
      </header>

      {/* Tab bar */}
      <div className="flex gap-4 mb-8 border-b border-white/[0.06] pb-3">
        <Skeleton className="h-5 w-12" />
        <Skeleton className="h-5 w-16" />
        <Skeleton className="h-5 w-28" />
      </div>

      {/* Chat area */}
      <div className="border border-white/[0.06] rounded-sm overflow-hidden">
        {/* Agent selector */}
        <div className="px-4 py-3 border-b border-white/[0.04] flex items-center gap-3">
          <Skeleton className="w-6 h-6 rounded-full" />
          <Skeleton className="h-3.5 w-28" />
        </div>

        {/* Message area */}
        <div className="p-6 space-y-4 min-h-[320px]">
          <div className="flex gap-2">
            <Skeleton className="w-6 h-6 rounded-full shrink-0" />
            <div className="space-y-1.5">
              <Skeleton className="h-4 w-64" />
              <Skeleton className="h-4 w-48" />
            </div>
          </div>
          <div className="flex gap-2 flex-row-reverse">
            <Skeleton className="h-10 w-40 rounded-sm" />
          </div>
          <div className="flex gap-2">
            <Skeleton className="w-6 h-6 rounded-full shrink-0" />
            <div className="space-y-1.5">
              <Skeleton className="h-4 w-72" />
              <Skeleton className="h-4 w-56" />
              <Skeleton className="h-4 w-36" />
            </div>
          </div>
        </div>

        {/* Input */}
        <div className="px-4 py-3 border-t border-white/[0.04]">
          <Skeleton className="h-10 w-full rounded-md" />
        </div>
      </div>
    </div>
  );
}
