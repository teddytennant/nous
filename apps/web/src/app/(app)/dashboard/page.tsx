"use client";

import { useState, useEffect } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { node, type NodeInfo } from "@/lib/api";
import { useConnection } from "@/components/connection-status";
import { SubsystemsWidget } from "@/components/subsystems";

export default function DashboardPage() {
  const { status: apiStatus, health } = useConnection();
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);

  useEffect(() => {
    async function fetchNodeInfo() {
      try {
        const n = await node.info();
        setNodeInfo(n);
      } catch {
        setNodeInfo(null);
      }
    }
    fetchNodeInfo();
    const interval = setInterval(fetchNodeInfo, 30000);
    return () => clearInterval(interval);
  }, []);

  function formatUptime(ms: number): string {
    const s = Math.floor(ms / 1000);
    if (s < 60) return `${s}s`;
    if (s < 3600) return `${Math.floor(s / 60)}m`;
    return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
  }

  const stats = [
    {
      label: "API",
      value: apiStatus === "online" ? "Online" : apiStatus === "offline" ? "Offline" : "...",
      detail: apiStatus === "online" && health ? `v${health.version}` : "connecting",
    },
    {
      label: "Uptime",
      value: health ? formatUptime(health.uptime_ms) : "—",
      detail: "since last restart",
    },
    {
      label: "Protocol",
      value: nodeInfo?.protocol || "—",
      detail: nodeInfo ? `v${nodeInfo.version}` : "",
    },
    {
      label: "Features",
      value: nodeInfo ? String(nodeInfo.features.length) : "—",
      detail: "active modules",
    },
  ];

  return (
    <div className="p-8 max-w-5xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Dashboard
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          System overview and node status
        </p>
      </header>

      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Status
        </h2>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03]">
          {stats.map((stat) => (
            <Card
              key={stat.label}
              className="bg-black border-0 rounded-none p-6"
            >
              <CardContent className="p-0">
                <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                  {stat.label}
                </p>
                {stat.value === "—" || stat.value === "..." ? (
                  <>
                    <Skeleton className="h-7 w-16 mb-1" />
                    <Skeleton className="h-3 w-24" />
                  </>
                ) : (
                  <>
                    <p className="text-2xl font-extralight mb-1">
                      {stat.value}
                    </p>
                    <p className="text-xs text-neutral-600 font-light font-mono truncate">
                      {stat.detail}
                    </p>
                  </>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Subsystems
        </h2>
        <SubsystemsWidget />
      </section>
    </div>
  );
}
