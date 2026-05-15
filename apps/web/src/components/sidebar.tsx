"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  useSyncExternalStore,
} from "react";
import type { ReactNode } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import { useConnection } from "@/components/connection-status";
import { NotificationBell } from "@/components/notification-panel";
import { DidAvatar } from "@/components/did-avatar";
import {
  IconDashboard,
  IconSocial,
  IconMessages,
  IconWallet,
  IconMarketplace,
  IconGovernance,
  IconAi,
  IconFiles,
  IconNetwork,
  IconIdentity,
  IconSettings,
  IconSearch,
  IconMenu,
  IconClose,
  IconChevronDown,
} from "@/components/icons";

// Read user identity from localStorage for sidebar avatar
const noopSubscribe = () => () => {};
function getStoredDid(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_did");
}
function getStoredName(): string | null {
  if (typeof window === "undefined") return null;
  return localStorage.getItem("nous_display_name");
}
function useStoredIdentity() {
  const did = useSyncExternalStore(noopSubscribe, getStoredDid, () => null);
  const name = useSyncExternalStore(noopSubscribe, getStoredName, () => null);
  return { did, name };
}

// ── Nav badge store ──────────────────────────────────────────────────────
// External store for notification badges. Any component can write badge counts
// and the sidebar will reactively display them.

const navBadgeStore = {
  _badges: new Map<string, number>(),
  _listeners: new Set<() => void>(),
  _snapshot: new Map<string, number>(),

  set(href: string, count: number) {
    if (count <= 0) {
      this._badges.delete(href);
    } else {
      this._badges.set(href, count);
    }
    this._snapshot = new Map(this._badges);
    for (const l of this._listeners) l();
  },

  subscribe(listener: () => void) {
    navBadgeStore._listeners.add(listener);
    return () => { navBadgeStore._listeners.delete(listener); };
  },

  getSnapshot() {
    return navBadgeStore._snapshot;
  },
};

const emptyBadges = new Map<string, number>();

function useNavBadges() {
  return useSyncExternalStore(
    navBadgeStore.subscribe,
    navBadgeStore.getSnapshot,
    () => emptyBadges,
  );
}

/** Set a notification badge count for a sidebar nav item. Pass 0 to clear. */
export function setNavBadge(href: string, count: number) {
  navBadgeStore.set(href, count);
}

// Keyboard shortcut hints for each nav item (G + key)
const navShortcuts: Record<string, string> = {
  "/dashboard": "D",
  "/social": "S",
  "/messages": "M",
  "/wallet": "W",
  "/marketplace": "K",
  "/governance": "G",
  "/ai": "A",
  "/files": "F",
  "/network": "N",
  "/identity": "I",
  "/settings": "E",
};

// Mono lowercase route labels — the route IS the data, set in mono.
const sections = [
  {
    label: "Overview",
    items: [
      { name: "dashboard", href: "/dashboard", icon: IconDashboard },
    ],
  },
  {
    label: "Communication",
    items: [
      { name: "social", href: "/social", icon: IconSocial },
      { name: "messages", href: "/messages", icon: IconMessages },
    ],
  },
  {
    label: "Finance",
    items: [
      { name: "wallet", href: "/wallet", icon: IconWallet },
      { name: "marketplace", href: "/marketplace", icon: IconMarketplace },
      { name: "governance", href: "/governance", icon: IconGovernance },
    ],
  },
  {
    label: "Intelligence",
    items: [
      { name: "ai", href: "/ai", icon: IconAi },
      { name: "files", href: "/files", icon: IconFiles },
      { name: "network", href: "/network", icon: IconNetwork },
    ],
  },
  {
    label: "Account",
    items: [
      { name: "identity", href: "/identity", icon: IconIdentity },
      { name: "settings", href: "/settings", icon: IconSettings },
    ],
  },
];

// Bottom tab bar items — the 5 most important nav destinations
const bottomTabs = [
  { name: "home", href: "/dashboard", icon: IconDashboard },
  { name: "social", href: "/social", icon: IconSocial },
  { name: "messages", href: "/messages", icon: IconMessages },
  { name: "wallet", href: "/wallet", icon: IconWallet },
  { name: "ai", href: "/ai", icon: IconAi },
];

// --- Collapsible section state (persisted to localStorage) ---

const COLLAPSED_KEY = "nous_sidebar_collapsed";

function readCollapsed(): string[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(COLLAPSED_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return [];
}

function writeCollapsed(labels: string[]) {
  try {
    localStorage.setItem(COLLAPSED_KEY, JSON.stringify(labels));
  } catch {
    // ignore
  }
}

// --- Mobile sidebar context ---

type MobileSidebarContextValue = {
  open: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;
};

const MobileSidebarContext = createContext<MobileSidebarContextValue>({
  open: false,
  setOpen: () => {},
  toggle: () => {},
});

export function MobileSidebarProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const toggle = useCallback(() => setOpen((v) => !v), []);

  // Lock body scroll when drawer is open
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  return (
    <MobileSidebarContext value={{ open, setOpen, toggle }}>
      {children}
    </MobileSidebarContext>
  );
}

export function useMobileSidebar() {
  return useContext(MobileSidebarContext);
}

// --- Sidebar footer with identity avatar ---

function SidebarFooter({ status, onNavigate }: { status: string; onNavigate?: () => void }) {
  const { did, name } = useStoredIdentity();

  return (
    <div className="px-4 py-4 border-t border-rule space-y-3" data-tour="user">
      {did ? (
        <Link
          href="/identity"
          onClick={onNavigate}
          className="flex items-center gap-3 px-2 py-2 -mx-2 hover:bg-foreground/[0.02] transition-colors duration-[160ms] group"
        >
          <DidAvatar did={did} size={28} />
          <div className="flex-1 min-w-0">
            <p className="text-xs font-light text-foreground/80 group-hover:text-foreground transition-colors duration-[160ms] truncate">
              {name || "Anonymous"}
            </p>
            <p className="text-[10px] font-mono text-stone truncate">
              {did.slice(-12)}
            </p>
          </div>
          <span
            className={cn(
              "inline-block w-1.5 h-1.5 shrink-0",
              status === "online"
                ? "bg-sage"
                : status === "connecting"
                  ? "bg-clay"
                  : "bg-oxblood",
            )}
          />
        </Link>
      ) : (
        <div className="flex items-center gap-2 px-2">
          <span
            className={cn(
              "inline-block w-1.5 h-1.5",
              status === "online"
                ? "bg-sage"
                : status === "connecting"
                  ? "bg-clay"
                  : "bg-oxblood",
            )}
          />
          <p className="text-[10px] font-mono text-stone tracking-[0.15em] uppercase">
            {status === "online"
              ? "connected"
              : status === "connecting"
                ? "connecting"
                : "offline"}
          </p>
        </div>
      )}
      <div className="flex items-center justify-between px-2">
        <a
          href="https://github.com/teddytennant/nous/releases"
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] font-mono text-stone hover:text-foreground transition-colors duration-[160ms]"
        >
          v0.1.0
        </a>
        <a
          href="https://github.com/teddytennant/nous"
          target="_blank"
          rel="noopener noreferrer"
          className="text-[10px] font-mono text-stone hover:text-foreground transition-colors duration-[160ms]"
        >
          github
        </a>
      </div>
    </div>
  );
}

// --- Sidebar navigation content (shared between desktop and mobile) ---

function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();
  const { status } = useConnection();
  const badges = useNavBadges();
  const [collapsed, setCollapsed] = useState<Set<string>>(
    () => new Set(readCollapsed()),
  );

  // Auto-expand the section containing the active page on navigation
  useEffect(() => {
    const activeSection = sections.find((s) =>
      s.items.some((i) => pathname === i.href),
    );
    if (activeSection && collapsed.has(activeSection.label)) {
      setCollapsed((prev) => {
        const next = new Set(prev);
        next.delete(activeSection.label);
        writeCollapsed([...next]);
        return next;
      });
    }
    // intentionally exclude `collapsed` — only react to pathname changes
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pathname]);

  function toggleSection(label: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(label)) {
        next.delete(label);
      } else {
        next.add(label);
      }
      writeCollapsed([...next]);
      return next;
    });
  }

  return (
    <>
      <div className="px-5 pt-7 pb-6 flex items-center justify-between">
        <Link
          href="/"
          onClick={onNavigate}
          className="font-display text-[1.5rem] leading-none tracking-[-0.02em] hover:text-oxblood transition-colors duration-[160ms]"
        >
          Nous
        </Link>
        <NotificationBell />
      </div>

      <div className="px-3 mb-4" data-tour="search">
        <button
          type="button"
          onClick={() => {
            onNavigate?.();
            window.dispatchEvent(
              new KeyboardEvent("keydown", { key: "k", metaKey: true }),
            );
          }}
          className="w-full flex items-center gap-2.5 px-3 py-2 text-xs font-mono text-stone hover:text-foreground border border-rule hover:border-foreground/30 transition-colors duration-[160ms] cursor-pointer"
        >
          <IconSearch size={12} />
          <span className="flex-1 text-left">search</span>
          <kbd className="text-[10px] font-mono text-stone px-1.5 py-0.5 border border-rule">
            ⌘K
          </kbd>
        </button>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 space-y-3" data-tour="sidebar">
        {sections.map((section) => {
          const isCollapsed = collapsed.has(section.label);
          const hasActiveItem = section.items.some((i) => pathname === i.href);
          const sectionBadgeTotal = section.items.reduce(
            (sum, item) => sum + (badges.get(item.href) ?? 0),
            0,
          );

          return (
            <div key={section.label} className="border-t border-rule pt-3 first:border-t-0 first:pt-0">
              <button
                type="button"
                onClick={() => toggleSection(section.label)}
                className="w-full flex items-center justify-between px-2 py-1.5 group cursor-pointer"
                aria-expanded={!isCollapsed}
              >
                <span
                  className={cn(
                    "text-[10px] font-mono uppercase tracking-[0.2em] transition-colors duration-[160ms]",
                    hasActiveItem
                      ? "text-foreground/70"
                      : "text-stone group-hover:text-foreground/70",
                  )}
                >
                  {section.label}
                </span>
                <div className="flex items-center gap-1.5">
                  {isCollapsed && sectionBadgeTotal > 0 && (
                    <span className="font-mono text-[9px] text-oxblood leading-none">
                      {sectionBadgeTotal > 99 ? "99+" : sectionBadgeTotal}
                    </span>
                  )}
                  {isCollapsed && hasActiveItem && sectionBadgeTotal === 0 && (
                    <span className="w-1.5 h-1.5 bg-oxblood" />
                  )}
                  <span className="text-stone group-hover:text-foreground/70 transition-colors duration-[160ms]">
                    <IconChevronDown
                      size={10}
                      className={cn(
                        "transition-transform duration-[160ms]",
                        isCollapsed ? "-rotate-90" : "rotate-0",
                      )}
                    />
                  </span>
                </div>
              </button>

              <div
                className="sidebar-section-collapse"
                data-collapsed={isCollapsed}
              >
                <div className="overflow-hidden">
                  {section.items.map((item) => {
                    const active = pathname === item.href;
                    const Icon = item.icon;
                    const badgeCount = badges.get(item.href) ?? 0;
                    const shortcutKey = navShortcuts[item.href];
                    return (
                      <Link
                        key={item.href}
                        href={item.href}
                        onClick={onNavigate}
                        className={cn(
                          "group/nav relative flex items-center gap-3 pl-4 pr-2 py-1.5 text-[0.8125rem] font-mono lowercase transition-colors duration-[160ms]",
                          active
                            ? "text-oxblood"
                            : "text-foreground/60 hover:text-foreground",
                        )}
                      >
                        {/* 2px oxblood active rail — not a fill, not a pill. */}
                        {active && (
                          <span className="absolute left-0 top-1.5 bottom-1.5 w-[2px] bg-oxblood" />
                        )}
                        <span
                          className={cn(
                            "shrink-0",
                            active ? "text-oxblood" : "text-stone",
                          )}
                        >
                          <Icon size={14} />
                        </span>
                        <span className="flex-1">{item.name}</span>
                        {badgeCount > 0 ? (
                          <span className="font-mono text-[9px] text-oxblood">
                            {badgeCount > 99 ? "99+" : badgeCount}
                          </span>
                        ) : shortcutKey ? (
                          <kbd className="hidden group-hover/nav:inline text-[9px] font-mono text-stone tracking-wider">
                            G{shortcutKey}
                          </kbd>
                        ) : null}
                      </Link>
                    );
                  })}
                </div>
              </div>
            </div>
          );
        })}
      </nav>

      <SidebarFooter status={status} onNavigate={onNavigate} />
    </>
  );
}

// --- Desktop sidebar (hidden on mobile) ---

export function Sidebar() {
  return (
    <aside className="hidden md:flex w-52 shrink-0 border-r border-rule flex-col h-screen sticky top-0 bg-background">
      <SidebarContent />
    </aside>
  );
}

// --- Mobile header bar (visible on mobile only) ---

export function MobileHeader() {
  const { toggle } = useMobileSidebar();

  return (
    <header className="md:hidden fixed top-0 left-0 right-0 z-40 h-12 bg-background/95 backdrop-blur-md border-b border-rule flex items-center justify-between px-4">
      <div className="flex items-center">
        <button
          type="button"
          onClick={toggle}
          className="p-2 -ml-2 hover:text-oxblood transition-colors duration-[160ms]"
          aria-label="Toggle navigation"
        >
          <IconMenu size={18} />
        </button>
        <Link
          href="/"
          className="ml-3 font-display text-[1.125rem] leading-none tracking-[-0.02em]"
        >
          Nous
        </Link>
      </div>
      <NotificationBell />
    </header>
  );
}

// --- Mobile drawer overlay ---

export function MobileDrawer() {
  const { open, setOpen } = useMobileSidebar();
  const pathname = usePathname();

  // Close drawer on route change
  useEffect(() => {
    setOpen(false);
  }, [pathname, setOpen]);

  // Close on Escape
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && open) {
        setOpen(false);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, setOpen]);

  return (
    <div
      className={cn("md:hidden fixed inset-0 z-50", !open && "pointer-events-none")}
    >
      {/* Backdrop */}
      <div
        className={cn(
          "absolute inset-0 bg-background/60 backdrop-blur-sm transition-opacity duration-[160ms]",
          open ? "opacity-100" : "opacity-0",
        )}
        onClick={() => setOpen(false)}
        aria-hidden="true"
      />

      {/* Drawer panel */}
      <aside
        className={cn(
          "absolute top-0 left-0 bottom-0 w-64 bg-background border-r border-rule flex flex-col transition-transform duration-[160ms] ease-out",
          open ? "translate-x-0" : "-translate-x-full",
        )}
      >
        {/* Close button */}
        <div className="absolute top-3 right-3">
          <button
            type="button"
            onClick={() => setOpen(false)}
            className="p-1.5 hover:text-oxblood transition-colors duration-[160ms]"
            aria-label="Close navigation"
          >
            <IconClose size={14} />
          </button>
        </div>

        <SidebarContent onNavigate={() => setOpen(false)} />
      </aside>
    </div>
  );
}

// --- Mobile bottom tab bar ---

export function BottomTabBar() {
  const pathname = usePathname();
  const badges = useNavBadges();

  return (
    <nav
      className="md:hidden fixed bottom-0 left-0 right-0 z-40 bg-background border-t border-rule flex items-stretch px-1"
      style={{
        paddingBottom: "max(0.25rem, env(safe-area-inset-bottom))",
        height: "calc(3.75rem + env(safe-area-inset-bottom, 0px))",
      }}
    >
      {bottomTabs.map((tab, i) => {
        const active = pathname === tab.href;
        const Icon = tab.icon;
        const badgeCount = badges.get(tab.href) ?? 0;
        return (
          <Link
            key={tab.href}
            href={tab.href}
            className={cn(
              "relative flex-1 flex flex-col items-center justify-center gap-1 py-1.5 transition-colors duration-[160ms]",
              i > 0 && "border-l border-rule",
              active ? "text-oxblood" : "text-stone active:text-foreground",
            )}
          >
            {/* 2px oxblood top-rail mirrors the sidebar active treatment. */}
            {active && (
              <span className="absolute top-0 left-1/4 right-1/4 h-[2px] bg-oxblood" />
            )}
            <div className="relative">
              <Icon size={18} />
              {badgeCount > 0 && (
                <span className="absolute -top-1 -right-2 font-mono text-[8px] text-oxblood leading-none">
                  {badgeCount > 99 ? "99+" : badgeCount}
                </span>
              )}
            </div>
            <span className="text-[10px] font-mono lowercase tracking-wide">
              {tab.name}
            </span>
          </Link>
        );
      })}
    </nav>
  );
}
