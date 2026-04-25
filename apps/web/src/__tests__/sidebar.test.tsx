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

  it("renders all navigation items (mono lowercase)", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("dashboard")).toBeInTheDocument();
    expect(screen.getByText("social")).toBeInTheDocument();
    expect(screen.getByText("messages")).toBeInTheDocument();
    expect(screen.getByText("wallet")).toBeInTheDocument();
    expect(screen.getByText("marketplace")).toBeInTheDocument();
    expect(screen.getByText("governance")).toBeInTheDocument();
    expect(screen.getByText("ai")).toBeInTheDocument();
    expect(screen.getByText("files")).toBeInTheDocument();
    expect(screen.getByText("network")).toBeInTheDocument();
    expect(screen.getByText("identity")).toBeInTheDocument();
    expect(screen.getByText("settings")).toBeInTheDocument();
  });

  it("highlights the active route with oxblood color", () => {
    mockUsePathname.mockReturnValue("/wallet");
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    const walletLink = screen.getByText("wallet").closest("a")!;
    expect(walletLink.className).toContain("text-oxblood");
  });

  it("renders the search/command palette trigger", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    expect(screen.getByText("search")).toBeInTheDocument();
    expect(screen.getByText("⌘K")).toBeInTheDocument();
  });

  it("renders correct links for nav items", () => {
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    const dashboardLink = screen.getByText("dashboard").closest("a")!;
    expect(dashboardLink.getAttribute("href")).toBe("/dashboard");

    const aiLink = screen.getByText("ai").closest("a")!;
    expect(aiLink.getAttribute("href")).toBe("/ai");
  });

  it("can collapse sections", async () => {
    const user = userEvent.setup();
    render(
      <MobileSidebarProvider>
        <Sidebar />
      </MobileSidebarProvider>,
    );

    // Finance section has wallet, marketplace, governance
    expect(screen.getByText("wallet")).toBeInTheDocument();

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

  it("renders 5 tab items (mono lowercase)", () => {
    render(<BottomTabBar />);

    expect(screen.getByText("home")).toBeInTheDocument();
    expect(screen.getByText("social")).toBeInTheDocument();
    expect(screen.getByText("messages")).toBeInTheDocument();
    expect(screen.getByText("wallet")).toBeInTheDocument();
    expect(screen.getByText("ai")).toBeInTheDocument();
  });

  it("highlights the active tab with oxblood", () => {
    mockUsePathname.mockReturnValue("/social");
    render(<BottomTabBar />);

    const socialTab = screen.getByText("social").closest("a")!;
    expect(socialTab.className).toContain("text-oxblood");
  });

  it("non-active tabs use stone color", () => {
    mockUsePathname.mockReturnValue("/dashboard");
    render(<BottomTabBar />);

    const walletTab = screen.getByText("wallet").closest("a")!;
    expect(walletTab.className).toContain("text-stone");
  });

  it("has correct href on tab links", () => {
    render(<BottomTabBar />);

    expect(screen.getByText("home").closest("a")!.getAttribute("href")).toBe(
      "/dashboard",
    );
    expect(
      screen.getByText("messages").closest("a")!.getAttribute("href"),
    ).toBe("/messages");
  });
});
