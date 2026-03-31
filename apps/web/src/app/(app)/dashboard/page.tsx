"use client";

import { useState, useEffect, useRef, useSyncExternalStore, startTransition } from "react";
import Link from "next/link";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  node,
  governance,
  payments,
  type NodeInfo,
  type DaoListResponse,
  type WalletResponse,
} from "@/lib/api";
import { useConnection } from "@/components/connection-status";
import { SubsystemsWidget } from "@/components/subsystems";
import { ActivityTimeline } from "@/components/activity-timeline";
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

// ── Count-Up Animation ──────────────────────────────────────────────────

function useCountUp(target: number, duration = 800): string {
  const [display, setDisplay] = useState("0");
  const prevTarget = useRef(0);

  useEffect(() => {
    if (target === 0) {
      setDisplay("0");
      prevTarget.current = 0;
      return;
    }

    const start = prevTarget.current;
    const diff = target - start;
    const startTime = performance.now();

    function tick(now: number) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      // ease-out cubic
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = start + diff * eased;

      // Format: preserve decimal places matching target
      const decimals = target % 1 === 0 ? 0 : String(target).split(".")[1]?.length ?? 2;
      setDisplay(current.toFixed(decimals));

      if (progress < 1) {
        requestAnimationFrame(tick);
      } else {
        prevTarget.current = target;
      }
    }

    requestAnimationFrame(tick);
  }, [target, duration]);

  return display;
}

function CountUpBalance({ amount, token }: { amount: string; token: string }) {
  const numericValue = parseFloat(amount) || 0;
  const animated = useCountUp(numericValue);
  return (
    <span className="text-sm font-extralight tabular-nums text-neutral-300">
      {animated}
    </span>
  );
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


// ── Page ─────────────────────────────────────────────────────────────────

export default function DashboardPage() {
  const { status: apiStatus, health } = useConnection();
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);
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

      {/* Wallet Balance Strip */}
      <section className="mb-12">
        <div className="flex items-center justify-between mb-4">
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
        {(() => {
          const balances =
            wallet && wallet.balances.length > 0
              ? wallet.balances.slice(0, 4)
              : [
                  { token: "ETH", amount: "0" },
                  { token: "NOUS", amount: "0" },
                  { token: "USDC", amount: "0" },
                ];
          return (
            <Link
              href="/wallet"
              className="group flex items-center gap-0 border border-white/[0.06] rounded-sm overflow-hidden hover:border-white/10 transition-colors duration-200"
            >
              {balances.map((b, i) => (
                <div
                  key={b.token}
                  className={`flex-1 flex items-baseline gap-2 px-4 sm:px-5 py-3 ${
                    i > 0 ? "border-l border-white/[0.06]" : ""
                  } group-hover:bg-white/[0.01] transition-colors duration-200`}
                >
                  <span className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
                    {b.token}
                  </span>
                  <CountUpBalance amount={b.amount} token={b.token} />
                </div>
              ))}
              <div className="px-4 py-3 flex items-center">
                <ArrowRight className="w-3.5 h-3.5 text-neutral-800 group-hover:text-[#d4af37] group-hover:translate-x-0.5 transition-all duration-200" />
              </div>
            </Link>
          );
        })()}
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
        {/* Cross-subsystem Activity Timeline */}
        <section>
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              Activity Timeline
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
            <ActivityTimeline />
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
    </div>
  );
}
