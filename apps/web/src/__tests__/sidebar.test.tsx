import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

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

// We need to mock usePathname per test
const mockUsePathname = vi.fn(() => "/dashboard");
vi.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

import { Sidebar, BottomTabBar, MobileSidebarProvider } from "@/components/sidebar";

describe("Sidebar", () => {
  beforeEach(() => {
    mockUsePathname.mockReturnValue("/dashboard");
    localStorage.clear();
  });

  it("renders the Nous logo", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("Nous")).toBeInTheDocument();
  });

  it("renders all navigation sections", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Communication")).toBeInTheDocument();
    expect(screen.getByText("Finance")).toBeInTheDocument();
    expect(screen.getByText("Intelligence")).toBeInTheDocument();
    expect(screen.getByText("Account")).toBeInTheDocument();
  });

  it("renders all navigation items", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Social")).toBeInTheDocument();
    expect(screen.getByText("Messages")).toBeInTheDocument();
    expect(screen.getByText("Wallet")).toBeInTheDocument();
    expect(screen.getByText("Marketplace")).toBeInTheDocument();
    expect(screen.getByText("Governance")).toBeInTheDocument();
    expect(screen.getByText("AI")).toBeInTheDocument();
    expect(screen.getByText("Files")).toBeInTheDocument();
    expect(screen.getByText("Network")).toBeInTheDocument();
    expect(screen.getByText("Identity")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("highlights the active route with gold color", () => {
    mockUsePathname.mockReturnValue("/wallet");
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    const walletLink = screen.getByText("Wallet").closest("a")!;
    expect(walletLink.className).toContain("text-[#d4af37]");
  });

  it("renders the search/command palette trigger", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("Search...")).toBeInTheDocument();
    expect(screen.getByText("⌘K")).toBeInTheDocument();
  });

  it("renders correct links for nav items", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    const dashboardLink = screen.getByText("Dashboard").closest("a")!;
    expect(dashboardLink.getAttribute("href")).toBe("/dashboard");

    const aiLink = screen.getByText("AI").closest("a")!;
    expect(aiLink.getAttribute("href")).toBe("/ai");
  });

  it("can collapse sections", async () => {
    const user = userEvent.setup();
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    // Finance section has Wallet, Marketplace, Governance
    expect(screen.getByText("Wallet")).toBeInTheDocument();

    // Click the Finance section header to collapse
    const financeHeader = screen.getByText("Finance").closest("button")!;
    await user.click(financeHeader);

    // Section should be collapsed (data-collapsed="true")
    const section = financeHeader
      .closest("div")!
      .querySelector('[data-collapsed="true"]');
    expect(section).toBeInTheDocument();
  });
});

describe("BottomTabBar", () => {
  beforeEach(() => {
    mockUsePathname.mockReturnValue("/dashboard");
  });

  it("renders 5 tab items", () => {
    render(<BottomTabBar />);

    expect(screen.getByText("Home")).toBeInTheDocument();
    expect(screen.getByText("Social")).toBeInTheDocument();
    expect(screen.getByText("Messages")).toBeInTheDocument();
    expect(screen.getByText("Wallet")).toBeInTheDocument();
    expect(screen.getByText("AI")).toBeInTheDocument();
  });

  it("highlights the active tab with gold", () => {
    mockUsePathname.mockReturnValue("/social");
    render(<BottomTabBar />);

    const socialTab = screen.getByText("Social").closest("a")!;
    expect(socialTab.className).toContain("text-[#d4af37]");
  });

  it("non-active tabs use neutral color", () => {
    mockUsePathname.mockReturnValue("/dashboard");
    render(<BottomTabBar />);

    const walletTab = screen.getByText("Wallet").closest("a")!;
    expect(walletTab.className).toContain("text-neutral-600");
  });

  it("has correct href on tab links", () => {
    render(<BottomTabBar />);

    expect(screen.getByText("Home").closest("a")!.getAttribute("href")).toBe(
      "/dashboard",
    );
    expect(
      screen.getByText("Messages").closest("a")!.getAttribute("href"),
    ).toBe("/messages");
  });
});
