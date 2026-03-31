"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter } from "next/navigation";
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
  ArrowRight,
  type LucideIcon,
} from "lucide-react";

interface CommandItem {
  id: string;
  label: string;
  description?: string;
  icon: LucideIcon;
  href?: string;
  action?: () => void;
  section: string;
  keywords?: string[];
}

const commands: CommandItem[] = [
  // Overview
  {
    id: "dashboard",
    label: "Dashboard",
    description: "Overview and system status",
    icon: LayoutDashboard,
    href: "/dashboard",
    section: "Navigation",
    keywords: ["home", "overview", "status"],
  },
  // Communication
  {
    id: "social",
    label: "Social",
    description: "Posts, feeds, and social graph",
    icon: Users,
    href: "/social",
    section: "Navigation",
    keywords: ["posts", "feed", "timeline", "people"],
  },
  {
    id: "messages",
    label: "Messages",
    description: "Direct messages and group chats",
    icon: MessageSquare,
    href: "/messages",
    section: "Navigation",
    keywords: ["chat", "dm", "conversation", "group"],
  },
  // Finance
  {
    id: "wallet",
    label: "Wallet",
    description: "Balances, transactions, and payments",
    icon: Wallet,
    href: "/wallet",
    section: "Navigation",
    keywords: ["money", "balance", "send", "receive", "payment"],
  },
  {
    id: "marketplace",
    label: "Marketplace",
    description: "Buy and sell on the network",
    icon: Store,
    href: "/marketplace",
    section: "Navigation",
    keywords: ["shop", "buy", "sell", "listings"],
  },
  {
    id: "governance",
    label: "Governance",
    description: "Proposals and voting",
    icon: Vote,
    href: "/governance",
    section: "Navigation",
    keywords: ["vote", "proposal", "dao", "delegate"],
  },
  // Intelligence
  {
    id: "ai",
    label: "AI",
    description: "AI assistants and models",
    icon: Brain,
    href: "/ai",
    section: "Navigation",
    keywords: ["assistant", "model", "chat", "inference"],
  },
  {
    id: "files",
    label: "Files",
    description: "Decentralized file storage",
    icon: FolderOpen,
    href: "/files",
    section: "Navigation",
    keywords: ["storage", "ipfs", "upload", "download"],
  },
  {
    id: "network",
    label: "Network",
    description: "Peers, nodes, and connectivity",
    icon: Globe,
    href: "/network",
    section: "Navigation",
    keywords: ["peers", "nodes", "p2p", "connectivity"],
  },
  // Account
  {
    id: "identity",
    label: "Identity",
    description: "DIDs, keys, and verification",
    icon: Fingerprint,
    href: "/identity",
    section: "Navigation",
    keywords: ["did", "key", "verification", "profile"],
  },
  {
    id: "settings",
    label: "Settings",
    description: "App configuration and preferences",
    icon: Settings,
    href: "/settings",
    section: "Navigation",
    keywords: ["config", "preferences", "theme", "account"],
  },
];

function scoreMatch(item: CommandItem, query: string): number {
  const q = query.toLowerCase();
  const label = item.label.toLowerCase();
  const desc = (item.description ?? "").toLowerCase();

  // Exact label match
  if (label === q) return 100;
  // Label starts with query
  if (label.startsWith(q)) return 80;
  // Label contains query
  if (label.includes(q)) return 60;
  // Description contains query
  if (desc.includes(q)) return 40;
  // Keyword match
  if (item.keywords?.some((k) => k.includes(q))) return 30;

  return 0;
}

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const router = useRouter();

  const filtered = useMemo(() => {
    if (!query.trim()) return commands;
    return commands
      .map((item) => ({ item, score: scoreMatch(item, query.trim()) }))
      .filter(({ score }) => score > 0)
      .sort((a, b) => b.score - a.score)
      .map(({ item }) => item);
  }, [query]);

  // Group by section
  const grouped = useMemo(() => {
    const groups: Record<string, CommandItem[]> = {};
    for (const item of filtered) {
      if (!groups[item.section]) groups[item.section] = [];
      groups[item.section].push(item);
    }
    return groups;
  }, [filtered]);

  const flatItems = filtered;

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
    setSelectedIndex(0);
  }, []);

  const execute = useCallback(
    (item: CommandItem) => {
      if (item.href) {
        router.push(item.href);
      } else if (item.action) {
        item.action();
      }
      close();
    },
    [router, close],
  );

  // Global keyboard shortcut
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
        if (!open) {
          setQuery("");
          setSelectedIndex(0);
        }
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open]);

  // Focus input when opened
  useEffect(() => {
    if (open) {
      // Small delay to ensure DOM is ready
      requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
    }
  }, [open]);

  // Keyboard navigation inside palette
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown": {
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev < flatItems.length - 1 ? prev + 1 : 0,
          );
          break;
        }
        case "ArrowUp": {
          e.preventDefault();
          setSelectedIndex((prev) =>
            prev > 0 ? prev - 1 : flatItems.length - 1,
          );
          break;
        }
        case "Enter": {
          e.preventDefault();
          if (flatItems[selectedIndex]) {
            execute(flatItems[selectedIndex]);
          }
          break;
        }
        case "Escape": {
          e.preventDefault();
          close();
          break;
        }
      }
    },
    [flatItems, selectedIndex, execute, close],
  );

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector("[data-selected='true']");
    if (selected) {
      selected.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  // Reset selection when query changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!open) return null;

  let flatIndex = 0;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm cmd-backdrop-enter"
        onClick={close}
      />

      {/* Dialog */}
      <div
        className="relative w-full max-w-lg mx-4 cmd-dialog-enter"
        onKeyDown={handleKeyDown}
      >
        <div className="overflow-hidden rounded-md border border-white/[0.08] bg-neutral-950 shadow-2xl shadow-black/50">
          {/* Search input */}
          <div className="flex items-center gap-3 px-4 border-b border-white/[0.06]">
            <Search className="w-4 h-4 text-neutral-600 shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search pages and actions..."
              className="flex-1 h-12 bg-transparent text-sm font-light text-white placeholder:text-neutral-600 outline-none"
              autoComplete="off"
              spellCheck={false}
            />
            <kbd className="hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 text-[10px] font-mono text-neutral-600 bg-white/[0.04] border border-white/[0.06] rounded">
              ESC
            </kbd>
          </div>

          {/* Results */}
          <div ref={listRef} className="max-h-72 overflow-y-auto py-2">
            {flatItems.length === 0 ? (
              <div className="px-4 py-8 text-center">
                <p className="text-sm text-neutral-600">No results found</p>
                <p className="text-xs text-neutral-700 mt-1">
                  Try a different search term
                </p>
              </div>
            ) : (
              Object.entries(grouped).map(([section, items]) => (
                <div key={section}>
                  <p className="px-4 pt-2 pb-1 text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-700">
                    {section}
                  </p>
                  {items.map((item) => {
                    const currentIndex = flatIndex++;
                    const isSelected = currentIndex === selectedIndex;
                    const Icon = item.icon;

                    return (
                      <button
                        key={item.id}
                        data-selected={isSelected}
                        onClick={() => execute(item)}
                        onMouseEnter={() => setSelectedIndex(currentIndex)}
                        className={`w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors duration-100 ${
                          isSelected
                            ? "bg-white/[0.04] text-white"
                            : "text-neutral-400 hover:text-white"
                        }`}
                      >
                        <Icon
                          className={`w-4 h-4 shrink-0 ${
                            isSelected ? "text-[#d4af37]" : "text-neutral-600"
                          }`}
                        />
                        <div className="flex-1 min-w-0">
                          <p className="text-sm font-light truncate">
                            {item.label}
                          </p>
                          {item.description && (
                            <p className="text-xs text-neutral-600 truncate mt-0.5">
                              {item.description}
                            </p>
                          )}
                        </div>
                        {isSelected && (
                          <ArrowRight className="w-3.5 h-3.5 text-neutral-600 shrink-0" />
                        )}
                      </button>
                    );
                  })}
                </div>
              ))
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center gap-4 px-4 py-2.5 border-t border-white/[0.06] text-[10px] font-mono text-neutral-700">
            <span className="flex items-center gap-1">
              <kbd className="px-1 py-0.5 bg-white/[0.04] border border-white/[0.06] rounded text-neutral-500">
                &uarr;
              </kbd>
              <kbd className="px-1 py-0.5 bg-white/[0.04] border border-white/[0.06] rounded text-neutral-500">
                &darr;
              </kbd>
              navigate
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1 py-0.5 bg-white/[0.04] border border-white/[0.06] rounded text-neutral-500">
                &crarr;
              </kbd>
              select
            </span>
            <span className="flex items-center gap-1">
              <kbd className="px-1 py-0.5 bg-white/[0.04] border border-white/[0.06] rounded text-neutral-500">
                esc
              </kbd>
              close
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
