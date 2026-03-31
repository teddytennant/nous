"use client";

import { useState, useEffect, useCallback } from "react";
import { node, type SubsystemStatus, type SubsystemsResponse } from "@/lib/api";

function statusColor(status: string): string {
  switch (status) {
    case "healthy":
      return "text-[#d4af37]";
    case "degraded":
      return "text-amber-500";
    case "down":
      return "text-red-500/70";
    default:
      return "text-neutral-600";
  }
}

function statusDot(status: string): string {
  switch (status) {
    case "healthy":
      return "bg-[#d4af37]";
    case "degraded":
      return "bg-amber-500";
    case "down":
      return "bg-red-500/70";
    default:
      return "bg-neutral-700";
  }
}

function SubsystemRow({ sub }: { sub: SubsystemStatus }) {
  return (
    <div className="flex items-center justify-between py-3.5 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150">
      <div className="flex items-center gap-3">
        <span className={`w-1.5 h-1.5 rounded-full ${statusDot(sub.status)}`} />
        <div>
          <p className="text-sm font-light capitalize">{sub.name}</p>
          {sub.message && (
            <p className="text-[11px] text-neutral-600 font-light mt-0.5">
              {sub.message}
            </p>
          )}
        </div>
      </div>
      <div className="flex items-center gap-6">
        {sub.active_count > 0 && (
          <span className="text-xs font-mono text-neutral-500 tabular-nums">
            {sub.active_count}
          </span>
        )}
        <span
          className={`text-[10px] font-mono uppercase tracking-wider ${statusColor(sub.status)}`}
        >
          {sub.status}
        </span>
      </div>
    </div>
  );
}

export function SubsystemsSkeleton() {
  return (
    <div className="space-y-px">
      {Array.from({ length: 8 }).map((_, i) => (
        <div
          key={i}
          className="flex items-center justify-between py-3.5 px-5 bg-white/[0.01]"
        >
          <div className="flex items-center gap-3">
            <span className="w-1.5 h-1.5 rounded-full bg-neutral-800 animate-pulse" />
            <div className="h-3.5 w-24 bg-neutral-800 rounded animate-pulse" />
          </div>
          <div className="h-3 w-14 bg-neutral-800 rounded animate-pulse" />
        </div>
      ))}
    </div>
  );
}

export function SubsystemsWidget() {
  const [data, setData] = useState<SubsystemsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchSubsystems = useCallback(async () => {
    try {
      const result = await node.subsystems();
      setData(result);
      setError(null);
    } catch {
      setError("Unable to fetch subsystems");
    }
  }, []);

  useEffect(() => {
    fetchSubsystems();
    const interval = setInterval(fetchSubsystems, 10000);
    return () => clearInterval(interval);
  }, [fetchSubsystems]);

  if (error) {
    return (
      <div className="py-8 text-center">
        <p className="text-xs text-neutral-600 font-mono">{error}</p>
      </div>
    );
  }

  if (!data) {
    return <SubsystemsSkeleton />;
  }

  return (
    <div>
      <div className="flex items-center gap-3 mb-4 px-5">
        <span
          className={`w-2 h-2 rounded-full ${statusDot(data.overall)}`}
        />
        <span
          className={`text-[10px] font-mono uppercase tracking-wider ${statusColor(data.overall)}`}
        >
          {data.overall === "healthy"
            ? "All systems operational"
            : data.overall === "degraded"
              ? "Some systems degraded"
              : "Systems down"}
        </span>
      </div>
      <div className="space-y-px">
        {data.subsystems.map((sub) => (
          <SubsystemRow key={sub.name} sub={sub} />
        ))}
      </div>
    </div>
  );
}
