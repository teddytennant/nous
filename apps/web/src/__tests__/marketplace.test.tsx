import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockSearch = vi.fn();
const mockCreateListing = vi.fn();
const mockGetSellerRating = vi.fn();
const mockOrdersList = vi.fn();
const mockDisputesList = vi.fn();
const mockOffersList = vi.fn();

vi.mock("@/lib/api", () => ({
  marketplace: {
    search: (params?: unknown) => mockSearch(params),
    createListing: (data: unknown) => mockCreateListing(data),
    getSellerRating: (did: string) => mockGetSellerRating(did),
  },
  orders: {
    list: () => mockOrdersList(),
  },
  disputes: {
    list: () => mockDisputesList(),
  },
  offers: {
    list: () => mockOffersList(),
  },
}));

const mockToast = vi.fn(() => "toast-id");

vi.mock("@/components/toast", () => ({
  useToast: () => ({ toast: mockToast, dismiss: vi.fn() }),
  ToastProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock("@/components/page-header", () => ({
  PageHeader: ({ title, subtitle }: { title: string; subtitle: string }) => (
    <div data-testid="page-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({
    title,
    description,
    action,
  }: {
    title: string;
    description: string;
    action?: React.ReactNode;
  }) => (
    <div data-testid="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  ),
  MarketplaceIllustration: () => <div data-testid="marketplace-illustration" />,
  OrdersIllustration: () => <div data-testid="orders-illustration" />,
  DisputeIllustration: () => <div data-testid="dispute-illustration" />,
  OffersIllustration: () => <div data-testid="offers-illustration" />,
}));

vi.mock("@/components/sidebar", () => ({
  setNavBadge: vi.fn(),
}));

vi.mock("@/components/keyboard-shortcuts", () => ({
  usePageShortcuts: vi.fn(),
  useListNavigation: () => ({
    selectedIndex: -1,
    setSelectedIndex: vi.fn(),
    containerRef: { current: null },
  }),
}));

vi.mock("@/components/avatar", () => ({
  Avatar: ({ did }: { did: string }) => (
    <div data-testid="avatar" data-did={did} />
  ),
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({
    children,
    content,
  }: {
    children: React.ReactNode;
    content: string;
  }) => <span title={content}>{children}</span>,
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({
    open,
    children,
  }: {
    open: boolean;
    onOpenChange: (v: boolean) => void;
    children: React.ReactNode;
  }) => (open ? <div data-testid="dialog">{children}</div> : null),
  DialogHeader: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  DialogTitle: ({ children }: { children: React.ReactNode }) => (
    <h2>{children}</h2>
  ),
  DialogDescription: ({ children }: { children: React.ReactNode }) => (
    <p>{children}</p>
  ),
  DialogBody: ({
    children,
  }: {
    children: React.ReactNode;
    className?: string;
  }) => <div>{children}</div>,
  DialogFooter: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: ({
    label,
    ...props
  }: React.InputHTMLAttributes<HTMLInputElement> & { label?: string }) => (
    <div>
      {label && <label>{label}</label>}
      <input {...props} />
    </div>
  ),
}));

vi.mock("@/components/ui/textarea", () => ({
  Textarea: ({
    label,
    ...props
  }: React.TextareaHTMLAttributes<HTMLTextAreaElement> & { label?: string }) => (
    <div>
      {label && <label>{label}</label>}
      <textarea {...props} />
    </div>
  ),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({
    label,
    value,
    onValueChange,
    options,
  }: {
    label?: string;
    value: string;
    onValueChange: (v: string) => void;
    options: { value: string; label: string }[];
  }) => (
    <div>
      {label && <label>{label}</label>}
      <select
        value={value}
        onChange={(e) => onValueChange(e.target.value)}
        data-testid={`select-${label?.toLowerCase().replace(/\s+/g, "-")}`}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  ),
}));

vi.mock("@/components/ui/data-table", () => ({
  DataTable: ({
    columns,
    data,
    rowKey,
    emptyState,
  }: {
    columns: { id: string; header: string; cell: (row: unknown) => React.ReactNode }[];
    data: unknown[];
    rowKey: (row: unknown, i: number) => string;
    emptyState?: React.ReactNode;
    defaultSortId?: string;
    defaultSortDir?: string;
  }) =>
    data.length === 0 ? (
      <div data-testid="data-table-empty">{emptyState}</div>
    ) : (
      <table data-testid="data-table">
        <thead>
          <tr>
            {columns.map((col) => (
              <th key={col.id}>{col.header}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.map((row, i) => (
            <tr key={rowKey(row, i)}>
              {columns.map((col) => (
                <td key={col.id}>{col.cell(row)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    ),
}));

// ── Test data ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

const MOCK_LISTINGS = [
  {
    id: "listing-1",
    seller_did: MOCK_DID,
    title: "Encrypted USB Drive",
    description: "Hardware-encrypted 256GB USB drive with self-destruct",
    category: "physical",
    price_token: "USDC",
    price_amount: 4999,
    quantity: 10,
    status: "Active",
    created_at: new Date(Date.now() - 86400000).toISOString(),
    tags: ["hardware", "crypto"],
    images: [],
  },
  {
    id: "listing-2",
    seller_did: "did:key:z6MkotherSeller",
    title: "Smart Contract Audit",
    description: "Full audit of your Solidity contracts with formal verification",
    category: "service",
    price_token: "ETH",
    price_amount: 200000,
    quantity: 1,
    status: "Active",
    created_at: new Date(Date.now() - 172800000).toISOString(),
    tags: ["audit", "security"],
    images: [],
  },
];

const MOCK_ORDERS = [
  {
    id: "order-1",
    listing_id: "listing-1",
    buyer_did: "did:key:z6Mkbuyer1",
    seller_did: MOCK_DID,
    token: "USDC",
    amount: 4999,
    quantity: 1,
    status: "Pending",
    escrow_id: null,
    shipping: null,
    created_at: new Date(Date.now() - 3600000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString(),
    completed_at: null,
  },
  {
    id: "order-2",
    listing_id: "listing-2",
    buyer_did: MOCK_DID,
    seller_did: "did:key:z6MkotherSeller",
    token: "ETH",
    amount: 200000,
    quantity: 1,
    status: "Completed",
    escrow_id: "escrow-1",
    shipping: null,
    created_at: new Date(Date.now() - 604800000).toISOString(),
    updated_at: new Date(Date.now() - 172800000).toISOString(),
    completed_at: new Date(Date.now() - 172800000).toISOString(),
  },
];

const MOCK_DISPUTES = [
  {
    id: "dispute-1",
    order_id: "order-1",
    initiator_did: "did:key:z6Mkbuyer1",
    respondent_did: MOCK_DID,
    reason: "ItemNotAsDescribed",
    description: "Item doesn't match listing photos",
    evidence_count: 3,
    status: "Open",
    arbiter_did: null,
    resolution_note: null,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    resolved_at: null,
  },
];

const MOCK_OFFERS = [
  {
    id: "offer-1",
    listing_id: "listing-1",
    buyer_did: "did:key:z6Mkbuyer2",
    seller_did: MOCK_DID,
    token: "USDC",
    amount: 3500,
    message: "Would you take 35?",
    status: "Pending",
    counter_amount: null,
    created_at: new Date(Date.now() - 7200000).toISOString(),
    expires_at: new Date(Date.now() + 86400000).toISOString(),
    responded_at: null,
  },
];

const MOCK_RATING = {
  seller_did: MOCK_DID,
  total_reviews: 12,
  average_rating: 4.7,
  verified_reviews: 10,
};

// ── Helpers ──────────────────────────────────────────────────────────────

import MarketplacePage from "@/app/(app)/marketplace/page";

function setupDefaults() {
  mockSearch.mockResolvedValue({ listings: MOCK_LISTINGS, count: 2 });
  mockOrdersList.mockResolvedValue({ orders: MOCK_ORDERS, count: 2 });
  mockDisputesList.mockResolvedValue({ disputes: MOCK_DISPUTES, count: 1 });
  mockOffersList.mockResolvedValue({ offers: MOCK_OFFERS, count: 1 });
  mockGetSellerRating.mockResolvedValue(MOCK_RATING);
  mockCreateListing.mockResolvedValue({
    id: "listing-new",
    seller_did: MOCK_DID,
    title: "New Item",
    description: "Desc",
    category: "digital",
    price_token: "USDC",
    price_amount: 100,
    quantity: 1,
    status: "Active",
    created_at: new Date().toISOString(),
    tags: [],
    images: [],
  });

  Object.defineProperty(window, "localStorage", {
    value: {
      getItem: vi.fn((key: string) => {
        if (key === "nous_did") return MOCK_DID;
        if (key === "nous_display_name") return "Teddy";
        return null;
      }),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    },
    writable: true,
  });
}

async function renderMarketplace() {
  setupDefaults();
  render(<MarketplacePage />);
  await waitFor(() => {
    expect(screen.getByTestId("page-header")).toBeInTheDocument();
  });
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Marketplace page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Page structure ────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header with title and subtitle", async () => {
      await renderMarketplace();
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Marketplace")).toBeInTheDocument();
      expect(
        within(header).getByText("P2P. Reputation-gated. Escrow-backed.")
      ).toBeInTheDocument();
    });

    it("renders all four tabs", async () => {
      await renderMarketplace();
      expect(screen.getByText("Listings")).toBeInTheDocument();
      expect(screen.getByText("Orders")).toBeInTheDocument();
      expect(screen.getByText("Disputes")).toBeInTheDocument();
      expect(screen.getByText("Offers")).toBeInTheDocument();
    });

    it("defaults to Listings tab", async () => {
      await renderMarketplace();
      const listingsTab = screen.getByText("Listings");
      expect(listingsTab.className).toContain("border-[#d4af37]");
    });

    it("renders New Listing button", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
    });
  });

  // ── Loading states ────────────────────────────────────────────────────

  describe("Loading states", () => {
    it("shows skeleton cards while listings load", async () => {
      mockSearch.mockReturnValue(new Promise(() => {})); // never resolves
      mockDisputesList.mockResolvedValue({ disputes: [], count: 0 });
      setupDefaults();
      mockSearch.mockReturnValue(new Promise(() => {}));
      render(<MarketplacePage />);
      // Skeletons are rendered as animated placeholder elements
      const skeletons = document.querySelectorAll('[class*="animate-pulse"], [data-slot="skeleton"]');
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });

  // ── Listings tab ──────────────────────────────────────────────────────

  describe("Listings tab", () => {
    it("renders listing titles", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
        expect(screen.getByText("Smart Contract Audit")).toBeInTheDocument();
      });
    });

    it("renders listing prices", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("49.99 USDC")).toBeInTheDocument();
        expect(screen.getByText("2000.00 ETH")).toBeInTheDocument();
      });
    });

    it("renders listing status", async () => {
      await renderMarketplace();
      await waitFor(() => {
        const activeLabels = screen.getAllByText("Active");
        expect(activeLabels.length).toBeGreaterThanOrEqual(2);
      });
    });

    it("renders listing categories", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("physical")).toBeInTheDocument();
        expect(screen.getByText("service")).toBeInTheDocument();
      });
    });

    it("renders listing tags", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("hardware")).toBeInTheDocument();
        expect(screen.getByText("crypto")).toBeInTheDocument();
        expect(screen.getByText("audit")).toBeInTheDocument();
        expect(screen.getByText("security")).toBeInTheDocument();
      });
    });

    it("shows search input", async () => {
      await renderMarketplace();
      expect(
        screen.getByPlaceholderText("Search listings...")
      ).toBeInTheDocument();
    });

    it("shows category filter buttons", async () => {
      await renderMarketplace();
      expect(screen.getByText("All")).toBeInTheDocument();
      expect(screen.getByText("Physical")).toBeInTheDocument();
      expect(screen.getByText("Digital")).toBeInTheDocument();
      expect(screen.getByText("Service")).toBeInTheDocument();
      expect(screen.getByText("NFT")).toBeInTheDocument();
      expect(screen.getByText("Data")).toBeInTheDocument();
      expect(screen.getByText("Other")).toBeInTheDocument();
    });

    it("shows sort options", async () => {
      await renderMarketplace();
      expect(screen.getByText("Newest")).toBeInTheDocument();
      expect(screen.getByText("Price ↑")).toBeInTheDocument();
      expect(screen.getByText("Price ↓")).toBeInTheDocument();
      expect(screen.getByText("Title A–Z")).toBeInTheDocument();
    });

    it("calls marketplace.search on mount", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(mockSearch).toHaveBeenCalledWith({ limit: 50 });
      });
    });
  });

  // ── Listing detail expand ─────────────────────────────────────────────

  describe("Listing detail", () => {
    // Note: Detail content uses CSS grid collapse (always in DOM, hidden via CSS).
    // In JSDOM, CSS doesn't apply, so detail content is always visible.

    it("renders detail section with quantity", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      // Detail content is always in DOM (CSS-hidden when collapsed)
      expect(screen.getByText("10 available")).toBeInTheDocument();
    });

    it("renders Buy Now buttons for active listings", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      // Both listings are Active, so 2 Buy Now buttons
      const buyButtons = screen.getAllByText("Buy Now");
      expect(buyButtons.length).toBe(2);
    });

    it("renders Make Offer buttons for active listings", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      const offerButtons = screen.getAllByText("Make Offer");
      expect(offerButtons.length).toBe(2);
    });

    it("renders Contact buttons for active listings", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      const contactButtons = screen.getAllByText("Contact");
      expect(contactButtons.length).toBe(2);
    });

    it("renders listing IDs in detail view", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      expect(screen.getByText(/ID: listing-1/)).toBeInTheDocument();
      expect(screen.getByText(/ID: listing-2/)).toBeInTheDocument();
    });

    it("sets expanded data attribute on click", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      // Find listing-detail elements
      const details = document.querySelectorAll(".listing-detail");
      expect(details[0]?.getAttribute("data-expanded")).toBe("false");
      await user.click(screen.getByText("Encrypted USB Drive"));
      expect(details[0]?.getAttribute("data-expanded")).toBe("true");
    });

    it("shows toast when Buy Now clicked", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      const buyButtons = screen.getAllByText("Buy Now");
      await user.click(buyButtons[0]);
      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({ title: "Purchase started" })
      );
    });

    it("fetches seller rating when listing expanded", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Encrypted USB Drive"));
      await waitFor(() => {
        expect(mockGetSellerRating).toHaveBeenCalledWith(MOCK_DID);
      });
    });
  });

  // ── Category filter ───────────────────────────────────────────────────

  describe("Category filter", () => {
    it("filters listings by category on click", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("Encrypted USB Drive")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Physical"));
      await waitFor(() => {
        expect(mockSearch).toHaveBeenCalledWith(
          expect.objectContaining({ category: "physical" })
        );
      });
    });

    it("highlights active category filter", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      const allBtn = screen.getByText("All");
      expect(allBtn.closest("button")?.className).toContain("text-[#d4af37]");
      await user.click(screen.getByText("Digital"));
      const digitalBtn = screen.getByText("Digital");
      expect(digitalBtn.closest("button")?.className).toContain(
        "text-[#d4af37]"
      );
    });
  });

  // ── Create listing dialog ─────────────────────────────────────────────

  describe("Create listing dialog", () => {
    it("opens create dialog on New Listing click", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      expect(screen.getByTestId("dialog")).toBeInTheDocument();
      expect(screen.getByText("Create Listing")).toBeInTheDocument();
    });

    it("shows form fields in create dialog", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      expect(screen.getByText("Title")).toBeInTheDocument();
      expect(screen.getByText("Description")).toBeInTheDocument();
      expect(screen.getByText("Category")).toBeInTheDocument();
      expect(screen.getByText("Token")).toBeInTheDocument();
      expect(screen.getByText("Tags")).toBeInTheDocument();
    });

    it("shows Publish and Cancel buttons", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      expect(screen.getByText("Publish")).toBeInTheDocument();
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("calls createListing API on Publish", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      const titleInput = screen.getByPlaceholderText(
        "What are you selling?"
      );
      await user.type(titleInput, "Test Item");
      const priceInput = screen.getByPlaceholderText("0");
      await user.type(priceInput, "1000");
      await user.click(screen.getByText("Publish"));
      await waitFor(() => {
        expect(mockCreateListing).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Test Item",
            price_amount: 1000,
          })
        );
      });
    });

    it("shows success toast after creating listing", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      await user.type(
        screen.getByPlaceholderText("What are you selling?"),
        "Test Item"
      );
      await user.type(screen.getByPlaceholderText("0"), "1000");
      await user.click(screen.getByText("Publish"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Listing published",
            variant: "success",
          })
        );
      });
    });
  });

  // ── Empty states ──────────────────────────────────────────────────────

  describe("Empty states", () => {
    it("shows empty state when no listings", async () => {
      setupDefaults();
      mockSearch.mockResolvedValue({ listings: [], count: 0 });
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("No listings found")).toBeInTheDocument();
      });
    });

    it("shows Create Listing button in empty listings state", async () => {
      setupDefaults();
      mockSearch.mockResolvedValue({ listings: [], count: 0 });
      render(<MarketplacePage />);
      await waitFor(() => {
        const emptyState = screen.getByTestId("empty-state");
        expect(
          within(emptyState).getByText("Create Listing")
        ).toBeInTheDocument();
      });
    });

    it("shows empty orders state", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockOrdersList.mockResolvedValue({ orders: [], count: 0 });
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("Orders")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Orders"));
      await waitFor(() => {
        expect(screen.getByText("No orders yet")).toBeInTheDocument();
      });
    });

    it("shows empty disputes state", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockDisputesList.mockResolvedValue({ disputes: [], count: 0 });
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("Disputes")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Disputes"));
      await waitFor(() => {
        expect(screen.getByText("No disputes")).toBeInTheDocument();
      });
    });

    it("shows empty offers state", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockOffersList.mockResolvedValue({ offers: [], count: 0 });
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("Offers")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Offers"));
      await waitFor(() => {
        expect(screen.getByText("No offers yet")).toBeInTheDocument();
      });
    });
  });

  // ── Tab switching ─────────────────────────────────────────────────────

  describe("Tab switching", () => {
    it("switches to Orders tab", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Orders"));
      const ordersTab = screen.getByText("Orders");
      expect(ordersTab.className).toContain("border-[#d4af37]");
    });

    it("switches to Disputes tab", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Disputes"));
      const tab = screen.getByText("Disputes");
      expect(tab.className).toContain("border-[#d4af37]");
    });

    it("switches to Offers tab", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Offers"));
      const tab = screen.getByText("Offers");
      expect(tab.className).toContain("border-[#d4af37]");
    });
  });

  // ── Orders tab ────────────────────────────────────────────────────────

  describe("Orders tab", () => {
    it("renders order data in table", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Orders"));
      await waitFor(() => {
        expect(screen.getByTestId("data-table")).toBeInTheDocument();
      });
    });

    it("shows order status", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Orders"));
      await waitFor(() => {
        expect(screen.getByText("Pending")).toBeInTheDocument();
        expect(screen.getByText("Completed")).toBeInTheDocument();
      });
    });

    it("calls orders.list API", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Orders"));
      await waitFor(() => {
        expect(mockOrdersList).toHaveBeenCalled();
      });
    });
  });

  // ── Disputes tab ──────────────────────────────────────────────────────

  describe("Disputes tab", () => {
    it("renders dispute data in table", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Disputes"));
      await waitFor(() => {
        expect(screen.getByTestId("data-table")).toBeInTheDocument();
      });
    });

    it("shows dispute reason", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Disputes"));
      await waitFor(() => {
        expect(screen.getByText(/Item Not As Described/)).toBeInTheDocument();
      });
    });

    it("shows dispute status", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Disputes"));
      await waitFor(() => {
        expect(screen.getByText("Open")).toBeInTheDocument();
      });
    });

    it("shows evidence count", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Disputes"));
      await waitFor(() => {
        expect(screen.getByText("3")).toBeInTheDocument();
      });
    });
  });

  // ── Offers tab ────────────────────────────────────────────────────────

  describe("Offers tab", () => {
    it("renders offer data in table", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Offers"));
      await waitFor(() => {
        expect(screen.getByTestId("data-table")).toBeInTheDocument();
      });
    });

    it("shows offer amount", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Offers"));
      await waitFor(() => {
        expect(screen.getByText("35.00 USDC")).toBeInTheDocument();
      });
    });

    it("shows offer status", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Offers"));
      await waitFor(() => {
        // "Pending" appears in offers
        const pendingItems = screen.getAllByText("Pending");
        expect(pendingItems.length).toBeGreaterThanOrEqual(1);
      });
    });

    it("shows offer message", async () => {
      const user = userEvent.setup();
      await renderMarketplace();
      await user.click(screen.getByText("Offers"));
      await waitFor(() => {
        expect(screen.getByText(/Would you take 35/)).toBeInTheDocument();
      });
    });
  });

  // ── API integration ───────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls marketplace.search on mount", async () => {
      await renderMarketplace();
      expect(mockSearch).toHaveBeenCalled();
    });

    it("calls disputes.list for sidebar badge on mount", async () => {
      await renderMarketplace();
      await waitFor(() => {
        expect(mockDisputesList).toHaveBeenCalled();
      });
    });

    it("handles search API failure gracefully", async () => {
      setupDefaults();
      mockSearch.mockRejectedValue(new Error("Network error"));
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "API offline", variant: "error" })
        );
      });
    });

    it("handles orders API failure gracefully", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockOrdersList.mockRejectedValue(new Error("Network error"));
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("Orders")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Orders"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "API offline", variant: "error" })
        );
      });
    });

    it("handles create listing API failure gracefully", async () => {
      const user = userEvent.setup();
      setupDefaults();
      mockCreateListing.mockRejectedValue(new Error("Server error"));
      render(<MarketplacePage />);
      await waitFor(() => {
        expect(screen.getByText("New Listing")).toBeInTheDocument();
      });
      await user.click(screen.getByText("New Listing"));
      await user.type(
        screen.getByPlaceholderText("What are you selling?"),
        "Fail Item"
      );
      await user.type(screen.getByPlaceholderText("0"), "500");
      await user.click(screen.getByText("Publish"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Failed to create listing",
            variant: "error",
          })
        );
      });
    });
  });
});
