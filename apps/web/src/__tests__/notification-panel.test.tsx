import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { NotificationBell } from "@/components/notification-panel";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockFeed = vi.fn();
const mockListProposals = vi.fn();
const mockGetTransactions = vi.fn();
const mockMarketplaceSearch = vi.fn();
const mockListChannels = vi.fn();

vi.mock("@/lib/api", () => ({
  social: { feed: (opts: unknown) => mockFeed(opts) },
  governance: { listProposals: () => mockListProposals() },
  payments: {
    getTransactions: (did: string, limit: number) =>
      mockGetTransactions(did, limit),
  },
  marketplace: { search: (opts: unknown) => mockMarketplaceSearch(opts) },
  messaging: {
    listChannels: (did: string) => mockListChannels(did),
  },
}));

vi.mock("@/components/avatar", () => ({
  Avatar: ({ did, size }: { did: string; size?: string }) => (
    <div data-testid="avatar" data-did={did} data-size={size} />
  ),
}));

// Helper to find a filter tab button by its label text
function getFilterTab(label: string): HTMLElement {
  const buttons = screen.getAllByRole("button");
  const match = buttons.find(
    (b) => b.textContent?.trim().startsWith(label) && b.className.includes("font-mono"),
  );
  if (!match) throw new Error(`Filter tab "${label}" not found`);
  return match;
}

// ── Fixtures ─────────────────────────────────────────────────────────────

const now = new Date("2026-04-05T10:00:00Z");

const feedEvents = {
  events: [
    {
      id: "evt-1",
      pubkey: "did:key:z6MkAlice",
      content: "Hello world from the decentralized web!",
      created_at: now.toISOString(),
    },
    {
      id: "evt-2",
      pubkey: "did:key:z6MkBob",
      content: "Building with Nous is incredible, the sovereignty stack is exactly what we needed for our community.",
      created_at: new Date(now.getTime() - 60_000).toISOString(),
    },
  ],
};

const proposals = {
  proposals: [
    {
      id: "prop-1",
      proposer_did: "did:key:z6MkProposer",
      title: "Increase treasury allocation",
      status: "Active",
      created_at: now.toISOString(),
    },
  ],
};

const transactions = [
  {
    id: "tx-1",
    from_did: "did:key:z6MkSender",
    amount: "1.5",
    token: "ETH",
    timestamp: now.toISOString(),
  },
];

const listings = {
  listings: [
    {
      id: "listing-1",
      seller_did: "did:key:z6MkSeller",
      title: "Premium Widget",
      price_amount: "50",
      price_token: "NOUS",
      created_at: now.toISOString(),
    },
  ],
};

const channels = [
  {
    id: "chan-1",
    kind: "dm",
    name: null,
    members: ["did:key:z6MkMember1", "did:key:z6MkMe"],
    created_at: now.toISOString(),
  },
];

function setupMocksWithData() {
  mockFeed.mockResolvedValue(feedEvents);
  mockListProposals.mockResolvedValue(proposals);
  mockGetTransactions.mockResolvedValue(transactions);
  mockMarketplaceSearch.mockResolvedValue(listings);
  mockListChannels.mockResolvedValue(channels);
}

function setupMocksEmpty() {
  mockFeed.mockResolvedValue({ events: [] });
  mockListProposals.mockResolvedValue({ proposals: [] });
  mockGetTransactions.mockResolvedValue([]);
  mockMarketplaceSearch.mockResolvedValue({ listings: [] });
  mockListChannels.mockResolvedValue([]);
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("NotificationBell", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    vi.setSystemTime(now);
    mockFeed.mockReset();
    mockListProposals.mockReset();
    mockGetTransactions.mockReset();
    mockMarketplaceSearch.mockReset();
    mockListChannels.mockReset();
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Bell icon", () => {
    it("renders a bell button", async () => {
      setupMocksEmpty();
      render(<NotificationBell />);

      const button = screen.getByRole("button", { name: /notifications/i });
      expect(button).toBeInTheDocument();
    });

    it("shows unread count badge when there are unread notifications", async () => {
      setupMocksWithData();
      localStorage.setItem("nous_did", "did:key:z6MkMe");
      render(<NotificationBell />);

      await waitFor(() => {
        const badge = screen.getByText(/\d/);
        expect(badge).toBeInTheDocument();
      });
    });

    it("does not show badge when there are no notifications", async () => {
      setupMocksEmpty();
      render(<NotificationBell />);

      // Wait for fetch to complete
      await waitFor(() => {
        expect(mockFeed).toHaveBeenCalled();
      });

      // No badge should be present — the bell aria-label should not include "unread"
      const button = screen.getByRole("button", { name: "Notifications" });
      expect(button).toBeInTheDocument();
    });

    it("includes unread count in aria-label", async () => {
      setupMocksWithData();
      localStorage.setItem("nous_did", "did:key:z6MkMe");
      render(<NotificationBell />);

      await waitFor(() => {
        const button = screen.getByRole("button", {
          name: /notifications.*unread/i,
        });
        expect(button).toBeInTheDocument();
      });
    });
  });

  describe("Panel open/close", () => {
    it("opens panel on bell click", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());

      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("Notifications")).toBeInTheDocument();
    });

    it("closes panel on second bell click", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());

      const bell = screen.getByRole("button", { name: /notifications/i });
      await user.click(bell);
      expect(screen.getByText("Notifications")).toBeInTheDocument();

      await user.click(bell);
      expect(screen.queryByText("Notifications")).not.toBeInTheDocument();
    });

    it("closes panel on Escape key", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());

      await user.click(screen.getByRole("button", { name: /notifications/i }));
      expect(screen.getByText("Notifications")).toBeInTheDocument();

      await user.keyboard("{Escape}");
      expect(screen.queryByText("Notifications")).not.toBeInTheDocument();
    });

    it("closes panel on click outside", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const { container } = render(
        <div>
          <div data-testid="outside">Outside</div>
          <NotificationBell />
        </div>,
      );

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());

      await user.click(screen.getByRole("button", { name: /notifications/i }));
      expect(screen.getByText("Notifications")).toBeInTheDocument();

      await user.click(screen.getByTestId("outside"));
      expect(screen.queryByText("Notifications")).not.toBeInTheDocument();
    });

    it("has close button in panel header", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());

      await user.click(screen.getByRole("button", { name: /notifications/i }));
      const closeBtn = screen.getByRole("button", {
        name: /close notifications/i,
      });
      expect(closeBtn).toBeInTheDocument();

      await user.click(closeBtn);
      expect(screen.queryByText("Notifications")).not.toBeInTheDocument();
    });
  });

  describe("Notification content", () => {
    it("renders social notification from feed", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // 2 feed events → 2 "New post" notifications
      const posts = screen.getAllByText("New post");
      expect(posts.length).toBe(2);
    });

    it("truncates long content to 60 chars", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // The second feed event has content > 60 chars, should be truncated with "..."
      const truncated = screen.getByText(/Building with Nous.*\.\.\./);
      expect(truncated).toBeInTheDocument();
    });

    it("renders governance notification from proposals", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(
        screen.getByText("Proposal: Increase treasury allocation"),
      ).toBeInTheDocument();
      expect(screen.getByText("Open for voting")).toBeInTheDocument();
    });

    it("renders payment notification from transactions", async () => {
      setupMocksWithData();
      localStorage.setItem("nous_did", "did:key:z6MkMe");
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("Received 1.5 ETH")).toBeInTheDocument();
    });

    it("renders marketplace notification from listings", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(
        screen.getByText("New listing: Premium Widget"),
      ).toBeInTheDocument();
      expect(screen.getByText("50 NOUS")).toBeInTheDocument();
    });

    it("renders message notification from channels", async () => {
      setupMocksWithData();
      localStorage.setItem("nous_did", "did:key:z6MkMe");
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("New conversation")).toBeInTheDocument();
    });

    it("shows timestamps on notifications", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Notifications at current time should show "just now"
      const justNow = screen.getAllByText("just now");
      expect(justNow.length).toBeGreaterThan(0);
    });

    it("renders avatar for each notification", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      const avatars = screen.getAllByTestId("avatar");
      expect(avatars.length).toBeGreaterThan(0);
    });
  });

  describe("Filter tabs", () => {
    it("shows all filter tabs", async () => {
      setupMocksEmpty();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Use getAllByRole to find filter buttons (the "All", "Social", etc. tabs)
      const buttons = screen.getAllByRole("button");
      const tabLabels = buttons.map((b) => b.textContent?.trim());
      expect(tabLabels).toContain("All");
      expect(tabLabels).toContain("Social");
      expect(tabLabels).toContain("Gov");
      expect(tabLabels).toContain("Pay");
      expect(tabLabels).toContain("Market");
      expect(tabLabels).toContain("Msg");
    });

    it("filters to governance notifications only", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));
      await user.click(getFilterTab("Gov"));

      expect(
        screen.getByText("Proposal: Increase treasury allocation"),
      ).toBeInTheDocument();
      // Social posts should be hidden
      expect(screen.queryAllByText("New post")).toHaveLength(0);
    });

    it("filters to social notifications only", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));
      await user.click(getFilterTab("Social"));

      expect(screen.getAllByText("New post").length).toBe(2);
      expect(
        screen.queryByText("Proposal: Increase treasury allocation"),
      ).not.toBeInTheDocument();
    });

    it("shows empty state for filter with no matching notifications", async () => {
      // Only provide feed events — no marketplace
      mockFeed.mockResolvedValue(feedEvents);
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockMarketplaceSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));
      await user.click(getFilterTab("Market"));

      expect(
        screen.getByText("No market notifications"),
      ).toBeInTheDocument();
    });
  });

  describe("Mark as read", () => {
    it("shows unread dot for unread notifications", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const { container } = render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Unread dots are w-2 h-2 rounded-full bg-[#d4af37]
      const dots = container.querySelectorAll(".bg-\\[\\#d4af37\\].rounded-full.w-2.h-2");
      expect(dots.length).toBeGreaterThan(0);
    });

    it("marks notification as read on click", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Click a notification link (use first "New post" since there are 2)
      const postLink = screen.getAllByText("New post")[0].closest("a");
      expect(postLink).toBeInTheDocument();
      await user.click(postLink!);

      // Read IDs should be persisted
      const stored = localStorage.getItem("nous_read_notifications");
      expect(stored).toBeTruthy();
      const ids = JSON.parse(stored!);
      expect(ids.length).toBeGreaterThan(0);
    });

    it("shows Mark all read button when there are unreads", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("Mark all read")).toBeInTheDocument();
    });

    it("marks all as read on Mark all read click", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));
      await user.click(screen.getByText("Mark all read"));

      // Mark all read button should disappear since count is now 0
      expect(screen.queryByText("Mark all read")).not.toBeInTheDocument();
    });

    it("persists read state to localStorage", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));
      await user.click(screen.getByText("Mark all read"));

      const stored = localStorage.getItem("nous_read_notifications");
      expect(stored).toBeTruthy();
      const ids = JSON.parse(stored!);
      expect(ids.length).toBeGreaterThan(0);
    });
  });

  describe("Empty state", () => {
    it("shows empty state when no notifications exist", async () => {
      setupMocksEmpty();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("No notifications yet")).toBeInTheDocument();
    });
  });

  describe("Loading state", () => {
    it("shows skeleton while loading", async () => {
      // Don't resolve the mock — keep it pending
      mockFeed.mockReturnValue(new Promise(() => {}));
      mockListProposals.mockReturnValue(new Promise(() => {}));
      mockGetTransactions.mockReturnValue(new Promise(() => {}));
      mockMarketplaceSearch.mockReturnValue(new Promise(() => {}));
      mockListChannels.mockReturnValue(new Promise(() => {}));

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const { container } = render(<NotificationBell />);

      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Skeleton uses animate-pulse divs
      const skeletons = container.querySelectorAll(".animate-pulse");
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });

  describe("API error handling", () => {
    it("handles partial API failures gracefully", async () => {
      // Only social succeeds, everything else fails
      mockFeed.mockResolvedValue(feedEvents);
      mockListProposals.mockRejectedValue(new Error("Network error"));
      mockGetTransactions.mockRejectedValue(new Error("Network error"));
      mockMarketplaceSearch.mockRejectedValue(new Error("Network error"));
      mockListChannels.mockRejectedValue(new Error("Network error"));

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Social notifications should still appear (2 feed events → 2 "New post")
      expect(screen.getAllByText("New post").length).toBe(2);
    });

    it("shows empty state when all APIs fail", async () => {
      mockFeed.mockRejectedValue(new Error("Network error"));
      mockListProposals.mockRejectedValue(new Error("Network error"));
      mockGetTransactions.mockRejectedValue(new Error("Network error"));
      mockMarketplaceSearch.mockRejectedValue(new Error("Network error"));
      mockListChannels.mockRejectedValue(new Error("Network error"));

      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      expect(screen.getByText("No notifications yet")).toBeInTheDocument();
    });
  });

  describe("Notification preferences", () => {
    it("respects muted notification kinds from settings", async () => {
      // Mute social notifications
      localStorage.setItem(
        "nous_notif_prefs",
        JSON.stringify({ social: false }),
      );

      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Social posts should not appear
      expect(screen.queryAllByText("New post")).toHaveLength(0);
      // Governance should still appear
      expect(
        screen.getByText("Proposal: Increase treasury allocation"),
      ).toBeInTheDocument();
    });
  });

  describe("Polling", () => {
    it("refreshes notifications every 30 seconds", async () => {
      setupMocksWithData();
      render(<NotificationBell />);

      await waitFor(() => {
        expect(mockFeed).toHaveBeenCalledTimes(1);
      });

      // Advance past the 30s interval
      vi.advanceTimersByTime(30_000);

      await waitFor(() => {
        expect(mockFeed).toHaveBeenCalledTimes(2);
      });
    });
  });

  describe("Accessibility", () => {
    it("bell button has aria-expanded attribute", async () => {
      setupMocksEmpty();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      const bell = screen.getByRole("button", { name: /notifications/i });
      expect(bell).toHaveAttribute("aria-expanded", "false");

      await user.click(bell);
      expect(bell).toHaveAttribute("aria-expanded", "true");
    });

    it("notification links navigate to correct pages", async () => {
      setupMocksWithData();
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<NotificationBell />);

      await waitFor(() => expect(mockFeed).toHaveBeenCalled());
      await user.click(screen.getByRole("button", { name: /notifications/i }));

      // Social notifications should link to /social
      const socialLink = screen.getAllByText("New post")[0].closest("a");
      expect(socialLink).toHaveAttribute("href", "/social");

      // Governance should link to /governance
      const govLink = screen
        .getByText("Proposal: Increase treasury allocation")
        .closest("a");
      expect(govLink).toHaveAttribute("href", "/governance");
    });
  });
});
