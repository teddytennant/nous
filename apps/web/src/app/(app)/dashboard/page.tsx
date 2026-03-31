"use client";

import { useState, useEffect, useSyncExternalStore, startTransition } from "react";
import Link from "next/link";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  node,
  social,
  governance,
  payments,
  type NodeInfo,
  type FeedEvent,
  type DaoListResponse,
  type WalletResponse,
} from "@/lib/api";
import { useConnection } from "@/components/connection-status";
import { SubsystemsWidget } from "@/components/subsystems";
import {
  Users,
  MessageSquare,
  Store,
  Vote,
  Brain,
  FolderOpen,
  ArrowRight,
  Activity,
  Zap,
  Clock,
  Shield,
} from "lucide-react";

const emptySubscribe = () => () => {};

// ── Helpers ──────────────────────────────────────────────────────────────

function getGreeting(): string {
  const h = new Date().getHours();
  if (h < 5) return "Late night";
  if (h < 12) return "Good morning";
  if (h < 17) return "Good afternoon";
  if (h < 21) return "Good evening";
  return "Late night";
}

function getDisplayName(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_display_name");
}

function getDid(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_did");
}

function formatUptime(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return m > 0 ? `${h}h ${m}m` : `${h}h`;
}

function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.max(0, now - then);
  const s = Math.floor(diff / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.floor(h / 24);
  return `${d}d ago`;
}

function truncateDid(did: string): string {
  if (did.length <= 24) return did;
  return `${did.slice(0, 16)}...${did.slice(-6)}`;
}

// ── Quick Actions ────────────────────────────────────────────────────────

const quickActions = [
  {
    name: "Create Post",
    description: "Share with the network",
    href: "/social",
    icon: Users,
  },
  {
    name: "Send Message",
    description: "E2E encrypted chat",
    href: "/messages",
    icon: MessageSquare,
  },
  {
    name: "Browse Market",
    description: "Explore listings",
    href: "/marketplace",
    icon: Store,
  },
  {
    name: "Governance",
    description: "Vote on proposals",
    href: "/governance",
    icon: Vote,
  },
  {
    name: "AI Chat",
    description: "Talk to agents",
    href: "/ai",
    icon: Brain,
  },
  {
    name: "Upload File",
    description: "Encrypted storage",
    href: "/files",
    icon: FolderOpen,
  },
];

// ── Components ───────────────────────────────────────────────────────────

function QuickActionCard({
  action,
}: {
  action: (typeof quickActions)[number];
}) {
  const Icon = action.icon;
  return (
    <Link
      href={action.href}
      className="group flex items-center gap-4 p-4 border border-white/[0.06] rounded-sm hover:border-[#d4af37]/20 hover:bg-[#d4af37]/[0.02] transition-all duration-200 card-lift"
    >
      <div className="w-10 h-10 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center group-hover:border-[#d4af37]/20 group-hover:bg-[#d4af37]/[0.04] transition-colors duration-200 shrink-0">
        <Icon className="w-4 h-4 text-neutral-500 group-hover:text-[#d4af37] transition-colors duration-200" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-neutral-200 group-hover:text-white transition-colors duration-200">
          {action.name}
        </p>
        <p className="text-[11px] text-neutral-600 font-light">
          {action.description}
        </p>
      </div>
      <ArrowRight className="w-3.5 h-3.5 text-neutral-800 group-hover:text-[#d4af37] group-hover:translate-x-0.5 transition-all duration-200 shrink-0" />
    </Link>
  );
}

function FeedEventRow({ event }: { event: FeedEvent }) {
  return (
    <div className="flex items-start gap-3 py-3 px-4 hover:bg-white/[0.01] transition-colors duration-150">
      <div className="w-7 h-7 rounded-full bg-white/[0.04] border border-white/[0.06] flex items-center justify-center shrink-0 mt-0.5">
        <Users className="w-3 h-3 text-neutral-600" />
      </div>
      <div className="flex-1 min-w-0">
        <p className="text-xs text-neutral-500 font-mono mb-1">
          {truncateDid(event.pubkey)}
        </p>
        <p className="text-sm text-neutral-300 font-light leading-relaxed line-clamp-2">
          {event.content}
        </p>
      </div>
      <span className="text-[10px] font-mono text-neutral-700 shrink-0 mt-0.5">
        {timeAgo(event.created_at)}
      </span>
    </div>
  );
}

function FeedSkeleton() {
  return (
    <div className="space-y-0">
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="flex items-start gap-3 py-3 px-4">
          <Skeleton className="w-7 h-7 rounded-full shrink-0" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3 w-28" />
            <Skeleton className="h-4 w-full" />
          </div>
          <Skeleton className="h-3 w-12 shrink-0" />
        </div>
      ))}
    </div>
  );
}

// ── Page ─────────────────────────────────────────────────────────────────

export default function DashboardPage() {
  const { status: apiStatus, health } = useConnection();
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);
  const [feed, setFeed] = useState<FeedEvent[] | null>(null);
  const [daoData, setDaoData] = useState<DaoListResponse | null>(null);
  const [wallet, setWallet] = useState<WalletResponse | null>(null);
  const displayName = useSyncExternalStore(
    emptySubscribe,
    getDisplayName,
    () => null,
  );

  // Fetch node info
  useEffect(() => {
    let cancelled = false;
    async function fetch() {
      try {
        const n = await node.info();
        if (!cancelled) setNodeInfo(n);
      } catch {
        /* offline */
      }
    }
    fetch();
    const interval = setInterval(fetch, 30000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  // Fetch feed
  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const f = await social.feed({ limit: 5 });
        if (active) setFeed(f.events);
      } catch {
        /* offline */
      }
    }
    startTransition(() => {
      load();
    });
    return () => { active = false; };
  }, []);

  // Fetch DAOs
  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const d = await governance.listDaos();
        if (active) setDaoData(d);
      } catch {
        /* offline */
      }
    }
    startTransition(() => {
      load();
    });
    return () => { active = false; };
  }, []);

  // Fetch wallet
  useEffect(() => {
    const did = getDid();
    if (!did) return;
    let active = true;
    async function load() {
      try {
        const w = await payments.getWallet(did!);
        if (active) setWallet(w);
      } catch {
        /* no wallet yet */
      }
    }
    startTransition(() => {
      load();
    });
    return () => { active = false; };
  }, []);

  const stats = [
    {
      label: "Status",
      value:
        apiStatus === "online"
          ? "Online"
          : apiStatus === "offline"
            ? "Offline"
            : "...",
      detail:
        apiStatus === "online" && health
          ? `v${health.version}`
          : "connecting",
      icon: Zap,
    },
    {
      label: "Uptime",
      value: health ? formatUptime(health.uptime_ms) : "—",
      detail: "since last restart",
      icon: Clock,
    },
    {
      label: "DAOs",
      value: daoData ? String(daoData.count) : "—",
      detail: "active organizations",
      icon: Shield,
    },
    {
      label: "Features",
      value: nodeInfo ? String(nodeInfo.features.length) : "—",
      detail: "active modules",
      icon: Activity,
    },
  ];

  const greeting = getGreeting();
  const dateStr = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
  });

  return (
    <div className="p-6 sm:p-8 max-w-5xl">
      {/* Welcome */}
      <header className="mb-12">
        <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-600 mb-3">
          {dateStr}
        </p>
        <h1 className="text-3xl sm:text-4xl font-extralight tracking-[-0.03em] mb-2">
          {greeting}
          {displayName && (
            <span className="text-[#d4af37]">, {displayName}</span>
          )}
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          Your sovereign digital infrastructure, at a glance.
        </p>
      </header>

      {/* Stats */}
      <section className="mb-12">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03] rounded-sm overflow-hidden">
          {stats.map((stat) => {
            const Icon = stat.icon;
            return (
              <Card
                key={stat.label}
                className="bg-black border-0 rounded-none p-5 sm:p-6"
              >
                <CardContent className="p-0">
                  <div className="flex items-center gap-2 mb-3">
                    <Icon className="w-3 h-3 text-neutral-700" />
                    <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
                      {stat.label}
                    </p>
                  </div>
                  {stat.value === "—" || stat.value === "..." ? (
                    <>
                      <Skeleton className="h-7 w-16 mb-1" />
                      <Skeleton className="h-3 w-24" />
                    </>
                  ) : (
                    <>
                      <p className="text-2xl font-extralight mb-1 tabular-nums">
                        {stat.value}
                      </p>
                      <p className="text-[11px] text-neutral-600 font-light font-mono truncate">
                        {stat.detail}
                      </p>
                    </>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>
      </section>

      {/* Quick Actions */}
      <section className="mb-12">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-6">
          Quick Actions
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3 stagger-in">
          {quickActions.map((action) => (
            <QuickActionCard key={action.name} action={action} />
          ))}
        </div>
      </section>

      {/* Two-column: Activity Feed + Subsystems */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8 mb-12">
        {/* Activity Feed */}
        <section>
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              Recent Activity
            </h2>
            <Link
              href="/social"
              className="text-[10px] font-mono text-neutral-700 hover:text-[#d4af37] transition-colors duration-200 flex items-center gap-1"
            >
              View all
              <ArrowRight className="w-3 h-3" />
            </Link>
          </div>
          <div className="border border-white/[0.06] rounded-sm overflow-hidden">
            {feed === null ? (
              <FeedSkeleton />
            ) : feed.length === 0 ? (
              <div className="py-12 px-4 text-center">
                <Users className="w-6 h-6 text-neutral-800 mx-auto mb-3" />
                <p className="text-xs text-neutral-600 font-light">
                  No activity yet. Create your first post!
                </p>
                <Link
                  href="/social"
                  className="inline-flex items-center gap-1.5 mt-4 text-[11px] text-[#d4af37] font-medium hover:text-[#c4a030] transition-colors duration-200"
                >
                  Go to Social
                  <ArrowRight className="w-3 h-3" />
                </Link>
              </div>
            ) : (
              <div className="divide-y divide-white/[0.04]">
                {feed.map((event) => (
                  <FeedEventRow key={event.id} event={event} />
                ))}
              </div>
            )}
          </div>
        </section>

        {/* Subsystems */}
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-6">
            Subsystems
          </h2>
          <div className="border border-white/[0.06] rounded-sm overflow-hidden">
            <SubsystemsWidget />
          </div>
        </section>
      </div>

      {/* Wallet Summary (if wallet exists) */}
      {wallet && wallet.balances.length > 0 && (
        <section className="mb-12">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              Wallet
            </h2>
            <Link
              href="/wallet"
              className="text-[10px] font-mono text-neutral-700 hover:text-[#d4af37] transition-colors duration-200 flex items-center gap-1"
            >
              Manage
              <ArrowRight className="w-3 h-3" />
            </Link>
          </div>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-3">
            {wallet.balances.slice(0, 4).map((b) => (
              <div
                key={b.token}
                className="p-4 border border-white/[0.06] rounded-sm"
              >
                <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-2">
                  {b.token}
                </p>
                <p className="text-lg font-extralight tabular-nums">
                  {b.amount}
                </p>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
