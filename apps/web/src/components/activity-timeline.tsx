"use client";

import { useState, useEffect, useSyncExternalStore } from "react";
import Link from "next/link";
import {
  Users,
  Vote,
  Wallet,
  Store,
  MessageSquare,
  ArrowRight,
  type LucideIcon,
} from "lucide-react";
import { Skeleton } from "@/components/ui/skeleton";
import { Avatar } from "@/components/avatar";
import {
  social,
  governance,
  payments,
  marketplace,
  messaging,
  type FeedEvent,
  type ProposalResponse,
  type TransactionResponse,
  type ListingResponse,
  type ChannelResponse,
} from "@/lib/api";

// ── Timeline event type ────────────────────────────────────────────────

type EventKind = "social" | "governance" | "payment" | "marketplace" | "message";

interface TimelineEvent {
  id: string;
  kind: EventKind;
  icon: LucideIcon;
  actor: string;
  title: string;
  detail?: string;
  href: string;
  timestamp: Date;
}

const kindMeta: Record<EventKind, { label: string; color: string }> = {
  social: { label: "Social", color: "text-blue-500" },
  governance: { label: "Governance", color: "text-[#d4af37]" },
  payment: { label: "Payment", color: "text-emerald-500" },
  marketplace: { label: "Market", color: "text-purple-400" },
  message: { label: "Message", color: "text-cyan-400" },
};

// ── Normalizers ────────────────────────────────────────────────────────

function fromFeedEvent(e: FeedEvent): TimelineEvent {
  return {
    id: `social-${e.id}`,
    kind: "social",
    icon: Users,
    actor: e.pubkey,
    title: "New post",
    detail:
      e.content.length > 80 ? `${e.content.slice(0, 80)}…` : e.content,
    href: "/social",
    timestamp: new Date(e.created_at),
  };
}

function fromProposal(p: ProposalResponse): TimelineEvent {
  return {
    id: `gov-${p.id}`,
    kind: "governance",
    icon: Vote,
    actor: p.proposer_did,
    title: `Proposal: ${p.title}`,
    detail: p.status === "Active" ? "Open for voting" : p.status,
    href: "/governance",
    timestamp: new Date(p.created_at),
  };
}

function fromTransaction(t: TransactionResponse): TimelineEvent {
  return {
    id: `pay-${t.id}`,
    kind: "payment",
    icon: Wallet,
    actor: t.from_did,
    title: `Sent ${t.amount} ${t.token}`,
    detail: `to ${t.to_did.length > 20 ? `${t.to_did.slice(0, 12)}…${t.to_did.slice(-6)}` : t.to_did}`,
    href: "/wallet",
    timestamp: new Date(t.timestamp),
  };
}

function fromListing(l: ListingResponse): TimelineEvent {
  return {
    id: `mkt-${l.id}`,
    kind: "marketplace",
    icon: Store,
    actor: l.seller_did,
    title: `Listed: ${l.title}`,
    detail: `${l.price_amount} ${l.price_token}`,
    href: "/marketplace",
    timestamp: new Date(l.created_at),
  };
}

function fromChannel(c: ChannelResponse): TimelineEvent {
  return {
    id: `msg-${c.id}`,
    kind: "message",
    icon: MessageSquare,
    actor: c.members[0] ?? "unknown",
    title:
      c.kind === "group"
        ? `Group created: ${c.name ?? "Unnamed"}`
        : "New conversation",
    detail: `${c.members.length} member${c.members.length !== 1 ? "s" : ""}`,
    href: "/messages",
    timestamp: new Date(c.created_at),
  };
}

// ── Helpers ─────────────────────────────────────────────────────────────

function truncateDid(did: string): string {
  if (did.length <= 20) return did;
  return `${did.slice(0, 12)}…${did.slice(-6)}`;
}

function timeAgo(date: Date): string {
  const diff = Math.max(0, Date.now() - date.getTime());
  const s = Math.floor(diff / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.floor(h / 24);
  return `${d}d ago`;
}

const emptySubscribe = () => () => {};

function getDid(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_did");
}

// ── Component ──────────────────────────────────────────────────────────

export function ActivityTimeline() {
  const [events, setEvents] = useState<TimelineEvent[] | null>(null);
  const userDid = useSyncExternalStore(emptySubscribe, getDid, () => null);

  useEffect(() => {
    let cancelled = false;

    async function fetchAll() {
      const results: TimelineEvent[] = [];

      // Fetch from all subsystems in parallel — each one is best-effort
      const [feedRes, proposalRes, txRes, listingRes, channelRes] =
        await Promise.allSettled([
          social.feed({ limit: 5 }),
          governance.listProposals(),
          userDid
            ? payments.getTransactions(userDid, 5)
            : Promise.resolve([] as TransactionResponse[]),
          marketplace.search({ limit: 5 }),
          userDid
            ? messaging.listChannels(userDid)
            : Promise.resolve([] as ChannelResponse[]),
        ]);

      if (feedRes.status === "fulfilled") {
        results.push(...feedRes.value.events.map(fromFeedEvent));
      }
      if (proposalRes.status === "fulfilled") {
        results.push(
          ...proposalRes.value.proposals.slice(0, 5).map(fromProposal)
        );
      }
      if (txRes.status === "fulfilled") {
        const txs = Array.isArray(txRes.value) ? txRes.value : [];
        results.push(...txs.slice(0, 5).map(fromTransaction));
      }
      if (listingRes.status === "fulfilled") {
        results.push(...listingRes.value.listings.map(fromListing));
      }
      if (channelRes.status === "fulfilled") {
        const channels = Array.isArray(channelRes.value)
          ? channelRes.value
          : [];
        results.push(...channels.slice(0, 3).map(fromChannel));
      }

      // Sort by most recent first, take top 8
      results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());

      if (!cancelled) {
        setEvents(results.slice(0, 8));
      }
    }

    fetchAll();
    return () => {
      cancelled = true;
    };
  }, [userDid]);

  if (events === null) {
    return <TimelineSkeleton />;
  }

  if (events.length === 0) {
    return (
      <div className="py-12 px-4 text-center">
        <Users className="w-6 h-6 text-neutral-800 mx-auto mb-3" />
        <p className="text-xs text-neutral-600 font-light">
          No activity yet. Start using Nous to see events here.
        </p>
        <Link
          href="/social"
          className="inline-flex items-center gap-1.5 mt-4 text-[11px] text-[#d4af37] font-medium hover:text-[#c4a030] transition-colors duration-200"
        >
          Create a post
          <ArrowRight className="w-3 h-3" />
        </Link>
      </div>
    );
  }

  return (
    <div className="divide-y divide-white/[0.04]">
      {events.map((event) => (
        <TimelineRow key={event.id} event={event} />
      ))}
    </div>
  );
}

// ── Row ──────────────────────────────────────────────────────────────────

function TimelineRow({ event }: { event: TimelineEvent }) {
  const meta = kindMeta[event.kind];
  const Icon = event.icon;

  return (
    <Link
      href={event.href}
      className="flex items-start gap-3 py-3 px-4 hover:bg-white/[0.01] transition-colors duration-150 group"
    >
      {/* Avatar with subsystem icon overlay */}
      <div className="relative shrink-0 mt-0.5">
        <Avatar did={event.actor} size="sm" />
        <div
          className={`absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-black border border-white/[0.08] flex items-center justify-center`}
        >
          <Icon className={`w-2 h-2 ${meta.color}`} />
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <span className="text-[10px] font-mono text-neutral-600 truncate">
            {truncateDid(event.actor)}
          </span>
          <span
            className={`text-[9px] font-mono uppercase tracking-wider ${meta.color} opacity-70`}
          >
            {meta.label}
          </span>
        </div>
        <p className="text-sm text-neutral-300 font-light leading-relaxed group-hover:text-white transition-colors duration-150 truncate">
          {event.title}
        </p>
        {event.detail && (
          <p className="text-xs text-neutral-600 font-light mt-0.5 truncate">
            {event.detail}
          </p>
        )}
      </div>

      {/* Timestamp */}
      <span className="text-[10px] font-mono text-neutral-700 shrink-0 mt-0.5">
        {timeAgo(event.timestamp)}
      </span>
    </Link>
  );
}

// ── Skeleton ────────────────────────────────────────────────────────────

function TimelineSkeleton() {
  return (
    <div className="space-y-0">
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="flex items-start gap-3 py-3 px-4">
          <Skeleton className="w-7 h-7 rounded-full shrink-0" />
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-2">
              <Skeleton className="h-2.5 w-24" />
              <Skeleton className="h-2.5 w-12" />
            </div>
            <Skeleton className="h-3.5 w-full max-w-[260px]" />
            <Skeleton className="h-3 w-40" />
          </div>
          <Skeleton className="h-2.5 w-12 shrink-0" />
        </div>
      ))}
    </div>
  );
}
