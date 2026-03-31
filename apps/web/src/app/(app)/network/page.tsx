"use client";

import { useCallback, useEffect, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  node,
  peers as peersApi,
  type HealthResponse,
  type NodeInfo,
  type PeerResponse,
} from "@/lib/api";
import { EmptyState, NetworkIllustration } from "@/components/empty-state";

interface Subsystem {
  name: string;
  status: "up" | "degraded" | "down" | "unknown";
  description: string;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
  {
    name: "Networking",
    status: "up",
    description: "libp2p transport, gossipsub, relay",
  },
  {
    name: "Identity",
    status: "up",
    description: "DID resolver, credential store",
  },
  {
    name: "Messaging",
    status: "up",
    description: "E2E encryption, channel management",
  },
  {
    name: "Governance",
    status: "up",
    description: "DAO engine, vote tallying",
  },
  {
    name: "Storage",
    status: "up",
    description: "SQLite, CRDT replication",
  },
  {
    name: "Payments",
    status: "up",
    description: "Wallet, escrow, streaming",
  },
  {
    name: "Social",
    status: "up",
    description: "Feed, follow graph, Nostr relay",
  },
  {
    name: "AI",
    status: "up",
    description: "Inference, embeddings, HNSW index",
  },
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
  const [peerList, setPeerList] = useState<PeerResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [connectAddr, setConnectAddr] = useState("");
  const [connecting, setConnecting] = useState(false);

  const fetchData = useCallback(async () => {
    try {
      const [h, n, p] = await Promise.all([
        node.health(),
        node.info(),
        peersApi.list(),
      ]);
      setHealth(h);
      setNodeInfo(n);
      setPeerList(p.peers);
      setError(null);
    } catch {
      setError("API offline");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    startTransition(() => {
      fetchData();
    });
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [fetchData]);

  async function handleConnect() {
    if (!connectAddr.trim() || connecting) return;
    setConnecting(true);
    try {
      await peersApi.connect(connectAddr.trim());
      setConnectAddr("");
      await fetchData();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Connection failed");
    } finally {
      setConnecting(false);
    }
  }

  async function handleDisconnect(peerId: string) {
    try {
      await peersApi.disconnect(peerId);
      await fetchData();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Disconnect failed");
    }
  }

  const overallStatus =
    health?.status === "ok"
      ? "operational"
      : error
        ? "offline"
        : "connecting";

  const totalBandwidth = peerList.reduce(
    (acc, p) => acc + p.bytes_sent + p.bytes_recv,
    0,
  );
  const avgLatency =
    peerList.length > 0
      ? Math.round(
          peerList.reduce((acc, p) => acc + (p.latency_ms ?? 0), 0) /
            peerList.length,
        )
      : 0;

  const stats = [
    {
      label: "Status",
      value:
        overallStatus === "operational"
          ? "Online"
          : overallStatus === "offline"
            ? "Offline"
            : "...",
      detail: health ? `v${health.version}` : "",
    },
    {
      label: "Peers",
      value: String(peerList.length),
      detail: "connected",
    },
    {
      label: "Bandwidth",
      value: formatBytes(totalBandwidth),
      detail: "total transferred",
    },
    {
      label: "Latency",
      value: peerList.length > 0 ? `${avgLatency}ms` : "\u2014",
      detail: "avg peer latency",
    },
  ];

  return (
    <div className="p-8 max-w-6xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Network
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          P2P mesh status, connected peers, and subsystem health
        </p>
      </header>

      {error && (
        <div className="text-xs text-red-500/70 font-mono mb-6 px-1 flex items-center justify-between">
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            className="text-neutral-600 hover:text-white ml-4"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Stats grid */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Overview
        </h2>
        {loading ? (
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03]">
            {Array.from({ length: 4 }).map((_, i) => (
              <Card key={i} className="bg-black border-0 rounded-none p-6">
                <CardContent className="p-0">
                  <Skeleton className="h-2.5 w-16 mb-3" />
                  <Skeleton className="h-7 w-20 mb-1" />
                  <Skeleton className="h-3 w-24" />
                </CardContent>
              </Card>
            ))}
          </div>
        ) : (
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
                  <p className="text-2xl font-extralight mb-1">{stat.value}</p>
                  <p className="text-xs text-neutral-600 font-light font-mono truncate">
                    {stat.detail}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
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
                <span
                  className={cn(
                    "inline-block w-1.5 h-1.5 rounded-full shrink-0",
                    statusColor(sub.status),
                  )}
                />
                <div className="min-w-0">
                  <p className="text-sm font-light">{sub.name}</p>
                  <p className="text-[10px] text-neutral-600 font-mono truncate">
                    {sub.description}
                  </p>
                </div>
              </div>
              <span
                className={cn(
                  "text-[10px] font-mono uppercase tracking-wider shrink-0 ml-4",
                  sub.status === "up"
                    ? "text-emerald-600"
                    : sub.status === "degraded"
                      ? "text-yellow-600"
                      : "text-red-500",
                )}
              >
                {statusLabel(sub.status)}
              </span>
            </div>
          ))}
        </div>
      </section>

      {/* Peer table */}
      <section className="mb-16">
        <div className="flex items-center justify-between mb-8">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
            Connected Peers
          </h2>
          <div className="flex gap-2">
            <input
              value={connectAddr}
              onChange={(e) => setConnectAddr(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleConnect()}
              placeholder="/ip4/.../tcp/9000"
              className="bg-white/[0.02] text-[10px] font-mono px-3 py-1.5 outline-none placeholder:text-neutral-700 w-56"
            />
            <button
              onClick={handleConnect}
              disabled={connecting || !connectAddr.trim()}
              className="text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 border border-white/[0.06] text-neutral-600 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all disabled:opacity-30"
            >
              {connecting ? "..." : "Connect"}
            </button>
          </div>
        </div>

        {loading ? (
          <div className="space-y-3">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="flex items-center gap-6 py-3">
                <div className="flex items-center gap-2">
                  <Skeleton className="h-1.5 w-1.5 rounded-full" />
                  <Skeleton className="h-3 w-28" />
                </div>
                <Skeleton className="h-3 w-36" />
                <Skeleton className="h-3 w-12 ml-auto" />
                <Skeleton className="h-3 w-14" />
                <Skeleton className="h-3 w-14" />
                <Skeleton className="h-3 w-12" />
              </div>
            ))}
          </div>
        ) : peerList.length === 0 ? (
          <EmptyState
            icon={<NetworkIllustration />}
            title={error ? "Node offline" : "No peers connected"}
            description={error ? "Start the Nous node to connect to the P2P mesh network." : "Enter a multiaddr above to connect to your first peer."}
          />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-white/[0.06]">
                  <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Peer ID
                  </th>
                  <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Address
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Latency
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Sent
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Recv
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Connected
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3" />
                </tr>
              </thead>
              <tbody>
                {peerList.map((peer) => (
                  <tr
                    key={peer.peer_id}
                    className="border-b border-white/[0.03] hover:bg-white/[0.01] transition-colors duration-100 group"
                  >
                    <td className="py-3 pr-6">
                      <div className="flex items-center gap-2">
                        <span className="inline-block w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
                        <span className="text-xs font-mono text-neutral-400">
                          {truncateId(peer.peer_id)}
                        </span>
                      </div>
                    </td>
                    <td className="py-3 pr-6">
                      <span className="text-xs font-mono text-neutral-500">
                        {peer.multiaddr}
                      </span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span
                        className={cn(
                          "text-xs font-mono",
                          (peer.latency_ms ?? 0) < 50
                            ? "text-emerald-600"
                            : (peer.latency_ms ?? 0) < 100
                              ? "text-neutral-400"
                              : "text-yellow-600",
                        )}
                      >
                        {peer.latency_ms != null ? `${peer.latency_ms}ms` : "\u2014"}
                      </span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className="text-xs font-mono text-neutral-500">
                        {formatBytes(peer.bytes_sent)}
                      </span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className="text-xs font-mono text-neutral-500">
                        {formatBytes(peer.bytes_recv)}
                      </span>
                    </td>
                    <td className="py-3 pr-6 text-right">
                      <span className="text-xs font-mono text-neutral-600">
                        {timeAgo(peer.connected_at)}
                      </span>
                    </td>
                    <td className="py-3 text-right">
                      <button
                        onClick={() => handleDisconnect(peer.peer_id)}
                        className="text-[10px] font-mono text-neutral-800 hover:text-red-400 opacity-0 group-hover:opacity-100 transition-opacity"
                      >
                        disconnect
                      </button>
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
            {[
              ["Protocol", `${nodeInfo.protocol}/v${nodeInfo.version}`],
              ["Transport", "TCP + Noise + Yamux"],
              ["Discovery", "mDNS + Kademlia DHT"],
              ["Relay", "DCUtR + Circuit v2"],
              ["Messaging", "GossipSub v1.1"],
              ["Active Features", nodeInfo.features.join(", ")],
            ].map(([label, value]) => (
              <div
                key={label}
                className="flex items-center justify-between py-3 px-5 bg-white/[0.01]"
              >
                <span className="text-sm font-light text-neutral-400">
                  {label}
                </span>
                <span className="text-sm font-mono text-neutral-300">
                  {value}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
