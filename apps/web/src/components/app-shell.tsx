"use client";

import { useCallback, useEffect, useRef, useSyncExternalStore, useState, type ReactNode } from "react";
import { usePathname } from "next/navigation";
import {
  Sidebar,
  MobileHeader,
  MobileDrawer,
  BottomTabBar,
  MobileSidebarProvider,
} from "@/components/sidebar";
import { ConnectionProvider, useConnection } from "@/components/connection-status";
import { ErrorBoundary } from "@/components/error-boundary";
import { ToastProvider, useToast } from "@/components/toast";
import { CommandPalette } from "@/components/command-palette";
import { KeyboardShortcutsProvider } from "@/components/keyboard-shortcuts";
import { Onboarding } from "@/components/onboarding";
import { OfflineState, ConnectingState } from "@/components/offline-state";
import { ProductTour } from "@/components/product-tour";

function PageTransition({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  return (
    <div key={pathname} className="page-enter">
      {children}
    </div>
  );
}

function ConnectionGate({ children }: { children: ReactNode }) {
  const { status } = useConnection();

  if (status === "offline") {
    return <OfflineState />;
  }

  if (status === "connecting") {
    return <ConnectingState />;
  }

  return <>{children}</>;
}

function ReconnectionToast() {
  const { status } = useConnection();
  const { toast } = useToast();
  const prevStatus = useRef(status);

  useEffect(() => {
    if (prevStatus.current === "offline" && status === "online") {
      toast({ title: "Back online", description: "Connection restored", variant: "success" });
    }
    prevStatus.current = status;
  }, [status, toast]);

  return null;
}

const emptySubscribe = () => () => {};

function OnboardingGate({ children }: { children: ReactNode }) {
  const storedDid = useSyncExternalStore(
    emptySubscribe,
    () => localStorage.getItem("nous_did"),
    () => null,
  );

  const [completed, setCompleted] = useState(false);

  const hasIdentity = completed || !!storedDid;

  const handleComplete = useCallback(() => {
    setCompleted(true);
  }, []);

  if (!hasIdentity) {
    return <Onboarding onComplete={handleComplete} />;
  }

  return <>{children}</>;
}

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <ConnectionProvider>
      <ToastProvider>
        <ReconnectionToast />
        <OnboardingGate>
          <MobileSidebarProvider>
            <div className="flex min-h-screen">
              <Sidebar />
              <MobileHeader />
              <MobileDrawer />
              <main className="flex-1 min-w-0 pt-14 md:pt-0 mobile-main-padding">
                <ErrorBoundary>
                  <ConnectionGate>
                    <PageTransition>{children}</PageTransition>
                  </ConnectionGate>
                </ErrorBoundary>
              </main>
              <BottomTabBar />
            </div>
            <CommandPalette />
            <KeyboardShortcutsProvider />
            <ProductTour />
          </MobileSidebarProvider>
        </OnboardingGate>
      </ToastProvider>
    </ConnectionProvider>
  );
}
