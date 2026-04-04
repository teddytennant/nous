"use client";

import { useCallback, useEffect, useRef, useState, startTransition } from "react";
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
import { PageHeader } from "@/components/page-header";
import { useToast } from "@/components/toast";
import { Sparkline } from "@/components/sparkline";
import { PeerGraph } from "@/components/peer-graph";

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

// ── Latency history ────────────────────────────────────────────────────

/** Max samples per peer (~100s at 5s interval) */
const MAX_HISTORY = 20;

type LatencyHistoryMap = Record<string, number[]>;

function pushLatencySample(
  history: LatencyHistoryMap,
  peers: PeerResponse[],
): LatencyHistoryMap {
  const next = { ...history };
  for (const peer of peers) {
    const val = peer.latency_ms ?? 0;
    const prev = next[peer.peer_id] ?? [];
    const updated = prev.length >= MAX_HISTORY ? [...prev.slice(1), val] : [...prev, val];
    next[peer.peer_id] = updated;
  }
  return next;
}

/** Compute trend: compare avg of last 3 to avg of first 3. Lower is better for latency. */
function latencyTrend(samples: number[]): boolean | null {
  if (samples.length < 4) return null;
  const head = samples.slice(0, 3).reduce((s, v) => s + v, 0) / 3;
  const tail = samples.slice(-3).reduce((s, v) => s + v, 0) / 3;
  const diff = tail - head;
  if (Math.abs(diff) < 3) return null; // stable
  return diff < 0; // improving (lower) = positive trend
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

// ── Signal strength bars ──────────────────────────────────────────────

type SignalLevel = "excellent" | "good" | "fair" | "poor" | "unknown";

function getSignalLevel(latencyMs: number | null | undefined): SignalLevel {
  if (latencyMs == null) return "unknown";
  if (latencyMs < 30) return "excellent";
  if (latencyMs < 80) return "good";
  if (latencyMs < 150) return "fair";
  return "poor";
}

const signalConfig: Record<SignalLevel, { bars: number; color: string; label: string }> = {
  excellent: { bars: 4, color: "bg-emerald-500", label: "Excellent" },
  good: { bars: 3, color: "bg-emerald-500", label: "Good" },
  fair: { bars: 2, color: "bg-yellow-500", label: "Fair" },
  poor: { bars: 1, color: "bg-red-500", label: "Poor" },
  unknown: { bars: 0, color: "bg-neutral-700", label: "Unknown" },
};

function SignalStrength({ latencyMs }: { latencyMs: number | null | undefined }) {
  const level = getSignalLevel(latencyMs);
  const config = signalConfig[level];

  return (
    <div
      className="flex items-end gap-[2px] h-3 shrink-0"
      title={`${config.label}${latencyMs != null ? ` (${latencyMs}ms)` : ""}`}
    >
      {[1, 2, 3, 4].map((bar) => (
        <div
          key={bar}
          className={cn(
            "w-[3px] rounded-[0.5px] signal-bar",
            bar <= config.bars ? config.color : "bg-white/[0.08]",
          )}
          style={{ height: `${bar * 3}px` }}
        />
      ))}
    </div>
  );
}

type PeerSortKey = "peer_id" | "latency" | "sent" | "recv" | "connected";
type SortDir = "asc" | "desc";

function sortPeers(peers: PeerResponse[], key: PeerSortKey, dir: SortDir): PeerResponse[] {
  return [...peers].sort((a, b) => {
    let cmp = 0;
    switch (key) {
      case "peer_id":
        cmp = a.peer_id.localeCompare(b.peer_id);
        break;
      case "latency":
        cmp = (a.latency_ms ?? Infinity) - (b.latency_ms ?? Infinity);
        break;
      case "sent":
        cmp = a.bytes_sent - b.bytes_sent;
        break;
      case "recv":
        cmp = a.bytes_recv - b.bytes_recv;
        break;
      case "connected":
        cmp = new Date(a.connected_at).getTime() - new Date(b.connected_at).getTime();
        break;
    }
    return dir === "asc" ? cmp : -cmp;
  });
}

function SortIndicator({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) return <span className="text-neutral-800 ml-1">↕</span>;
  return <span className="text-[#d4af37] ml-1">{dir === "asc" ? "↑" : "↓"}</span>;
}

export default function NetworkPage() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);
  const [peerList, setPeerList] = useState<PeerResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [apiOffline, setApiOffline] = useState(false);
  const { toast } = useToast();
  const [connectAddr, setConnectAddr] = useState("");
  const [connecting, setConnecting] = useState(false);
  const [sortKey, setSortKey] = useState<PeerSortKey>("connected");
  const [sortDir, setSortDir] = useState<SortDir>("desc");
  const [latencyHistory, setLatencyHistory] = useState<LatencyHistoryMap>({});
  const [avgLatencyHistory, setAvgLatencyHistory] = useState<number[]>([]);
  const [peerCountHistory, setPeerCountHistory] = useState<number[]>([]);
  const [bandwidthHistory, setBandwidthHistory] = useState<number[]>([]);
  const prevBandwidthRef = useRef(0);

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
      setApiOffline(false);

      // Update latency history
      setLatencyHistory((prev) => pushLatencySample(prev, p.peers));

      // Update aggregate stats history
      const avg =
        p.peers.length > 0
          ? Math.round(p.peers.reduce((s, pr) => s + (pr.latency_ms ?? 0), 0) / p.peers.length)
          : 0;
      setAvgLatencyHistory((prev) =>
        prev.length >= MAX_HISTORY ? [...prev.slice(1), avg] : [...prev, avg],
      );
      setPeerCountHistory((prev) =>
        prev.length >= MAX_HISTORY
          ? [...prev.slice(1), p.peers.length]
          : [...prev, p.peers.length],
      );
      const totalBw = p.peers.reduce((s, pr) => s + pr.bytes_sent + pr.bytes_recv, 0);
      const bwDelta = Math.max(0, totalBw - prevBandwidthRef.current);
      prevBandwidthRef.current = totalBw;
      setBandwidthHistory((prev) =>
        prev.length >= MAX_HISTORY ? [...prev.slice(1), bwDelta] : [...prev, bwDelta],
      );
    } catch {
      setApiOffline(true);
      toast({ title: "API offline", variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [toast]);

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
      toast({ title: "Connection failed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setConnecting(false);
    }
  }

  async function handleDisconnect(peerId: string) {
    try {
      await peersApi.disconnect(peerId);
      await fetchData();
    } catch (e) {
      toast({ title: "Disconnect failed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  function toggleSort(key: PeerSortKey) {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir(key === "latency" ? "asc" : "desc");
    }
  }

  const sortedPeers = sortPeers(peerList, sortKey, sortDir);

  const overallStatus =
    health?.status === "ok"
      ? "operational"
      : apiOffline
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
      sparkData: null as number[] | null,
      sparkTrend: null as boolean | null,
    },
    {
      label: "Peers",
      value: String(peerList.length),
      detail: "connected",
      sparkData: peerCountHistory.length >= 2 ? peerCountHistory : null,
      sparkTrend: peerCountHistory.length >= 4
        ? (peerCountHistory[peerCountHistory.length - 1] >= peerCountHistory[0] ? true : false)
        : null,
    },
    {
      label: "Bandwidth",
      value: formatBytes(totalBandwidth),
      detail: "total transferred",
      sparkData: bandwidthHistory.length >= 2 ? bandwidthHistory : null,
      sparkTrend: true as boolean | null, // bandwidth throughput is always "positive"
    },
    {
      label: "Latency",
      value: peerList.length > 0 ? `${avgLatency}ms` : "\u2014",
      detail: "avg peer latency",
      sparkData: avgLatencyHistory.length >= 2 ? avgLatencyHistory : null,
      sparkTrend: latencyTrend(avgLatencyHistory),
    },
  ];

  return (
    <div className="p-4 sm:p-8 max-w-6xl">
      <PageHeader title="Network" subtitle="P2P mesh status, connected peers, and subsystem health" />

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
          <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03] stagger-in">
            {stats.map((stat) => (
              <Card
                key={stat.label}
                className="bg-black border-0 rounded-none p-6"
              >
                <CardContent className="p-0">
                  <div className="flex items-start justify-between mb-3">
                    <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600">
                      {stat.label}
                    </p>
                    {stat.sparkData && (
                      <Sparkline
                        data={stat.sparkData}
                        width={56}
                        height={18}
                        strokeWidth={1.2}
                        trend={stat.sparkTrend}
                      />
                    )}
                  </div>
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

      {/* Peer topology graph */}
      {!loading && peerList.length > 0 && (
        <section className="mb-16">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Topology
          </h2>
          <div className="border border-white/[0.04] rounded-sm bg-white/[0.01] overflow-hidden">
            <PeerGraph
              peers={peerList}
              latencyHistory={latencyHistory}
            />
          </div>
          <p className="text-[10px] font-mono text-neutral-700 mt-3 text-center">
            Hover peers to inspect. Distance from center reflects latency.
          </p>
        </section>
      )}

      {/* Subsystem health */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Subsystems
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-px bg-white/[0.03] stagger-in">
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
              className="bg-white/[0.02] text-[10px] font-mono px-3 py-1.5 outline-none placeholder:text-neutral-700 w-full sm:w-56"
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
            title={apiOffline ? "Node offline" : "No peers connected"}
            description={apiOffline ? "Start the Nous node to connect to the P2P mesh network." : "Enter a multiaddr above to connect to your first peer."}
          />
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-white/[0.06]">
                  <th
                    className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 cursor-pointer select-none hover:text-neutral-400 transition-colors duration-150"
                    onClick={() => toggleSort("peer_id")}
                  >
                    Peer ID
                    <SortIndicator active={sortKey === "peer_id"} dir={sortDir} />
                  </th>
                  <th className="text-left text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6">
                    Address
                  </th>
                  <th
                    className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 cursor-pointer select-none hover:text-neutral-400 transition-colors duration-150"
                    onClick={() => toggleSort("latency")}
                  >
                    Latency
                    <SortIndicator active={sortKey === "latency"} dir={sortDir} />
                  </th>
                  <th className="text-center text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 hidden lg:table-cell">
                    Trend
                  </th>
                  <th
                    className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 cursor-pointer select-none hover:text-neutral-400 transition-colors duration-150"
                    onClick={() => toggleSort("sent")}
                  >
                    Sent
                    <SortIndicator active={sortKey === "sent"} dir={sortDir} />
                  </th>
                  <th
                    className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 cursor-pointer select-none hover:text-neutral-400 transition-colors duration-150"
                    onClick={() => toggleSort("recv")}
                  >
                    Recv
                    <SortIndicator active={sortKey === "recv"} dir={sortDir} />
                  </th>
                  <th
                    className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3 pr-6 cursor-pointer select-none hover:text-neutral-400 transition-colors duration-150"
                    onClick={() => toggleSort("connected")}
                  >
                    Connected
                    <SortIndicator active={sortKey === "connected"} dir={sortDir} />
                  </th>
                  <th className="text-right text-[10px] font-mono uppercase tracking-wider text-neutral-600 pb-3" />
                </tr>
              </thead>
              <tbody>
                {sortedPeers.map((peer) => (
                  <tr
                    key={peer.peer_id}
                    className="border-b border-white/[0.03] hover:bg-white/[0.01] transition-colors duration-100 group"
                  >
                    <td className="py-3 pr-6">
                      <div className="flex items-center gap-2.5">
                        <SignalStrength latencyMs={peer.latency_ms} />
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
                    <td className="py-3 pr-6 hidden lg:table-cell">
                      {(latencyHistory[peer.peer_id]?.length ?? 0) >= 2 ? (
                        <div className="flex justify-center">
                          <Sparkline
                            data={latencyHistory[peer.peer_id]}
                            width={52}
                            height={18}
                            strokeWidth={1.2}
                            showDot={true}
                            trend={latencyTrend(latencyHistory[peer.peer_id])}
                          />
                        </div>
                      ) : (
                        <span className="text-[10px] text-neutral-700 font-mono block text-center">&mdash;</span>
                      )}
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
