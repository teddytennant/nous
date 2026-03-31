import Link from "next/link";

export default function NotFound() {
  return (
    <div className="flex flex-col items-center justify-center min-h-screen px-6">
      {/* Animated gradient orb */}
      <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] opacity-[0.04] pointer-events-none">
        <div className="w-full h-full rounded-full bg-[radial-gradient(circle,#d4af37_0%,transparent_70%)] animate-[pulse_6s_ease-in-out_infinite]" />
      </div>

      <div className="relative text-center max-w-md">
        <p className="text-[10px] font-mono uppercase tracking-[0.3em] text-neutral-700 mb-6">
          Not Found
        </p>

        <h1 className="text-8xl sm:text-9xl font-extralight tracking-[-0.05em] mb-4">
          <span className="text-white">4</span>
          <span className="text-[#d4af37]">0</span>
          <span className="text-white">4</span>
        </h1>

        <p className="text-sm text-neutral-500 font-light leading-relaxed mb-12">
          This page has drifted into the void.
          <br />
          <span className="text-neutral-600">
            The route you requested doesn&apos;t exist on this node.
          </span>
        </p>

        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Link
            href="/"
            className="group flex items-center gap-2 bg-white text-black px-8 py-3 rounded-md text-sm font-medium hover:bg-neutral-200 transition-colors duration-200"
          >
            Return Home
          </Link>
          <Link
            href="/dashboard"
            className="flex items-center gap-2 border border-white/10 px-8 py-3 rounded-md text-sm font-light text-neutral-400 hover:border-white/20 hover:text-white transition-all duration-200"
          >
            Open App
          </Link>
        </div>
      </div>

      <p className="absolute bottom-8 text-[10px] font-mono text-neutral-800 tracking-wider">
        nous v0.1.0
      </p>
    </div>
  );
}
