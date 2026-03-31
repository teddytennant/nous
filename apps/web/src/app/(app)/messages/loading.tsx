import { Skeleton } from "@/components/ui/skeleton";

export default function MessagesLoading() {
  return (
    <div className="flex h-[calc(100dvh-3.5rem)] md:h-[calc(100vh-3.5rem)]">
      {/* Channel list */}
      <div className="w-full md:w-72 border-r border-white/[0.06] flex flex-col shrink-0">
        {/* Channel header */}
        <div className="px-4 sm:px-6 py-4 border-b border-white/[0.06] flex items-center justify-between">
          <Skeleton className="h-5 w-24" />
          <Skeleton className="h-7 w-7 rounded-md" />
        </div>

        {/* Channel list items */}
        <div className="flex-1 overflow-hidden">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="flex items-center gap-3 px-4 sm:px-6 py-3.5 border-b border-white/[0.04]"
            >
              <Skeleton className="w-7 h-7 rounded-full shrink-0" />
              <div className="flex-1 min-w-0">
                <Skeleton className="h-3.5 w-28 mb-1.5" />
                <Skeleton className="h-3 w-40" />
              </div>
              <Skeleton className="h-2.5 w-6 shrink-0" />
            </div>
          ))}
        </div>
      </div>

      {/* Chat area (hidden on mobile) */}
      <div className="hidden md:flex flex-1 flex-col">
        {/* Chat header */}
        <div className="px-6 py-4 border-b border-white/[0.06] flex items-center gap-3">
          <Skeleton className="w-8 h-8 rounded-full" />
          <div>
            <Skeleton className="h-4 w-32 mb-1" />
            <Skeleton className="h-2.5 w-20" />
          </div>
        </div>

        {/* Messages */}
        <div className="flex-1 p-6 space-y-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div
              key={i}
              className={`flex gap-2 ${i % 2 === 0 ? "" : "flex-row-reverse"}`}
            >
              {i % 2 === 0 && (
                <Skeleton className="w-5 h-5 rounded-full shrink-0 mt-1" />
              )}
              <Skeleton
                className={`h-10 rounded-sm ${i % 2 === 0 ? "w-48" : "w-56"}`}
              />
            </div>
          ))}
        </div>

        {/* Input area */}
        <div className="px-6 py-4 border-t border-white/[0.06]">
          <Skeleton className="h-10 w-full rounded-md" />
        </div>
      </div>
    </div>
  );
}
