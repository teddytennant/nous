"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import { useConnection } from "@/components/connection-status";
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
} from "lucide-react";

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

export function Sidebar() {
  const pathname = usePathname();
  const { status } = useConnection();

  return (
    <aside className="w-56 shrink-0 border-r border-white/[0.06] flex flex-col h-screen sticky top-0">
      <div className="px-6 pt-8 pb-8">
        <Link
          href="/"
          className="text-2xl font-extralight tracking-[-0.04em] hover:text-[#d4af37] transition-colors duration-200"
        >
          Nous
        </Link>
      </div>

      <nav className="flex-1 overflow-y-auto px-3 space-y-6">
        {sections.map((section) => (
          <div key={section.label}>
            <p className="px-3 mb-2 text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-700">
              {section.label}
            </p>
            {section.items.map((item) => {
              const active = pathname === item.href;
              const Icon = item.icon;
              return (
                <Link
                  key={item.href}
                  href={item.href}
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
        ))}
      </nav>

      <div className="px-6 py-6 border-t border-white/[0.04]">
        <div className="flex items-center gap-2">
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
      </div>
    </aside>
  );
}
