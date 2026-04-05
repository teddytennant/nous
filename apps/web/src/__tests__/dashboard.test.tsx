import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockNodeInfo = vi.fn();
const mockListDaos = vi.fn();
const mockGetWallet = vi.fn();
const mockHealth = vi.fn();

vi.mock("@/lib/api", () => ({
  node: {
    info: () => mockNodeInfo(),
    health: () => mockHealth(),
  },
  governance: { listDaos: () => mockListDaos() },
  payments: { getWallet: (did: string) => mockGetWallet(did) },
}));

const mockConnectionStatus = vi.fn<() => { status: string; health: { version: string; uptime_ms: number } | null }>(() => ({
  status: "online",
  health: { version: "0.1.0", uptime_ms: 120_000 },
}));

vi.mock("@/components/connection-status", () => ({
  ConnectionProvider: ({ children }: { children: React.ReactNode }) => children,
  useConnection: () => mockConnectionStatus(),
}));

vi.mock("@/components/subsystems", () => ({
  SubsystemsWidget: () => <div data-testid="subsystems-widget">Subsystems</div>,
}));

vi.mock("@/components/activity-timeline", () => ({
  ActivityTimeline: () => <div data-testid="activity-timeline">Timeline</div>,
}));

vi.mock("@/components/sparkline", () => ({
  Sparkline: ({ data }: { data: number[] }) => (
    <svg data-testid="sparkline" data-points={data.length} />
  ),
  MiniBarChart: ({ data }: { data: number[] }) => (
    <svg data-testid="mini-bar-chart" data-points={data.length} />
  ),
}));

vi.mock("@/components/did-avatar", () => ({
  DidAvatar: ({ did }: { did: string }) => (
    <div data-testid="did-avatar">{did}</div>
  ),
}));

import DashboardPage from "@/app/(app)/dashboard/page";

// ── Helpers ──────────────────────────────────────────────────────────────

function renderDashboard() {
  return render(<DashboardPage />);
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Dashboard page", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    localStorage.clear();

    mockNodeInfo.mockResolvedValue({
      protocol: "nous/1.0",
      version: "0.1.0",
      features: ["social", "messaging", "governance", "payments", "ai", "files", "marketplace", "identity"],
    });
    mockListDaos.mockResolvedValue({ daos: [], count: 3 });
    mockGetWallet.mockResolvedValue({
      did: "did:nous:test123",
      balances: [
        { token: "ETH", amount: "1.5" },
        { token: "NOUS", amount: "42000" },
        { token: "USDC", amount: "250.75" },
      ],
    });
    mockConnectionStatus.mockReturnValue({
      status: "online",
      health: { version: "0.1.0", uptime_ms: 120_000 },
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Welcome header ──────────────────────────────────────────────────

  describe("Welcome header", () => {
    it("renders a greeting", () => {
      renderDashboard();
      // The greeting depends on time — just check that one of the known greetings appears
      const header = screen.getByRole("heading", { level: 1 });
      expect(header).toBeInTheDocument();
      expect(header.textContent).toMatch(
        /Good morning|Good afternoon|Good evening|Late night/
      );
    });

    it("shows the current date", () => {
      renderDashboard();
      // The date line should contain the day of the week
      const days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
      const today = days[new Date().getDay()];
      expect(screen.getByText(new RegExp(today))).toBeInTheDocument();
    });

    it("shows the display name when set in localStorage", () => {
      localStorage.setItem("nous_display_name", "Teddy");
      renderDashboard();
      expect(screen.getByText(", Teddy")).toBeInTheDocument();
    });

    it("does not show display name when not set", () => {
      renderDashboard();
      const header = screen.getByRole("heading", { level: 1 });
      // Header should not contain a comma (which introduces the display name)
      expect(header.textContent).not.toContain(",");
    });

    it("shows DID avatar when did is stored", () => {
      localStorage.setItem("nous_did", "did:nous:abc123");
      renderDashboard();
      expect(screen.getByTestId("did-avatar")).toHaveTextContent("did:nous:abc123");
    });

    it("shows the subtitle", () => {
      renderDashboard();
      expect(
        screen.getByText("Your sovereign digital infrastructure, at a glance.")
      ).toBeInTheDocument();
    });
  });

  // ── Stats section ───────────────────────────────────────────────────

  describe("Stats cards", () => {
    it("renders all four stat labels", () => {
      renderDashboard();
      expect(screen.getByText("Status")).toBeInTheDocument();
      expect(screen.getByText("Uptime")).toBeInTheDocument();
      expect(screen.getByText("DAOs")).toBeInTheDocument();
      expect(screen.getByText("Features")).toBeInTheDocument();
    });

    it("shows Online when connected", () => {
      renderDashboard();
      expect(screen.getByText("Online")).toBeInTheDocument();
    });

    it("shows version detail when online", () => {
      renderDashboard();
      expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    });

    it("shows Offline when disconnected", () => {
      mockConnectionStatus.mockReturnValue({ status: "offline", health: null });
      renderDashboard();
      expect(screen.getByText("Offline")).toBeInTheDocument();
    });

    it("shows skeleton when data is loading", () => {
      mockConnectionStatus.mockReturnValue({ status: "connecting", health: null });
      renderDashboard();
      // When status is "connecting", stat values are "..." which triggers Skeleton rendering
      // Skeleton is a div with animate-pulse class
      const skeletons = document.querySelectorAll(".animate-pulse");
      expect(skeletons.length).toBeGreaterThan(0);
    });

    it("renders sparklines in stat cards", () => {
      renderDashboard();
      const sparklines = screen.getAllByTestId("sparkline");
      expect(sparklines.length).toBeGreaterThanOrEqual(4);
    });

    it("shows uptime formatted", async () => {
      renderDashboard();
      // 120000ms = 2m
      await waitFor(() => {
        expect(screen.getByText("2m")).toBeInTheDocument();
      });
    });

    it("shows DAO count after fetch", async () => {
      renderDashboard();
      await waitFor(() => {
        expect(screen.getByText("3")).toBeInTheDocument();
      });
    });

    it("shows feature count after fetch", async () => {
      renderDashboard();
      await waitFor(() => {
        expect(screen.getByText("8")).toBeInTheDocument();
      });
    });
  });

  // ── Weekly activity ─────────────────────────────────────────────────

  describe("Weekly activity", () => {
    it("renders the section heading", () => {
      renderDashboard();
      expect(screen.getByText("Weekly Activity")).toBeInTheDocument();
    });

    it("renders all four activity metrics", () => {
      renderDashboard();
      expect(screen.getByText("Posts")).toBeInTheDocument();
      expect(screen.getByText("Messages")).toBeInTheDocument();
      expect(screen.getByText("Transactions")).toBeInTheDocument();
      expect(screen.getByText("Peers")).toBeInTheDocument();
    });

    it("renders mini bar charts", () => {
      renderDashboard();
      const charts = screen.getAllByTestId("mini-bar-chart");
      expect(charts).toHaveLength(4);
    });

    it("shows day labels", () => {
      renderDashboard();
      // Mon and Sun as range labels
      expect(screen.getAllByText("Mon")).toHaveLength(4);
      expect(screen.getAllByText("Sun")).toHaveLength(4);
    });
  });

  // ── Wallet balance strip ────────────────────────────────────────────

  describe("Wallet balance strip", () => {
    it("renders the wallet heading", () => {
      renderDashboard();
      expect(screen.getByText("Wallet")).toBeInTheDocument();
    });

    it("renders manage link", () => {
      renderDashboard();
      expect(screen.getByText("Manage")).toBeInTheDocument();
    });

    it("shows default token labels when no wallet loaded", () => {
      // Before wallet loads, default tokens should show
      renderDashboard();
      expect(screen.getByText("ETH")).toBeInTheDocument();
      expect(screen.getByText("NOUS")).toBeInTheDocument();
      expect(screen.getByText("USDC")).toBeInTheDocument();
    });

    it("links wallet section to /wallet", () => {
      renderDashboard();
      const manageLink = screen.getByText("Manage").closest("a");
      expect(manageLink).toHaveAttribute("href", "/wallet");
    });
  });

  // ── Quick actions ───────────────────────────────────────────────────

  describe("Quick actions", () => {
    it("renders the section heading", () => {
      renderDashboard();
      expect(screen.getByText("Quick Actions")).toBeInTheDocument();
    });

    it("renders all six quick actions", () => {
      renderDashboard();
      expect(screen.getByText("Create Post")).toBeInTheDocument();
      expect(screen.getByText("Send Message")).toBeInTheDocument();
      expect(screen.getByText("Browse Market")).toBeInTheDocument();
      expect(screen.getByText("Governance")).toBeInTheDocument();
      expect(screen.getByText("AI Chat")).toBeInTheDocument();
      expect(screen.getByText("Upload File")).toBeInTheDocument();
    });

    it("shows descriptions for each action", () => {
      renderDashboard();
      expect(screen.getByText("Share with the network")).toBeInTheDocument();
      expect(screen.getByText("E2E encrypted chat")).toBeInTheDocument();
      expect(screen.getByText("Explore listings")).toBeInTheDocument();
      expect(screen.getByText("Vote on proposals")).toBeInTheDocument();
      expect(screen.getByText("Talk to agents")).toBeInTheDocument();
      expect(screen.getByText("Encrypted storage")).toBeInTheDocument();
    });

    it("links to correct routes", () => {
      renderDashboard();
      expect(screen.getByText("Create Post").closest("a")).toHaveAttribute("href", "/social");
      expect(screen.getByText("Send Message").closest("a")).toHaveAttribute("href", "/messages");
      expect(screen.getByText("Browse Market").closest("a")).toHaveAttribute("href", "/marketplace");
      expect(screen.getByText("AI Chat").closest("a")).toHaveAttribute("href", "/ai");
      expect(screen.getByText("Upload File").closest("a")).toHaveAttribute("href", "/files");
    });
  });

  // ── Activity & Subsystems ───────────────────────────────────────────

  describe("Activity timeline and subsystems", () => {
    it("renders the activity timeline section", () => {
      renderDashboard();
      expect(screen.getByText("Activity Timeline")).toBeInTheDocument();
      expect(screen.getByTestId("activity-timeline")).toBeInTheDocument();
    });

    it("renders the subsystems section", () => {
      renderDashboard();
      // Multiple "Subsystems" text nodes may exist (section heading + sidebar)
      expect(screen.getAllByText("Subsystems").length).toBeGreaterThanOrEqual(1);
      expect(screen.getByTestId("subsystems-widget")).toBeInTheDocument();
    });

    it("links 'View all' to /social", () => {
      renderDashboard();
      expect(screen.getByText("View all").closest("a")).toHaveAttribute("href", "/social");
    });
  });

  // ── API integration ─────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls node.info on mount", async () => {
      renderDashboard();
      await waitFor(() => {
        expect(mockNodeInfo).toHaveBeenCalled();
      });
    });

    it("calls governance.listDaos on mount", async () => {
      renderDashboard();
      await waitFor(() => {
        expect(mockListDaos).toHaveBeenCalled();
      });
    });

    it("calls payments.getWallet when DID exists", async () => {
      localStorage.setItem("nous_did", "did:nous:test");
      renderDashboard();
      await waitFor(() => {
        expect(mockGetWallet).toHaveBeenCalledWith("did:nous:test");
      });
    });

    it("does not call payments.getWallet when no DID", async () => {
      // Clear all mocks to ensure no prior calls pollute
      mockGetWallet.mockClear();
      // localStorage is already cleared in beforeEach, no nous_did set
      renderDashboard();
      // Give effects time to run
      await act(async () => {
        vi.advanceTimersByTime(100);
      });
      expect(mockGetWallet).not.toHaveBeenCalled();
    });

    it("handles node.info failure gracefully", async () => {
      mockNodeInfo.mockRejectedValue(new Error("offline"));
      renderDashboard();
      // Should not throw — the page renders with fallback values
      await waitFor(() => {
        expect(screen.getByText("Status")).toBeInTheDocument();
      });
    });

    it("handles governance.listDaos failure gracefully", async () => {
      mockListDaos.mockRejectedValue(new Error("offline"));
      renderDashboard();
      await waitFor(() => {
        expect(screen.getByText("DAOs")).toBeInTheDocument();
      });
    });
  });
});
