"use client";

import { useState, useEffect, useCallback, startTransition } from "react";
import { useConnection } from "@/components/connection-status";
import { RefreshCw } from "lucide-react";

/* ── Disconnected Network Illustration ──────────────────────────────────── */
/* Minimalist, geometric, monochrome + gold accent (#d4af37).               */
/* Five nodes with broken/dashed connections — representing an offline mesh. */

function DisconnectedIllustration() {
  return (
    <svg
      width="160"
      height="160"
      viewBox="0 0 160 160"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="offline-illustration"
      aria-hidden="true"
    >
      {/* Central node — dimmed */}
      <circle
        cx="80"
        cy="80"
        r="8"
        stroke="#d4af37"
        strokeWidth="1"
        opacity="0.25"
      />
      <circle cx="80" cy="80" r="3" fill="#d4af37" opacity="0.15" />

      {/* Outer nodes */}
      <circle
        cx="80"
        cy="28"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.2"
      />
      <circle
        cx="130"
        cy="56"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.15"
      />
      <circle
        cx="130"
        cy="104"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.2"
      />
      <circle
        cx="80"
        cy="132"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.15"
      />
      <circle
        cx="30"
        cy="104"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.2"
      />
      <circle
        cx="30"
        cy="56"
        r="6"
        stroke="currentColor"
        strokeWidth="1"
        opacity="0.15"
      />

      {/* Broken connections — dashed lines from center to nodes */}
      <line
        x1="80"
        y1="72"
        x2="80"
        y2="34"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.1"
        strokeDasharray="3 5"
      />
      <line
        x1="87"
        y1="75"
        x2="125"
        y2="60"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.08"
        strokeDasharray="3 5"
      />
      <line
        x1="87"
        y1="85"
        x2="125"
        y2="100"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.1"
        strokeDasharray="3 5"
      />
      <line
        x1="80"
        y1="88"
        x2="80"
        y2="126"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.08"
        strokeDasharray="3 5"
      />
      <line
        x1="73"
        y1="85"
        x2="35"
        y2="100"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.1"
        strokeDasharray="3 5"
      />
      <line
        x1="73"
        y1="75"
        x2="35"
        y2="60"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.08"
        strokeDasharray="3 5"
      />

      {/* Cross mark over center — subtle "X" indicating disconnect */}
      <line
        x1="73"
        y1="73"
        x2="87"
        y2="87"
        stroke="#d4af37"
        strokeWidth="1"
        opacity="0.3"
        strokeLinecap="round"
      />
      <line
        x1="87"
        y1="73"
        x2="73"
        y2="87"
        stroke="#d4af37"
        strokeWidth="1"
        opacity="0.3"
        strokeLinecap="round"
      />

      {/* Subtle outer ring — broken orbit */}
      <circle
        cx="80"
        cy="80"
        r="56"
        stroke="currentColor"
        strokeWidth="0.5"
        opacity="0.05"
        strokeDasharray="8 12"
      />
    </svg>
  );
}

/* ── Retry countdown ────────────────────────────────────────────────────── */

const RETRY_INTERVAL = 15; // seconds — matches ConnectionProvider polling

/* ── OfflineState ───────────────────────────────────────────────────────── */

export function OfflineState() {
  const { retry } = useConnection();
  const [countdown, setCountdown] = useState(RETRY_INTERVAL);
  const [retrying, setRetrying] = useState(false);

  // Countdown timer — resets every RETRY_INTERVAL seconds
  useEffect(() => {
    startTransition(() => setCountdown(RETRY_INTERVAL));
    const interval = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) return RETRY_INTERVAL;
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleRetry = useCallback(() => {
    setRetrying(true);
    retry();
    // Reset visual state after a brief moment
    setTimeout(() => {
      setRetrying(false);
      setCountdown(RETRY_INTERVAL);
    }, 1500);
  }, [retry]);

  const apiUrl =
    typeof window !== "undefined"
      ? process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api/v1"
      : "";

  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] px-6 py-20 offline-state-enter">
      {/* Illustration */}
      <div className="mb-10 text-neutral-700">
        <DisconnectedIllustration />
      </div>

      {/* Title */}
      <h2 className="text-xl sm:text-2xl font-extralight tracking-[-0.02em] text-neutral-300 mb-3 text-center">
        Unable to reach{" "}
        <span className="text-[#d4af37]">Nous</span>
      </h2>

      {/* Description */}
      <p className="text-sm text-neutral-600 font-light text-center max-w-sm leading-relaxed mb-2">
        The API server isn&apos;t responding. Make sure the Nous node is running locally.
      </p>

      {/* API URL */}
      <p className="text-[10px] font-mono text-neutral-700 tracking-wider mb-8">
        {apiUrl}
      </p>

      {/* Retry button */}
      <button
        onClick={handleRetry}
        disabled={retrying}
        className="group flex items-center gap-2.5 px-6 py-2.5 border border-white/[0.08] rounded-md text-sm font-light text-neutral-400 hover:border-[#d4af37]/30 hover:text-[#d4af37] transition-all duration-200 disabled:opacity-50"
      >
        <RefreshCw
          className={`w-3.5 h-3.5 ${retrying ? "animate-spin" : "group-hover:rotate-45 transition-transform duration-300"}`}
        />
        {retrying ? "Reconnecting..." : "Retry Now"}
      </button>

      {/* Auto-retry indicator */}
      <div className="mt-6 flex items-center gap-2">
        <span className="inline-block w-1.5 h-1.5 rounded-full bg-neutral-700 offline-pulse" />
        <p className="text-[10px] font-mono text-neutral-700 tracking-wider">
          Auto-retry in {countdown}s
        </p>
      </div>

      {/* Help hint */}
      <div className="mt-12 p-4 border border-white/[0.04] rounded-sm max-w-sm w-full">
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
          Quick fix
        </p>
        <code className="block text-xs font-mono text-neutral-500 leading-relaxed">
          cargo run --bin nous-api
        </code>
      </div>
    </div>
  );
}

/* ── ConnectingState ────────────────────────────────────────────────────── */
/* Shown briefly during initial connection attempt.                         */

export function ConnectingState() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] px-6 py-20 offline-state-enter">
      {/* Pulsing gold dot */}
      <div className="mb-8">
        <div className="w-3 h-3 rounded-full bg-[#d4af37] connecting-pulse" />
      </div>

      <p className="text-sm text-neutral-500 font-light">
        Connecting to Nous...
      </p>
    </div>
  );
}
