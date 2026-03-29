"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";

const nav = [
  { name: "Dashboard", href: "/dashboard" },
  { name: "Social", href: "/social" },
  { name: "Messages", href: "/messages" },
  { name: "Governance", href: "/governance" },
  { name: "Wallet", href: "/wallet" },
  { name: "Identity", href: "/identity" },
];

export function Sidebar() {
  const pathname = usePathname();

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
                  : "text-neutral-500 hover:text-white"
              )}
            >
              {item.name}
            </Link>
          );
        })}
      </nav>

      <div className="px-6 py-6 border-t border-white/[0.04]">
        <p className="text-[10px] font-mono text-neutral-700 tracking-wider uppercase">
          nous v0.1.0
        </p>
      </div>
    </aside>
  );
}
