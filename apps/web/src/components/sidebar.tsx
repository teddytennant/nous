"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import { useConnection } from "@/components/connection-status";

const nav = [
  { name: "Dashboard", href: "/dashboard" },
  { name: "Social", href: "/social" },
  { name: "Messages", href: "/messages" },
  { name: "Governance", href: "/governance" },
  { name: "Marketplace", href: "/marketplace" },
  { name: "Files", href: "/files" },
  { name: "Wallet", href: "/wallet" },
  { name: "AI", href: "/ai" },
  { name: "Identity", href: "/identity" },
  { name: "Network", href: "/network" },
  { name: "Settings", href: "/settings" },
];

export function Sidebar() {
  const pathname = usePathname();
  const { status } = useConnection();

  return (
    <aside className="w-56 shrink-0 border-r border-white/[0.06] flex flex-col h-screen sticky top-0">
      <div className="px-6 pt-8 pb-12">
        <Link
          href="/"
          className="text-2xl font-extralight tracking-[-0.04em] hover:text-[#d4af37] transition-colors duration-200"
        >
          Nous
        </Link>
      </div>

      <nav className="flex-1 px-3">
        {nav.map((item) => {
          const active = pathname === item.href;
          return (
            <Link
              key={item.href}
              href={item.href}
              className={cn(
                "block px-3 py-2.5 text-sm font-light tracking-wide transition-colors duration-150",
                active
                  ? "text-[#d4af37]"
                  : "text-neutral-500 hover:text-white",
              )}
            >
              {item.name}
            </Link>
          );
        })}
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
