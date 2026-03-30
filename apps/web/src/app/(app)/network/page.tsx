"use client";

import { useCallback, useEffect, useMemo, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { node, type HealthResponse, type NodeInfo } from "@/lib/api";

interface Peer {
  id: string;
  address: string;
  protocol: string;
  latency_ms: number;
  connected_at: string;
  bytes_sent: number;
  bytes_recv: number;
  status: "connected" | "connecting" | "disconnected";
}

interface Subsystem {
  name: string;
  status: "up" | "degraded" | "down" | "unknown";
  failure_rate: number;
  check_count: number;
  description: string;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return `${Math.floor(diff / 86_400_000)}d ago`;
}

function truncateId(id: string): string {
  if (id.length > 20) return `${id.slice(0, 8)}...${id.slice(-6)}`;
  return id;
}

const SUBSYSTEMS: Subsystem[] = [
  { name: "Networking", status: "up", failure_rate: 0, check_count: 128, description: "libp2p transport, gossipsub, relay" },
  { name: "Identity", status: "up", failure_rate: 0, check_count: 64, description: "DID resolver, credential store" },
  { name: "Messaging", status: "up", failure_rate: 0, check_count: 96, description: "E2E encryption, channel management" },
  { name: "Governance", status: "up", failure_rate: 0, check_count: 48, description: "DAO engine, vote tallying" },
  { name: "Storage", status: "up", failure_rate: 0, check_count: 112, description: "SQLite, CRDT replication" },
  { name: "Payments", status: "up", failure_rate: 0, check_count: 56, description: "Wallet, escrow, streaming" },
  { name: "Social", status: "up", failure_rate: 0, check_count: 72, description: "Feed, follow graph, Nostr relay" },
  { name: "AI", status: "up", failure_rate: 0, check_count: 32, description: "Inference, embeddings, HNSW index" },
];

function statusColor(status: string): string {
  switch (status) {
    case "up":
    case "connected":
      return "bg-emerald-500";
    case "degraded":
    case "connecting":
      return "bg-yellow-500";
    case "down":
    case "disconnected":
      return "bg-red-500";
    default:
      return "bg-neutral-700";
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case "up":
      return "operational";
    case "degraded":
      return "degraded";
    case "down":
      return "down";
    default:
      return "unknown";
  }
}

export default function NetworkPage() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    try {
      const [h, n] = await Promise.all([node.health(), node.info()]);
      setHealth(h);
      setNodeInfo(n);
      setError(null);
    } catch {
      setError("API offline");
    }
  }, []);

  useEffect(() => {
    startTransition(() => { fetchData(); });
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [fetchData]);

  // Simulated peer list (in production, would come from /api/v1/peers)
  const peers: Peer[] = useMemo(() => {
    if (!health || health.status !== "ok") return [];
    const now = new Date();
    return [
      {
        id: "12D3KooWRf7b...q4xN",
        address: "/ip4/10.0.1.5/tcp/9000",
        protocol: "/nous/1.0.0",
        latency_ms: 12,
        connected_at: new Date(now.getTime() - 3_600_000 * 4).toISOString(),
        bytes_sent: 15_240_192,
        bytes_recv: 22_891_520,
        status: "connected" as const,
      },
      {
        id: "12D3KooWHx2L...r7mK",
        address: "/ip4/172.16.0.8/tcp/9000",
        protocol: "/nous/1.0.0",
        latency_ms: 34,
        connected_at: new Date(now.getTime() - 3_600_000 * 2).toISOString(),
        bytes_sent: 8_192_000,
        bytes_recv: 12_480_512,
        status: "connected" as const,
      },
      {
        id: "12D3KooWYt9Z...k3pQ",
        address: "/dns4/relay.nous.dev/tcp/443/wss",
        protocol: "/nous/1.0.0",
        latency_ms: 87,
        connected_at: new Date(now.getTime() - 3_600_000 * 8).toISOString(),
        bytes_sent: 4_096_000,
        bytes_recv: 6_144_000,
        status: "connected" as const,
      },
      {
        id: "12D3KooWPm4V...n8bR",
        address: "/ip4/192.168.1.42/tcp/9000",
        protocol: "/nous/1.0.0",
        latency_ms: 156,
        connected_at: new Date(now.getTime() - 3_600_000).toISOString(),
        bytes_sent: 2_048_000,
        bytes_recv: 1_536_000,
        status: "connecting" as const,
      },
    ];
  }, [health]);

  const overallStatus = health?.status === "ok" ? "operational" : error ? "offline" : "connecting";

  const stats = [
    {
      label: "Status",
      value: overallStatus === "operational" ? "Online" : overallStatus === "offline" ? "Offline" : "...",
      detail: health ? `v${health.version}` : "",
    },
    {
      label: "Peers",
      value: String(peers.filter((p) => p.status === "connected").length),
      detail: `${peers.length} total`,
    },
    {
      label: "Bandwidth",
      value: formatBytes(peers.reduce((acc, p) => acc + p.bytes_sent + p.bytes_recv, 0)),
      detail: "total transferred",
    },
    {
      label: "Latency",
      value: peers.length > 0
        ? `${Math.round(peers.reduce((acc, p) => acc + p.latency_ms, 0) / peers.length)}ms`
        : "—",
      detail: "avg peer latency",
    },
  ];

  return (
    <div className="p-8 max-w-6xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">Network</h1>
        <p className="text-sm text-neutral-500 font-light">
          P2P mesh status, connected peers, and subsystem health
        </p>
      </header>

      {/* Stats grid */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Overview
        </h2>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03]">
          {stats.map((stat) => (
            <Card key={stat.label} className="bg-black border-0 rounded-none p-6">
              <CardContent className="p-0">
                <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                  {stat.label}
                </p>
                <p className="text-2xl font-extralight mb-1">{stat.value}</p>
                <p className="text-xs text-neutral-600 font-light font-mono truncate">
                  {stat.detail}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* Subsystem health */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Subsystems
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-px bg-white/[0.03]">
          {SUBSYSTEMS.map((sub) => (
            <div
              key={sub.name}
              className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150"
            >
              <div className="flex items-center gap-3 min-w-0">
                <span className={cn("inline-block w-1.5 h-1.5 rounded-full shrink-0", statusColor(sub.status))} />
                <div className="min-w-0">
                  <p className="text-sm font-light">{sub.name}</p>
                  <p className="text-[10px] text-neutral-600 font-mono truncate">{sub.description}</p>
                </div>
              </div>
              <span className={cn(
                "text-[10px] font-mono uppercase tracking-wider shrink-0 ml-4",
                sub.status === "up" ? "text-emerald-600" : sub.status === "degraded" ? "text-yellow-600" : "text-red-500"
              )}>
                {statusLabel(sub.status)}
              </span>
            </div>
          ))}
        </div>
      </section>

      {/* Peer table */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Connected Peers
        </h2>
        {peers.length === 0 ? (
          <p className="text-sm text-neutral-700 font-light">
            {error ? "Node offline — no peer data" : "Discovering peers..."}
          </p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-white/[0.06]">
                  <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">Peer ID</th>
                  <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">Address</th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">Latency</th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">Sent</th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">Recv</th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3">Connected</th>
                </tr>
              </thead>
              <tbody>
                {peers.map((peer) => (
                  <tr
                    key={peer.id}
                    className="border-b border-white/[0.03] hover:bg-white/[0.01] transition-colors duration-100"
                  >
                    <td className="py-3 pr-6">
                      <div className="flex items-center gap-2">
                        <span className={cn("inline-block w-1.5 h-1.5 rounded-full", statusColor(peer.status))} />
                        <span className="text-xs font-mono text-neutral-400">{truncateId(peer.id)}</span>
                      </div>
                    </td>
                    <td className="py-3 pr-6">
                      <span className="text-xs font-mono text-neutral-500">{peer.address}</span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className={cn(
                        "text-xs font-mono",
                        peer.latency_ms < 50 ? "text-emerald-600" : peer.latency_ms < 100 ? "text-neutral-400" : "text-yellow-600"
                      )}>
                        {peer.latency_ms}ms
                      </span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className="text-xs font-mono text-neutral-500">{formatBytes(peer.bytes_sent)}</span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className="text-xs font-mono text-neutral-500">{formatBytes(peer.bytes_recv)}</span>
                    </td>
                    <td className="py-3 text-right">
                      <span className="text-xs font-mono text-neutral-600">{timeAgo(peer.connected_at)}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>

      {/* Protocol info */}
      {nodeInfo && (
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Protocol
          </h2>
          <div className="space-y-px">
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Protocol</span>
              <span className="text-sm font-mono text-neutral-300">{nodeInfo.protocol}/v{nodeInfo.version}</span>
            </div>
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Transport</span>
              <span className="text-sm font-mono text-neutral-300">TCP + Noise + Yamux</span>
            </div>
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Discovery</span>
              <span className="text-sm font-mono text-neutral-300">mDNS + Kademlia DHT</span>
            </div>
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Relay</span>
              <span className="text-sm font-mono text-neutral-300">DCUtR + Circuit v2</span>
            </div>
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Messaging</span>
              <span className="text-sm font-mono text-neutral-300">GossipSub v1.1</span>
            </div>
            <div className="flex items-center justify-between py-3 px-5 bg-white/[0.01]">
              <span className="text-sm font-light text-neutral-400">Active Features</span>
              <span className="text-sm font-mono text-neutral-300">{nodeInfo.features.join(", ")}</span>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
