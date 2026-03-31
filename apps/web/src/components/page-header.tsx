"use client";

import { usePathname } from "next/navigation";
import type { ReactNode } from "react";

const BREADCRUMBS: Record<string, string> = {
  "/dashboard": "Overview",
  "/social": "Communication",
  "/messages": "Communication",
  "/wallet": "Finance",
  "/marketplace": "Finance",
  "/governance": "Finance",
  "/ai": "Intelligence",
  "/files": "Intelligence",
  "/network": "Intelligence",
  "/identity": "Account",
  "/settings": "Account",
};

type PageHeaderProps = {
  title: string;
  subtitle: string;
  status?: "online" | "offline";
  actions?: ReactNode;
};

export function PageHeader({
  title,
  subtitle,
  status,
  actions,
}: PageHeaderProps) {
  const pathname = usePathname();
  const section = BREADCRUMBS[pathname];

  return (
    <header className="mb-12">
      {section && (
        <nav
          className="flex items-center gap-1.5 mb-3"
          aria-label="Breadcrumb"
        >
          <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-700">
            {section}
          </span>
          <span className="text-neutral-800">/</span>
          <span className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-500">
            {title}
          </span>
        </nav>
      )}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
            {title}
          </h1>
          <div className="flex items-center gap-3">
            <p className="text-sm text-neutral-500 font-light">{subtitle}</p>
            {status !== undefined && (
              <span
                className={`inline-block w-1.5 h-1.5 rounded-full ${
                  status === "online" ? "bg-emerald-500" : "bg-red-500"
                }`}
              />
            )}
          </div>
        </div>
        {actions && <div className="shrink-0">{actions}</div>}
      </div>
    </header>
  );
}
