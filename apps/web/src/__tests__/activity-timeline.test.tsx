import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockFeed = vi.fn();
const mockListProposals = vi.fn();
const mockGetTransactions = vi.fn();
const mockSearch = vi.fn();
const mockListChannels = vi.fn();

vi.mock("@/lib/api", () => ({
  social: { feed: (...args: unknown[]) => mockFeed(...args) },
  governance: { listProposals: (...args: unknown[]) => mockListProposals(...args) },
  payments: { getTransactions: (...args: unknown[]) => mockGetTransactions(...args) },
  marketplace: { search: (...args: unknown[]) => mockSearch(...args) },
  messaging: { listChannels: (...args: unknown[]) => mockListChannels(...args) },
}));

vi.mock("@/components/avatar", () => ({
  Avatar: ({ did, size }: { did: string; size?: string }) => (
    <div data-testid="avatar" data-did={did} data-size={size} />
  ),
}));

import { ActivityTimeline } from "@/components/activity-timeline";

// ── Test data helpers ────────────────────────────────────────────────────

const TEST_DID = "did:key:z6MktestDID12345";
const LONG_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc";

function makeFeedEvent(overrides: Partial<{
  id: string;
  pubkey: string;
  created_at: string;
  kind: number;
  content: string;
  tags: string[][];
}> = {}) {
  return {
    id: overrides.id ?? "feed-1",
    pubkey: overrides.pubkey ?? LONG_DID,
    created_at: overrides.created_at ?? new Date(Date.now() - 60_000).toISOString(),
    kind: overrides.kind ?? 1,
    content: overrides.content ?? "Hello world from Nous!",
    tags: overrides.tags ?? [],
  };
}

function makeProposal(overrides: Partial<{
  id: string;
  dao_id: string;
  title: string;
  description: string;
  proposer_did: string;
  status: string;
  created_at: string;
}> = {}) {
  return {
    id: overrides.id ?? "prop-1",
    dao_id: overrides.dao_id ?? "dao-1",
    title: overrides.title ?? "Increase treasury allocation",
    description: overrides.description ?? "A proposal to increase allocation",
    proposer_did: overrides.proposer_did ?? LONG_DID,
    status: overrides.status ?? "Active",
    created_at: overrides.created_at ?? new Date(Date.now() - 120_000).toISOString(),
    voting_starts: new Date().toISOString(),
    voting_ends: new Date(Date.now() + 86_400_000).toISOString(),
    quorum: 50,
  };
}

function makeTransaction(overrides: Partial<{
  id: string;
  from_did: string;
  to_did: string;
  token: string;
  amount: string;
  fee: string;
  memo: string | null;
  status: string;
  timestamp: string;
}> = {}) {
  return {
    id: overrides.id ?? "tx-1",
    from_did: overrides.from_did ?? LONG_DID,
    to_did: overrides.to_did ?? "did:key:z6MkrecipientABCDEF1234567890abcdef",
    token: overrides.token ?? "NOUS",
    amount: overrides.amount ?? "500",
    fee: overrides.fee ?? "0.01",
    memo: overrides.memo ?? null,
    status: overrides.status ?? "confirmed",
    timestamp: overrides.timestamp ?? new Date(Date.now() - 180_000).toISOString(),
  };
}

function makeListing(overrides: Partial<{
  id: string;
  seller_did: string;
  title: string;
  description: string;
  category: string;
  price_token: string;
  price_amount: number;
  quantity: number;
  status: string;
  created_at: string;
  tags: string[];
  images: string[];
}> = {}) {
  return {
    id: overrides.id ?? "listing-1",
    seller_did: overrides.seller_did ?? LONG_DID,
    title: overrides.title ?? "Vintage Keyboard",
    description: overrides.description ?? "A classic keyboard",
    category: overrides.category ?? "electronics",
    price_token: overrides.price_token ?? "NOUS",
    price_amount: overrides.price_amount ?? 100,
    quantity: overrides.quantity ?? 1,
    status: overrides.status ?? "active",
    created_at: overrides.created_at ?? new Date(Date.now() - 240_000).toISOString(),
    tags: overrides.tags ?? [],
    images: overrides.images ?? [],
  };
}

function makeChannel(overrides: Partial<{
  id: string;
  kind: string;
  name: string | null;
  members: string[];
  created_at: string;
}> = {}) {
  return {
    id: overrides.id ?? "chan-1",
    kind: overrides.kind ?? "group",
    name: overrides.name ?? "General Chat",
    members: overrides.members ?? [LONG_DID, "did:key:z6Mkother"],
    created_at: overrides.created_at ?? new Date(Date.now() - 300_000).toISOString(),
  };
}

function setAllApisEmpty() {
  mockFeed.mockResolvedValue({ events: [], count: 0 });
  mockListProposals.mockResolvedValue({ proposals: [] });
  mockGetTransactions.mockResolvedValue([]);
  mockSearch.mockResolvedValue({ listings: [] });
  mockListChannels.mockResolvedValue([]);
}

function setAllApisWithData() {
  mockFeed.mockResolvedValue({
    events: [makeFeedEvent()],
    count: 1,
  });
  mockListProposals.mockResolvedValue({
    proposals: [makeProposal()],
  });
  mockGetTransactions.mockResolvedValue([makeTransaction()]);
  mockSearch.mockResolvedValue({
    listings: [makeListing()],
  });
  mockListChannels.mockResolvedValue([makeChannel()]);
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("ActivityTimeline", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    localStorage.clear();
    localStorage.setItem("nous_did", TEST_DID);

    mockFeed.mockReset();
    mockListProposals.mockReset();
    mockGetTransactions.mockReset();
    mockSearch.mockReset();
    mockListChannels.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Skeleton ─────────────────────────────────────────────────────────

  describe("TimelineSkeleton", () => {
    it("renders 4 skeleton rows while loading", () => {
      // Make APIs never resolve to keep loading state
      mockFeed.mockReturnValue(new Promise(() => {}));
      mockListProposals.mockReturnValue(new Promise(() => {}));
      mockGetTransactions.mockReturnValue(new Promise(() => {}));
      mockSearch.mockReturnValue(new Promise(() => {}));
      mockListChannels.mockReturnValue(new Promise(() => {}));

      const { container } = render(<ActivityTimeline />);

      // Each skeleton row has a round avatar skeleton (w-7 h-7 rounded-full)
      const avatarSkeletons = container.querySelectorAll(".rounded-full.animate-pulse");
      expect(avatarSkeletons).toHaveLength(4);
    });

    it("each skeleton row has avatar, text, and timestamp placeholders", () => {
      mockFeed.mockReturnValue(new Promise(() => {}));
      mockListProposals.mockReturnValue(new Promise(() => {}));
      mockGetTransactions.mockReturnValue(new Promise(() => {}));
      mockSearch.mockReturnValue(new Promise(() => {}));
      mockListChannels.mockReturnValue(new Promise(() => {}));

      const { container } = render(<ActivityTimeline />);

      // Each row: 1 avatar skeleton + 4 text skeletons + 1 timestamp skeleton = 6 per row
      const allSkeletons = container.querySelectorAll(".animate-pulse");
      // 4 rows x 6 skeletons = 24
      expect(allSkeletons).toHaveLength(24);
    });
  });

  // ── Loading state ────────────────────────────────────────────────────

  describe("Loading state", () => {
    it("shows skeleton while API calls are pending", () => {
      mockFeed.mockReturnValue(new Promise(() => {}));
      mockListProposals.mockReturnValue(new Promise(() => {}));
      mockGetTransactions.mockReturnValue(new Promise(() => {}));
      mockSearch.mockReturnValue(new Promise(() => {}));
      mockListChannels.mockReturnValue(new Promise(() => {}));

      const { container } = render(<ActivityTimeline />);

      const skeletons = container.querySelectorAll(".animate-pulse");
      expect(skeletons.length).toBeGreaterThan(0);
      // Should NOT have any event content
      expect(screen.queryByText("New post")).not.toBeInTheDocument();
    });
  });

  // ── Empty state ──────────────────────────────────────────────────────

  describe("Empty state", () => {
    it("shows 'No activity yet' when all APIs return empty", async () => {
      setAllApisEmpty();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText(/No activity yet/)).toBeInTheDocument();
      });
    });

    it("shows 'Create a post' CTA link", async () => {
      setAllApisEmpty();
      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("Create a post");
        expect(link).toBeInTheDocument();
        expect(link.closest("a")).toHaveAttribute("href", "/social");
      });
    });

    it("renders Users icon in empty state", async () => {
      setAllApisEmpty();
      const { container } = render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText(/No activity yet/)).toBeInTheDocument();
      });
      // The Users icon is rendered as an SVG inside the empty state div
      const svg = container.querySelector("svg");
      expect(svg).toBeInTheDocument();
    });
  });

  // ── Successful data loading ──────────────────────────────────────────

  describe("Successful data loading", () => {
    it("renders social events from feed", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("New post")).toBeInTheDocument();
      });
    });

    it("renders governance events from proposals", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Proposal: Increase treasury allocation")).toBeInTheDocument();
      });
    });

    it("renders payment events from transactions", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Sent 500 NOUS")).toBeInTheDocument();
      });
    });

    it("renders marketplace events from listings", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Listed: Vintage Keyboard")).toBeInTheDocument();
      });
    });

    it("renders message events from channels", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Group created: General Chat")).toBeInTheDocument();
      });
    });

    it("shows correct kind labels", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Social")).toBeInTheDocument();
        expect(screen.getByText("Governance")).toBeInTheDocument();
        expect(screen.getByText("Payment")).toBeInTheDocument();
        expect(screen.getByText("Market")).toBeInTheDocument();
        expect(screen.getByText("Message")).toBeInTheDocument();
      });
    });

    it("shows truncated DID for actor", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        // LONG_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc"
        // truncateDid: first 12 + "..." + last 6 = "did:key:z6Mk...vSJfc"
        const truncated = `${LONG_DID.slice(0, 12)}…${LONG_DID.slice(-6)}`;
        const elements = screen.getAllByText(truncated);
        expect(elements.length).toBeGreaterThan(0);
      });
    });

    it("shows event detail for social (content truncated at 80 chars)", async () => {
      const longContent = "A".repeat(100);
      mockFeed.mockResolvedValue({
        events: [makeFeedEvent({ content: longContent })],
        count: 1,
      });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        // Should show first 80 chars + ellipsis
        expect(screen.getByText(`${"A".repeat(80)}…`)).toBeInTheDocument();
      });
    });

    it("shows event detail for proposal status", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        // Active proposals show "Open for voting"
        expect(screen.getByText("Open for voting")).toBeInTheDocument();
      });
    });

    it("shows event detail for transaction amount", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([makeTransaction({ to_did: "did:key:z6MkrecipientABCDEF1234567890abcdef" })]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        // The to_did is >20 chars so it gets truncated in detail
        const toDid = "did:key:z6MkrecipientABCDEF1234567890abcdef";
        const truncated = `to ${toDid.slice(0, 12)}…${toDid.slice(-6)}`;
        expect(screen.getByText(truncated)).toBeInTheDocument();
      });
    });

    it("shows event detail for listing price", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("100 NOUS")).toBeInTheDocument();
      });
    });

    it("shows event detail for channel member count", async () => {
      setAllApisWithData();
      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("2 members")).toBeInTheDocument();
      });
    });

    it("shows singular 'member' for channels with 1 member", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([
        makeChannel({ members: [LONG_DID] }),
      ]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("1 member")).toBeInTheDocument();
      });
    });
  });

  // ── Sorting and limit ────────────────────────────────────────────────

  describe("Sorting and limit", () => {
    it("events are sorted by timestamp (newest first)", async () => {
      const now = Date.now();
      // Create events at different times
      mockFeed.mockResolvedValue({
        events: [
          makeFeedEvent({ id: "old", content: "Old post", created_at: new Date(now - 600_000).toISOString() }),
        ],
        count: 1,
      });
      mockListProposals.mockResolvedValue({
        proposals: [
          makeProposal({ id: "new", title: "New proposal", created_at: new Date(now - 10_000).toISOString() }),
        ],
      });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      const { container } = render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Proposal: New proposal")).toBeInTheDocument();
      });

      // Check that the proposal (newer) appears before the social post (older) in the DOM
      const links = container.querySelectorAll("a");
      const texts = Array.from(links).map((a) => a.textContent);
      const proposalIdx = texts.findIndex((t) => t?.includes("Proposal: New proposal"));
      const postIdx = texts.findIndex((t) => t?.includes("Old post"));
      expect(proposalIdx).toBeLessThan(postIdx);
    });

    it("only shows top 8 events", async () => {
      const now = Date.now();
      // Create 10+ events across sources
      const feedEvents = Array.from({ length: 5 }, (_, i) =>
        makeFeedEvent({
          id: `feed-${i}`,
          content: `Feed post ${i}`,
          created_at: new Date(now - i * 60_000).toISOString(),
        })
      );
      const proposals = Array.from({ length: 5 }, (_, i) =>
        makeProposal({
          id: `prop-${i}`,
          title: `Proposal ${i}`,
          created_at: new Date(now - (i + 5) * 60_000).toISOString(),
        })
      );

      mockFeed.mockResolvedValue({ events: feedEvents, count: 5 });
      mockListProposals.mockResolvedValue({ proposals });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      const { container } = render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getAllByText("New post").length).toBeGreaterThan(0);
      });

      // Each event is rendered as an <a> link
      const rows = container.querySelectorAll("a");
      expect(rows).toHaveLength(8);
    });
  });

  // ── Partial failures (Promise.allSettled) ────────────────────────────

  describe("Partial failures", () => {
    it("shows available data when social API fails", async () => {
      mockFeed.mockRejectedValue(new Error("Social API down"));
      mockListProposals.mockResolvedValue({ proposals: [makeProposal()] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Proposal: Increase treasury allocation")).toBeInTheDocument();
      });
      expect(screen.queryByText("New post")).not.toBeInTheDocument();
    });

    it("shows available data when governance API fails", async () => {
      mockFeed.mockResolvedValue({ events: [makeFeedEvent()], count: 1 });
      mockListProposals.mockRejectedValue(new Error("Governance API down"));
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("New post")).toBeInTheDocument();
      });
      expect(screen.queryByText(/Proposal:/)).not.toBeInTheDocument();
    });

    it("shows events from remaining APIs when multiple fail", async () => {
      mockFeed.mockRejectedValue(new Error("down"));
      mockListProposals.mockRejectedValue(new Error("down"));
      mockGetTransactions.mockRejectedValue(new Error("down"));
      mockSearch.mockResolvedValue({ listings: [makeListing()] });
      mockListChannels.mockResolvedValue([makeChannel()]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("Listed: Vintage Keyboard")).toBeInTheDocument();
        expect(screen.getByText("Group created: General Chat")).toBeInTheDocument();
      });
    });
  });

  // ── Links ────────────────────────────────────────────────────────────

  describe("Event links", () => {
    it("social events link to /social", async () => {
      mockFeed.mockResolvedValue({ events: [makeFeedEvent()], count: 1 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("New post").closest("a");
        expect(link).toHaveAttribute("href", "/social");
      });
    });

    it("governance events link to /governance", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [makeProposal()] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("Proposal: Increase treasury allocation").closest("a");
        expect(link).toHaveAttribute("href", "/governance");
      });
    });

    it("payment events link to /wallet", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([makeTransaction()]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("Sent 500 NOUS").closest("a");
        expect(link).toHaveAttribute("href", "/wallet");
      });
    });

    it("marketplace events link to /marketplace", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [makeListing()] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("Listed: Vintage Keyboard").closest("a");
        expect(link).toHaveAttribute("href", "/marketplace");
      });
    });

    it("message events link to /messages", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([makeChannel()]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        const link = screen.getByText("Group created: General Chat").closest("a");
        expect(link).toHaveAttribute("href", "/messages");
      });
    });
  });

  // ── DID handling ─────────────────────────────────────────────────────

  describe("DID handling", () => {
    it("passes userDid to payments.getTransactions and messaging.listChannels", async () => {
      localStorage.setItem("nous_did", TEST_DID);
      setAllApisEmpty();
      render(<ActivityTimeline />);

      await waitFor(() => {
        expect(mockGetTransactions).toHaveBeenCalledWith(TEST_DID, 5);
        expect(mockListChannels).toHaveBeenCalledWith(TEST_DID);
      });
    });

    it("without userDid, payments and messaging get empty arrays (no API calls)", async () => {
      localStorage.clear(); // No DID set
      mockFeed.mockResolvedValue({ events: [makeFeedEvent()], count: 1 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);

      await waitFor(() => {
        expect(screen.getByText("New post")).toBeInTheDocument();
      });

      // When there's no DID, Promise.resolve([]) is used instead of calling the APIs
      expect(mockGetTransactions).not.toHaveBeenCalled();
      expect(mockListChannels).not.toHaveBeenCalled();
    });
  });

  // ── Time formatting ──────────────────────────────────────────────────

  describe("Time formatting", () => {
    it("shows 'just now' for events within the last 60 seconds", async () => {
      mockFeed.mockResolvedValue({
        events: [makeFeedEvent({ created_at: new Date(Date.now() - 5_000).toISOString() })],
        count: 1,
      });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("just now")).toBeInTheDocument();
      });
    });

    it("shows 'Xm ago' for events minutes old", async () => {
      mockFeed.mockResolvedValue({
        events: [makeFeedEvent({ created_at: new Date(Date.now() - 5 * 60_000).toISOString() })],
        count: 1,
      });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("5m ago")).toBeInTheDocument();
      });
    });

    it("shows 'Xh ago' for events hours old", async () => {
      mockFeed.mockResolvedValue({
        events: [makeFeedEvent({ created_at: new Date(Date.now() - 3 * 3_600_000).toISOString() })],
        count: 1,
      });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("3h ago")).toBeInTheDocument();
      });
    });

    it("shows 'Xd ago' for events days old", async () => {
      mockFeed.mockResolvedValue({
        events: [makeFeedEvent({ created_at: new Date(Date.now() - 2 * 86_400_000).toISOString() })],
        count: 1,
      });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("2d ago")).toBeInTheDocument();
      });
    });
  });

  // ── DM channel (non-group) ──────────────────────────────────────────

  describe("Channel variants", () => {
    it("shows 'New conversation' for DM channels", async () => {
      mockFeed.mockResolvedValue({ events: [], count: 0 });
      mockListProposals.mockResolvedValue({ proposals: [] });
      mockGetTransactions.mockResolvedValue([]);
      mockSearch.mockResolvedValue({ listings: [] });
      mockListChannels.mockResolvedValue([
        makeChannel({ kind: "dm", name: null }),
      ]);

      render(<ActivityTimeline />);
      await waitFor(() => {
        expect(screen.getByText("New conversation")).toBeInTheDocument();
      });
    });
  });
});
