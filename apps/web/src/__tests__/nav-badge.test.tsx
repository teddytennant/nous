import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";

// Mock connection-status before importing sidebar
vi.mock("@/components/connection-status", () => ({
  ConnectionProvider: ({ children }: { children: React.ReactNode }) => children,
  useConnection: () => ({ status: "online", health: null }),
}));

// Mock notification-panel
vi.mock("@/components/notification-panel", () => ({
  NotificationBell: () => <div data-testid="notification-bell" />,
}));

// Mock did-avatar
vi.mock("@/components/did-avatar", () => ({
  DidAvatar: ({ did }: { did: string }) => (
    <div data-testid="did-avatar">{did}</div>
  ),
}));

const mockUsePathname = vi.fn(() => "/dashboard");
vi.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

import { setNavBadge, Sidebar, BottomTabBar, MobileSidebarProvider } from "@/components/sidebar";

describe("setNavBadge", () => {
  beforeEach(() => {
    // Clear all badges before each test
    setNavBadge("/social", 0);
    setNavBadge("/governance", 0);
    setNavBadge("/messages", 0);
    setNavBadge("/marketplace", 0);
    mockUsePathname.mockReturnValue("/dashboard");
  });

  it("displays a badge count on a sidebar nav item", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    act(() => {
      setNavBadge("/social", 3);
    });

    // The badge should show the count "3"
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("removes the badge when count is set to 0", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    act(() => {
      setNavBadge("/messages", 5);
    });

    expect(screen.getByText("5")).toBeInTheDocument();

    act(() => {
      setNavBadge("/messages", 0);
    });

    expect(screen.queryByText("5")).not.toBeInTheDocument();
  });

  it("caps display at 99+", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    act(() => {
      setNavBadge("/social", 150);
    });

    expect(screen.getByText("99+")).toBeInTheDocument();
  });

  it("supports multiple badges simultaneously", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    act(() => {
      setNavBadge("/social", 2);
      setNavBadge("/governance", 4);
    });

    expect(screen.getByText("2")).toBeInTheDocument();
    expect(screen.getByText("4")).toBeInTheDocument();
  });

  it("updates badge count when value changes", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    act(() => {
      setNavBadge("/messages", 1);
    });

    expect(screen.getByText("1")).toBeInTheDocument();

    act(() => {
      setNavBadge("/messages", 7);
    });

    expect(screen.queryByText("1")).not.toBeInTheDocument();
    expect(screen.getByText("7")).toBeInTheDocument();
  });
});

describe("BottomTabBar badges", () => {
  beforeEach(() => {
    setNavBadge("/social", 0);
    setNavBadge("/messages", 0);
    mockUsePathname.mockReturnValue("/dashboard");
  });

  it("shows badge on bottom tab when set", () => {
    render(<BottomTabBar />);

    act(() => {
      setNavBadge("/social", 3);
    });

    // BottomTabBar also renders the badge count
    expect(screen.getByText("3")).toBeInTheDocument();
  });
});
