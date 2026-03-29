"use client";

import type { ReactNode } from "react";
import { Sidebar } from "@/components/sidebar";
import { ConnectionProvider } from "@/components/connection-status";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <ConnectionProvider>
      <div className="flex min-h-screen">
        <Sidebar />
        <main className="flex-1 min-w-0">{children}</main>
      </div>
    </ConnectionProvider>
  );
}
