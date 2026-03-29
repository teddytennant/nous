"use client";

import { useState, useEffect } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { node, type NodeInfo } from "@/lib/api";
import { useConnection } from "@/components/connection-status";

const features = [
  { name: "X3DH", description: "Extended Triple Diffie-Hellman key agreement" },
  { name: "Double Ratchet", description: "Forward-secret message encryption" },
  {
    name: "ZK Proofs",
    description: "Schnorr proofs, Pedersen commitments, range proofs",
  },
  { name: "GraphQL", description: "Full query and mutation API" },
  {
    name: "Quadratic Voting",
    description: "Sybil-resistant governance with ZK privacy",
  },
  { name: "CRDT Storage", description: "Conflict-free replicated data" },
  { name: "REST API", description: "36 endpoints across 5 domains" },
  { name: "gRPC", description: "Binary protocol for node communication" },
  { name: "Nostr Relay", description: "NIP-01 WebSocket relay bridge" },
  { name: "OpenAPI", description: "Auto-generated API documentation" },
];

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
                <p className="text-2xl font-extralight mb-1">
                  {stat.value}
                </p>
                <p className="text-xs text-neutral-600 font-light font-mono truncate">
                  {stat.detail}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Protocol Modules
        </h2>
        <div className="space-y-px">
          {features.map((f) => {
            const isLive =
              nodeInfo?.features.some(
                (feat) =>
                  feat.toLowerCase().includes(f.name.toLowerCase().split(" ")[0])
              ) ?? false;

            return (
              <div
                key={f.name}
                className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150"
              >
                <div>
                  <p className="text-sm font-light">{f.name}</p>
                  <p className="text-xs text-neutral-600 font-light mt-0.5">
                    {f.description}
                  </p>
                </div>
                <span
                  className={`text-[10px] font-mono uppercase tracking-wider ${
                    isLive ? "text-[#d4af37]" : "text-neutral-700"
                  }`}
                >
                  {isLive ? "live" : "ready"}
                </span>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
}
