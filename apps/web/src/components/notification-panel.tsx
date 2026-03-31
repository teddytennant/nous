"use client";

import {
  useState,
  useEffect,
  useCallback,
  useRef,
  useSyncExternalStore,
} from "react";
import Link from "next/link";
import { cn } from "@/lib/utils";
import {
  Bell,
  Users,
  Vote,
  Wallet,
  Store,
  MessageSquare,
  Check,
  X,
  type LucideIcon,
} from "lucide-react";
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

// ── Types ──────────────────────────────────────────────────────────────

type NotifKind = "social" | "governance" | "payment" | "marketplace" | "message";

interface Notification {
  id: string;
  kind: NotifKind;
  icon: LucideIcon;
  actor: string;
  title: string;
  detail?: string;
  href: string;
  timestamp: Date;
}

const kindMeta: Record<NotifKind, { label: string; color: string }> = {
  social: { label: "Social", color: "text-blue-500" },
  governance: { label: "Governance", color: "text-[#d4af37]" },
  payment: { label: "Payment", color: "text-emerald-500" },
  marketplace: { label: "Market", color: "text-purple-400" },
  message: { label: "Message", color: "text-cyan-400" },
};

// ── Normalizers ────────────────────────────────────────────────────────

function fromFeedEvent(e: FeedEvent): Notification {
  return {
    id: `social-${e.id}`,
    kind: "social",
    icon: Users,
    actor: e.pubkey,
    title: "New post",
    detail: e.content.length > 60 ? `${e.content.slice(0, 60)}...` : e.content,
    href: "/social",
    timestamp: new Date(e.created_at),
  };
}

function fromProposal(p: ProposalResponse): Notification {
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

function fromTransaction(t: TransactionResponse): Notification {
  return {
    id: `pay-${t.id}`,
    kind: "payment",
    icon: Wallet,
    actor: t.from_did,
    title: `Received ${t.amount} ${t.token}`,
    detail: `from ${truncateDid(t.from_did)}`,
    href: "/wallet",
    timestamp: new Date(t.timestamp),
  };
}

function fromListing(l: ListingResponse): Notification {
  return {
    id: `mkt-${l.id}`,
    kind: "marketplace",
    icon: Store,
    actor: l.seller_did,
    title: `New listing: ${l.title}`,
    detail: `${l.price_amount} ${l.price_token}`,
    href: "/marketplace",
    timestamp: new Date(l.created_at),
  };
}

function fromChannel(c: ChannelResponse): Notification {
  return {
    id: `msg-${c.id}`,
    kind: "message",
    icon: MessageSquare,
    actor: c.members[0] ?? "unknown",
    title:
      c.kind === "group"
        ? `New group: ${c.name ?? "Unnamed"}`
        : "New conversation",
    detail: `${c.members.length} member${c.members.length !== 1 ? "s" : ""}`,
    href: "/messages",
    timestamp: new Date(c.created_at),
  };
}

// ── Helpers ────────────────────────────────────────────────────────────

function truncateDid(did: string): string {
  if (did.length <= 20) return did;
  return `${did.slice(0, 12)}...${did.slice(-6)}`;
}

function timeAgo(date: Date): string {
  const diff = Math.max(0, Date.now() - date.getTime());
  const s = Math.floor(diff / 1000);
  if (s < 60) return "just now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  const d = Math.floor(h / 24);
  return `${d}d`;
}

const STORAGE_KEY = "nous_read_notifications";

function getReadIds(): Set<string> {
  if (typeof window === "undefined") return new Set();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return new Set(JSON.parse(raw));
  } catch {
    // ignore
  }
  return new Set();
}

function persistReadIds(ids: Set<string>) {
  try {
    // Keep max 200 entries to avoid bloat
    const arr = [...ids].slice(-200);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(arr));
  } catch {
    // ignore
  }
}

const emptySubscribe = () => () => {};

function getDid(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_did");
}

// ── Notification Bell ──────────────────────────────────────────────────

export function NotificationBell() {
  const [open, setOpen] = useState(false);
  const [notifications, setNotifications] = useState<Notification[] | null>(
    null,
  );
  const [readIds, setReadIds] = useState<Set<string>>(() => getReadIds());
  const panelRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const userDid = useSyncExternalStore(emptySubscribe, getDid, () => null);

  // Fetch notifications
  const fetchNotifications = useCallback(async () => {
    const results: Notification[] = [];

    const [feedRes, proposalRes, txRes, listingRes, channelRes] =
      await Promise.allSettled([
        social.feed({ limit: 5 }),
        governance.listProposals(),
        userDid
          ? payments.getTransactions(userDid, 5)
          : Promise.resolve([] as TransactionResponse[]),
        marketplace.search({ limit: 3 }),
        userDid
          ? messaging.listChannels(userDid)
          : Promise.resolve([] as ChannelResponse[]),
      ]);

    if (feedRes.status === "fulfilled") {
      results.push(...feedRes.value.events.map(fromFeedEvent));
    }
    if (proposalRes.status === "fulfilled") {
      results.push(
        ...proposalRes.value.proposals.slice(0, 5).map(fromProposal),
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
      const channels = Array.isArray(channelRes.value) ? channelRes.value : [];
      results.push(...channels.slice(0, 3).map(fromChannel));
    }

    results.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
    setNotifications(results.slice(0, 12));
  }, [userDid]);

  // Initial fetch + polling
  useEffect(() => {
    fetchNotifications();
    const interval = setInterval(fetchNotifications, 30000);
    return () => clearInterval(interval);
  }, [fetchNotifications]);

  // Close on Escape
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && open) {
        setOpen(false);
        buttonRef.current?.focus();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open]);

  // Close on click outside
  useEffect(() => {
    if (!open) return;
    function onClick(e: MouseEvent) {
      if (
        panelRef.current &&
        !panelRef.current.contains(e.target as Node) &&
        buttonRef.current &&
        !buttonRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", onClick);
    return () => document.removeEventListener("mousedown", onClick);
  }, [open]);

  const unreadCount = notifications
    ? notifications.filter((n) => !readIds.has(n.id)).length
    : 0;

  function markAsRead(id: string) {
    setReadIds((prev) => {
      const next = new Set(prev);
      next.add(id);
      persistReadIds(next);
      return next;
    });
  }

  function markAllAsRead() {
    if (!notifications) return;
    setReadIds((prev) => {
      const next = new Set(prev);
      for (const n of notifications) {
        next.add(n.id);
      }
      persistReadIds(next);
      return next;
    });
  }

  return (
    <div className="relative">
      {/* Bell button */}
      <button
        ref={buttonRef}
        type="button"
        onClick={() => setOpen((v) => !v)}
        className={cn(
          "relative p-2 rounded-sm transition-colors duration-150",
          open
            ? "bg-white/[0.06] text-white"
            : "text-neutral-600 hover:text-neutral-400 hover:bg-white/[0.04]",
        )}
        aria-label={`Notifications${unreadCount > 0 ? ` (${unreadCount} unread)` : ""}`}
        aria-expanded={open}
      >
        <Bell className="w-4 h-4" />
        {unreadCount > 0 && (
          <span className="absolute -top-0.5 -right-0.5 min-w-[16px] h-4 px-1 rounded-full bg-[#d4af37] text-black text-[9px] font-mono font-bold flex items-center justify-center notif-badge-enter">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        )}
      </button>

      {/* Panel */}
      {open && (
        <div
          ref={panelRef}
          className="absolute top-full left-0 mt-2 w-80 bg-black border border-white/[0.08] rounded-md shadow-2xl z-50 notif-panel-enter overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-white/[0.06]">
            <h3 className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-500">
              Notifications
            </h3>
            <div className="flex items-center gap-1">
              {unreadCount > 0 && (
                <button
                  type="button"
                  onClick={markAllAsRead}
                  className="text-[10px] font-mono text-neutral-600 hover:text-[#d4af37] transition-colors duration-150 px-2 py-1 rounded-sm hover:bg-white/[0.04]"
                >
                  Mark all read
                </button>
              )}
              <button
                type="button"
                onClick={() => setOpen(false)}
                className="p-1 rounded-sm text-neutral-700 hover:text-neutral-400 hover:bg-white/[0.04] transition-colors duration-150"
                aria-label="Close notifications"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {/* Body */}
          <div className="max-h-96 overflow-y-auto terminal-scroll">
            {notifications === null ? (
              <NotificationSkeleton />
            ) : notifications.length === 0 ? (
              <div className="py-12 px-4 text-center">
                <Bell className="w-5 h-5 text-neutral-800 mx-auto mb-3" />
                <p className="text-xs text-neutral-600 font-light">
                  No notifications yet
                </p>
              </div>
            ) : (
              <div className="divide-y divide-white/[0.04]">
                {notifications.map((notif) => (
                  <NotificationRow
                    key={notif.id}
                    notification={notif}
                    isRead={readIds.has(notif.id)}
                    onRead={() => markAsRead(notif.id)}
                    onClose={() => setOpen(false)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Row ────────────────────────────────────────────────────────────────

function NotificationRow({
  notification,
  isRead,
  onRead,
  onClose,
}: {
  notification: Notification;
  isRead: boolean;
  onRead: () => void;
  onClose: () => void;
}) {
  const meta = kindMeta[notification.kind];
  const Icon = notification.icon;

  return (
    <Link
      href={notification.href}
      onClick={() => {
        onRead();
        onClose();
      }}
      className={cn(
        "flex items-start gap-3 py-3 px-4 transition-colors duration-150 group",
        isRead
          ? "opacity-50 hover:opacity-75"
          : "hover:bg-white/[0.02]",
      )}
    >
      {/* Avatar with subsystem icon */}
      <div className="relative shrink-0 mt-0.5">
        <Avatar did={notification.actor} size="sm" />
        <div className="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-black border border-white/[0.08] flex items-center justify-center">
          <Icon className={`w-2 h-2 ${meta.color}`} />
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-0.5">
          <span
            className={`text-[9px] font-mono uppercase tracking-wider ${meta.color} opacity-70`}
          >
            {meta.label}
          </span>
          <span className="text-[10px] font-mono text-neutral-700">
            {timeAgo(notification.timestamp)}
          </span>
        </div>
        <p className="text-[13px] text-neutral-300 font-light leading-snug group-hover:text-white transition-colors duration-150 truncate">
          {notification.title}
        </p>
        {notification.detail && (
          <p className="text-[11px] text-neutral-600 font-light mt-0.5 truncate">
            {notification.detail}
          </p>
        )}
      </div>

      {/* Unread dot */}
      {!isRead && (
        <span className="w-2 h-2 rounded-full bg-[#d4af37] shrink-0 mt-2" />
      )}
    </Link>
  );
}

// ── Skeleton ──────────────────────────────────────────────────────────

function NotificationSkeleton() {
  return (
    <div>
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="flex items-start gap-3 py-3 px-4">
          <div className="w-7 h-7 rounded-full bg-white/[0.04] shrink-0 animate-pulse" />
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-2">
              <div className="h-2.5 w-14 bg-white/[0.04] rounded animate-pulse" />
              <div className="h-2.5 w-8 bg-white/[0.04] rounded animate-pulse" />
            </div>
            <div className="h-3.5 w-full max-w-[200px] bg-white/[0.04] rounded animate-pulse" />
          </div>
        </div>
      ))}
    </div>
  );
}
