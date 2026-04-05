"use client";

import { useEffect } from "react";
import Link from "next/link";
import { RotateCw } from "lucide-react";

export default function Error({
  error,
  unstable_retry,
}: {
  error: Error & { digest?: string };
  unstable_retry: () => void;
}) {
  useEffect(() => {
    console.error("[Nous] Runtime error:", error);
  }, [error]);

  return (
    <div className="flex flex-col items-center justify-center min-h-screen px-6">
      {/* Animated gradient orb */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] opacity-[0.04] pointer-events-none">
        <div className="w-full h-full rounded-full bg-[radial-gradient(circle,#d4af37_0%,transparent_70%)] animate-[pulse_6s_ease-in-out_infinite]" />
      </div>

      <div className="relative text-center max-w-md">
        <p className="text-[10px] font-mono uppercase tracking-[0.3em] text-neutral-700 mb-6">
          Runtime Error
        </p>

        <h1 className="text-6xl sm:text-7xl font-extralight tracking-[-0.05em] mb-4">
          <span className="text-white">Something</span>{" "}
          <span className="text-[#d4af37]">broke</span>
        </h1>

        <p className="text-sm text-neutral-500 font-light leading-relaxed mb-4">
          An unexpected error occurred while rendering this page.
        </p>

        {error.digest && (
          <p className="text-[10px] font-mono text-neutral-700 mb-8">
            Digest: {error.digest}
          </p>
        )}

        {error.message && process.env.NODE_ENV === "development" && (
          <div className="mb-8 p-4 bg-white/[0.02] border border-red-500/20 rounded-sm text-left">
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-red-500/60 mb-2">
              Error Details
            </p>
            <p className="text-xs font-mono text-red-400/80 break-all leading-relaxed">
              {error.message}
            </p>
          </div>
        )}

        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <button
            onClick={() => unstable_retry()}
            className="group flex items-center gap-2 bg-[#d4af37] text-black px-8 py-3 rounded-md text-sm font-medium hover:bg-[#c4a030] transition-colors duration-200"
          >
            <RotateCw className="w-4 h-4 group-hover:rotate-180 transition-transform duration-300" />
            Try Again
          </button>
          <Link
            href="/dashboard"
            className="flex items-center gap-2 border border-white/10 px-8 py-3 rounded-md text-sm font-light text-neutral-400 hover:border-white/20 hover:text-white transition-all duration-200"
          >
            Go to Dashboard
          </Link>
        </div>
      </div>

      <p className="absolute bottom-8 text-[10px] font-mono text-neutral-800 tracking-wider">
        nous v0.1.0
      </p>
    </div>
  );
}
