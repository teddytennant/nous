import { Skeleton } from "@/components/ui/skeleton";

export default function FilesLoading() {
  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      {/* Page header */}
      <header className="mb-12">
        <div className="flex items-center gap-1.5 mb-3">
          <Skeleton className="h-2.5 w-20" />
          <span className="text-neutral-800">/</span>
          <Skeleton className="h-2.5 w-10" />
        </div>
        <div className="flex items-start justify-between gap-4">
          <div>
            <Skeleton className="h-8 w-20 mb-2" />
            <Skeleton className="h-4 w-52" />
          </div>
          <Skeleton className="h-9 w-24 rounded-md" />
        </div>
      </header>

      {/* Stats bar */}
      <div className="grid grid-cols-2 sm:flex gap-4 sm:gap-8 mb-8">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i}>
            <Skeleton className="h-2.5 w-14 mb-2" />
            <Skeleton className="h-5 w-12" />
          </div>
        ))}
      </div>

      {/* File list */}
      <div className="border border-white/[0.06] rounded-sm overflow-hidden">
        {/* Table header */}
        <div className="px-4 py-3 border-b border-white/[0.06] flex gap-4">
          <Skeleton className="h-3 w-40 flex-1" />
          <Skeleton className="h-3 w-16" />
          <Skeleton className="h-3 w-20" />
          <Skeleton className="h-3 w-16" />
        </div>

        {/* File rows */}
        {Array.from({ length: 6 }).map((_, i) => (
          <div
            key={i}
            className="flex items-center gap-4 px-4 py-3 border-b border-white/[0.04] last:border-0"
          >
            <Skeleton className="w-5 h-5 rounded shrink-0" />
            <Skeleton className="h-3.5 w-40 flex-1" />
            <Skeleton className="h-3 w-14" />
            <Skeleton className="h-3 w-20" />
            <Skeleton className="h-3 w-14" />
          </div>
        ))}
      </div>
    </div>
  );
}
