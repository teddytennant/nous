"use client";

import type { ReactNode } from "react";
import { Sidebar } from "@/components/sidebar";
import { ConnectionProvider } from "@/components/connection-status";
import { ErrorBoundary } from "@/components/error-boundary";
import { ToastProvider } from "@/components/toast";
import { CommandPalette } from "@/components/command-palette";

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <ConnectionProvider>
      <ToastProvider>
        <div className="flex min-h-screen">
          <Sidebar />
          <main className="flex-1 min-w-0">
            <ErrorBoundary>{children}</ErrorBoundary>
          </main>
        </div>
        <CommandPalette />
      </ToastProvider>
    </ConnectionProvider>
  );
}
