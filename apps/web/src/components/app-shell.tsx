"use client";

import type { ReactNode } from "react";
import { usePathname } from "next/navigation";
import {
  Sidebar,
  MobileHeader,
  MobileDrawer,
  BottomTabBar,
  MobileSidebarProvider,
} from "@/components/sidebar";
import { ConnectionProvider } from "@/components/connection-status";
import { ErrorBoundary } from "@/components/error-boundary";
import { ToastProvider } from "@/components/toast";
import { CommandPalette } from "@/components/command-palette";

function PageTransition({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  return (
    <div key={pathname} className="page-enter">
      {children}
    </div>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <ConnectionProvider>
      <ToastProvider>
        <MobileSidebarProvider>
          <div className="flex min-h-screen">
            <Sidebar />
            <MobileHeader />
            <MobileDrawer />
            <main className="flex-1 min-w-0 pt-14 md:pt-0 mobile-main-padding">
              <ErrorBoundary>
                <PageTransition>{children}</PageTransition>
              </ErrorBoundary>
            </main>
            <BottomTabBar />
          </div>
          <CommandPalette />
        </MobileSidebarProvider>
      </ToastProvider>
    </ConnectionProvider>
  );
}
