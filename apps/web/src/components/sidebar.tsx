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
  LayoutDashboard,
  Users,
  MessageSquare,
  Wallet,
  Store,
  Vote,
  Brain,
  FolderOpen,
  Globe,
  Fingerprint,
  Settings,
  Search,
  Menu,
  X,
  ChevronDown,
} from "lucide-react";

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

const sections = [
  {
    label: "Overview",
    items: [
      { name: "Dashboard", href: "/dashboard", icon: LayoutDashboard },
    ],
  },
  {
    label: "Communication",
    items: [
      { name: "Social", href: "/social", icon: Users },
      { name: "Messages", href: "/messages", icon: MessageSquare },
    ],
  },
  {
    label: "Finance",
    items: [
      { name: "Wallet", href: "/wallet", icon: Wallet },
      { name: "Marketplace", href: "/marketplace", icon: Store },
      { name: "Governance", href: "/governance", icon: Vote },
    ],
  },
  {
    label: "Intelligence",
    items: [
      { name: "AI", href: "/ai", icon: Brain },
      { name: "Files", href: "/files", icon: FolderOpen },
      { name: "Network", href: "/network", icon: Globe },
    ],
  },
  {
    label: "Account",
    items: [
      { name: "Identity", href: "/identity", icon: Fingerprint },
      { name: "Settings", href: "/settings", icon: Settings },
    ],
  },
];

// Bottom tab bar items — the 5 most important nav destinations
const bottomTabs = [
  { name: "Home", href: "/dashboard", icon: LayoutDashboard },
  { name: "Social", href: "/social", icon: Users },
  { name: "Messages", href: "/messages", icon: MessageSquare },
  { name: "Wallet", href: "/wallet", icon: Wallet },
  { name: "AI", href: "/ai", icon: Brain },
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
    <div className="px-4 py-4 border-t border-white/[0.04]" data-tour="user">
      {did ? (
        <Link
          href="/identity"
          onClick={onNavigate}
          className="flex items-center gap-3 px-2 py-2 -mx-2 rounded-sm hover:bg-white/[0.02] transition-colors duration-150 group"
        >
          <DidAvatar did={did} size={28} />
          <div className="flex-1 min-w-0">
            <p className="text-xs font-light text-neutral-400 group-hover:text-white transition-colors duration-150 truncate">
              {name || "Anonymous"}
            </p>
            <p className="text-[10px] font-mono text-neutral-700 truncate">
              {did.slice(-12)}
            </p>
          </div>
          <span
            className={cn(
              "inline-block w-1.5 h-1.5 rounded-full shrink-0",
              status === "online"
                ? "bg-emerald-500"
                : status === "connecting"
                  ? "bg-yellow-500 animate-pulse"
                  : "bg-red-500",
            )}
          />
        </Link>
      ) : (
        <div className="flex items-center gap-2 px-2">
          <span
            className={cn(
              "inline-block w-1.5 h-1.5 rounded-full",
              status === "online"
                ? "bg-emerald-500"
                : status === "connecting"
                  ? "bg-yellow-500 animate-pulse"
                  : "bg-red-500",
            )}
          />
          <p className="text-[10px] font-mono text-neutral-700 tracking-wider uppercase">
            {status === "online"
              ? "connected"
              : status === "connecting"
                ? "connecting"
                : "offline"}
          </p>
        </div>
      )}
    </div>
  );
}

// --- Sidebar navigation content (shared between desktop and mobile) ---

function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();
  const { status } = useConnection();
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
      <div className="px-6 pt-8 pb-8 flex items-center justify-between">
        <Link
          href="/"
          onClick={onNavigate}
          className="text-2xl font-extralight tracking-[-0.04em] hover:text-[#d4af37] transition-colors duration-200"
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
          className="w-full flex items-center gap-2.5 px-3 py-2 text-sm font-light text-neutral-600 hover:text-neutral-400 bg-white/[0.02] hover:bg-white/[0.04] border border-white/[0.06] rounded-sm transition-all duration-150 cursor-pointer"
        >
          <Search className="w-3.5 h-3.5" />
          <span className="flex-1 text-left">Search...</span>
          <kbd className="text-[10px] font-mono text-neutral-700 bg-white/[0.04] border border-white/[0.06] px-1.5 py-0.5 rounded">
            ⌘K
          </kbd>
        </button>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 space-y-1" data-tour="sidebar">
        {sections.map((section) => {
          const isCollapsed = collapsed.has(section.label);
          const hasActiveItem = section.items.some((i) => pathname === i.href);

          return (
            <div key={section.label}>
              <button
                type="button"
                onClick={() => toggleSection(section.label)}
                className="w-full flex items-center justify-between px-3 py-2 rounded-sm group cursor-pointer hover:bg-white/[0.02] transition-colors duration-150"
                aria-expanded={!isCollapsed}
              >
                <span
                  className={cn(
                    "text-[10px] font-mono uppercase tracking-[0.2em] transition-colors duration-150",
                    hasActiveItem
                      ? "text-neutral-500"
                      : "text-neutral-700 group-hover:text-neutral-500",
                  )}
                >
                  {section.label}
                </span>
                <div className="flex items-center gap-1.5">
                  {isCollapsed && hasActiveItem && (
                    <span className="w-1.5 h-1.5 rounded-full bg-[#d4af37]" />
                  )}
                  <ChevronDown
                    className={cn(
                      "w-3 h-3 transition-all duration-200",
                      isCollapsed ? "-rotate-90" : "rotate-0",
                      "text-neutral-700 group-hover:text-neutral-500",
                    )}
                  />
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
                    return (
                      <Link
                        key={item.href}
                        href={item.href}
                        onClick={onNavigate}
                        className={cn(
                          "relative flex items-center gap-3 px-3 py-2 text-sm font-light tracking-wide transition-all duration-150 rounded-sm",
                          active
                            ? "text-[#d4af37] bg-[#d4af37]/[0.04]"
                            : "text-neutral-500 hover:text-white hover:bg-white/[0.02]",
                        )}
                      >
                        {active && (
                          <span className="absolute left-0 top-1/2 -translate-y-1/2 w-[2px] h-4 bg-[#d4af37] rounded-full" />
                        )}
                        <Icon
                          className={cn(
                            "w-4 h-4 shrink-0",
                            active ? "text-[#d4af37]" : "text-neutral-600",
                          )}
                        />
                        {item.name}
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
    <aside className="hidden md:flex w-56 shrink-0 border-r border-white/[0.06] flex-col h-screen sticky top-0">
      <SidebarContent />
    </aside>
  );
}

// --- Mobile header bar (visible on mobile only) ---

export function MobileHeader() {
  const { toggle } = useMobileSidebar();

  return (
    <header className="md:hidden fixed top-0 left-0 right-0 z-40 h-14 bg-black/80 backdrop-blur-xl border-b border-white/[0.06] flex items-center justify-between px-4">
      <div className="flex items-center">
        <button
          type="button"
          onClick={toggle}
          className="p-2 -ml-2 rounded-sm hover:bg-white/[0.04] transition-colors duration-150"
          aria-label="Toggle navigation"
        >
          <Menu className="w-5 h-5 text-neutral-400" />
        </button>
        <Link
          href="/"
          className="ml-3 text-base font-extralight tracking-[-0.04em]"
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
          "absolute inset-0 bg-black/60 backdrop-blur-sm transition-opacity duration-200",
          open ? "opacity-100" : "opacity-0",
        )}
        onClick={() => setOpen(false)}
        aria-hidden="true"
      />

      {/* Drawer panel */}
      <aside
        className={cn(
          "absolute top-0 left-0 bottom-0 w-64 bg-black border-r border-white/[0.06] flex flex-col transition-transform duration-200 ease-out",
          open ? "translate-x-0" : "-translate-x-full",
        )}
      >
        {/* Close button */}
        <div className="absolute top-4 right-4">
          <button
            type="button"
            onClick={() => setOpen(false)}
            className="p-1.5 rounded-sm hover:bg-white/[0.04] transition-colors duration-150"
            aria-label="Close navigation"
          >
            <X className="w-4 h-4 text-neutral-500" />
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

  return (
    <nav className="md:hidden fixed bottom-0 left-0 right-0 z-40 bg-black/80 backdrop-blur-xl border-t border-white/[0.06] flex items-center justify-around px-2" style={{ paddingBottom: "max(0.25rem, env(safe-area-inset-bottom))", height: "calc(4rem + env(safe-area-inset-bottom, 0px))" }}>
      {bottomTabs.map((tab) => {
        const active = pathname === tab.href;
        const Icon = tab.icon;
        return (
          <Link
            key={tab.href}
            href={tab.href}
            className={cn(
              "flex flex-col items-center justify-center gap-1 px-3 py-1.5 rounded-sm transition-colors duration-150 min-w-[3.5rem]",
              active
                ? "text-[#d4af37]"
                : "text-neutral-600 active:text-neutral-400",
            )}
          >
            <Icon className="w-5 h-5" />
            <span className="text-[10px] font-mono tracking-wide">
              {tab.name}
            </span>
          </Link>
        );
      })}
    </nav>
  );
}
