import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockHealth = vi.fn();
const mockGetWallet = vi.fn();
const mockCreateWallet = vi.fn();
const mockGetTransactions = vi.fn();
const mockTransfer = vi.fn();
const mockListInvoices = vi.fn();
const mockCreateInvoice = vi.fn();
const mockPayInvoice = vi.fn();
const mockCancelInvoice = vi.fn();
const mockCreateEscrow = vi.fn();
const mockReleaseEscrow = vi.fn();

vi.mock("@/lib/api", () => ({
  node: { health: () => mockHealth() },
  payments: {
    getWallet: (did: string) => mockGetWallet(did),
    createWallet: (did: string) => mockCreateWallet(did),
    getTransactions: (did: string, limit?: number) => mockGetTransactions(did, limit),
    transfer: (data: unknown) => mockTransfer(data),
    listInvoices: (did: string) => mockListInvoices(did),
    createInvoice: (data: unknown) => mockCreateInvoice(data),
    payInvoice: (id: string) => mockPayInvoice(id),
    cancelInvoice: (id: string) => mockCancelInvoice(id),
    createEscrow: (data: unknown) => mockCreateEscrow(data),
    releaseEscrow: (id: string, did: string) => mockReleaseEscrow(id, did),
  },
}));

const mockToast = vi.fn(() => "toast-id");

vi.mock("@/components/toast", () => ({
  useToast: () => ({ toast: mockToast, dismiss: vi.fn() }),
  ToastProvider: ({ children }: { children: React.ReactNode }) => children,
}));

vi.mock("@/components/page-header", () => ({
  PageHeader: ({ title, subtitle, status }: { title: string; subtitle: string; status?: string }) => (
    <div data-testid="page-header" data-status={status}>
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({ title, description, action }: { title: string; description: string; action?: React.ReactNode }) => (
    <div data-testid="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  ),
  WalletIllustration: () => <div data-testid="wallet-illustration" />,
  TransactionsIllustration: () => <div data-testid="transactions-illustration" />,
  InvoiceIllustration: () => <div data-testid="invoice-illustration" />,
  EscrowIllustration: () => <div data-testid="escrow-illustration" />,
}));

vi.mock("@/components/sidebar", () => ({
  setNavBadge: vi.fn(),
}));

vi.mock("@/components/keyboard-shortcuts", () => ({
  usePageShortcuts: vi.fn(),
}));

vi.mock("@/components/wallet-chart", () => ({
  WalletChart: () => <div data-testid="wallet-chart">Chart</div>,
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: { children: React.ReactNode; open: boolean }) =>
    open ? <div data-testid="dialog">{children}</div> : null,
  DialogHeader: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  DialogTitle: ({ children }: { children: React.ReactNode }) => <h2>{children}</h2>,
  DialogDescription: ({ children }: { children: React.ReactNode }) => <p>{children}</p>,
  DialogBody: ({ children, className }: { children: React.ReactNode; className?: string }) => <div className={className}>{children}</div>,
  DialogFooter: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/components/ui/input", () => ({
  Input: ({ label, ...props }: { label?: string; [key: string]: unknown }) => (
    <div>
      {label && <label>{label}</label>}
      <input aria-label={label} {...props} />
    </div>
  ),
}));

vi.mock("@/components/ui/textarea", () => ({
  Textarea: ({ label, ...props }: { label?: string; [key: string]: unknown }) => (
    <div>
      {label && <label>{label}</label>}
      <textarea aria-label={label} {...props} />
    </div>
  ),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({ label, value, onValueChange, options }: { label?: string; value: string; onValueChange: (v: string) => void; options: { value: string; label: string }[] }) => (
    <div>
      {label && <label>{label}</label>}
      <select aria-label={label} value={value} onChange={(e) => onValueChange(e.target.value)}>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
    </div>
  ),
}));

import WalletPage from "@/app/(app)/wallet/page";

// ── Fixtures ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc";
const OTHER_DID = "did:key:z6MkpTHR8VNs0xo2UQc5bgdXKPaeq9a3gLsLe2QHMHogNxRR";

const MOCK_WALLET = {
  did: MOCK_DID,
  balances: [
    { token: "ETH", amount: "1.5" },
    { token: "NOUS", amount: "42000" },
    { token: "USDC", amount: "250.75" },
  ],
  nonce: 5,
  created_at: "2026-01-01T00:00:00Z",
};

const MOCK_TRANSACTIONS = [
  {
    id: "tx-1",
    from_did: MOCK_DID,
    to_did: OTHER_DID,
    token: "ETH",
    amount: "0.5",
    fee: "0.001",
    memo: "Payment for services",
    status: "confirmed",
    timestamp: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    id: "tx-2",
    from_did: OTHER_DID,
    to_did: MOCK_DID,
    token: "NOUS",
    amount: "1000",
    fee: "0",
    memo: null,
    status: "confirmed",
    timestamp: new Date(Date.now() - 7200000).toISOString(),
  },
];

const MOCK_INVOICES = [
  {
    id: "inv-1",
    from_did: MOCK_DID,
    to_did: OTHER_DID,
    token: "NOUS",
    total: "500",
    status: "pending",
    memo: "Consulting work",
    items: [
      { description: "Design review", quantity: 2, unit_price: "250", total: "500" },
    ],
    created_at: "2026-03-01T00:00:00Z",
    due_at: "2026-04-01T00:00:00Z",
    paid_at: null,
  },
  {
    id: "inv-2",
    from_did: OTHER_DID,
    to_did: MOCK_DID,
    token: "ETH",
    total: "1.0",
    status: "pending",
    memo: null,
    items: [
      { description: "Widget", quantity: 1, unit_price: "1.0", total: "1.0" },
    ],
    created_at: "2026-03-10T00:00:00Z",
    due_at: "2026-04-10T00:00:00Z",
    paid_at: null,
  },
];

// ── Helpers ──────────────────────────────────────────────────────────────

function setupMocks() {
  mockHealth.mockResolvedValue({ status: "healthy", version: "0.4.0", uptime_ms: 100000 });
  mockGetWallet.mockResolvedValue(MOCK_WALLET);
  mockCreateWallet.mockResolvedValue(MOCK_WALLET);
  mockGetTransactions.mockResolvedValue(MOCK_TRANSACTIONS);
  mockTransfer.mockResolvedValue({ id: "tx-new" });
  mockListInvoices.mockResolvedValue(MOCK_INVOICES);
  mockCreateInvoice.mockResolvedValue({ id: "inv-new" });
  mockPayInvoice.mockResolvedValue({ id: "inv-2", status: "paid" });
  mockCancelInvoice.mockResolvedValue({ id: "inv-1", status: "cancelled" });
  mockCreateEscrow.mockResolvedValue({ id: "esc-new" });
  mockReleaseEscrow.mockResolvedValue({ id: "esc-1", status: "released" });
}

async function renderWallet(did?: string) {
  if (did) localStorage.setItem("nous_did", did);
  render(<WalletPage />);
  // Wait for loading to finish
  await screen.findByText("Balances");
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Wallet page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setupMocks();
  });

  // ── Page structure ─────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header", async () => {
      await renderWallet(MOCK_DID);
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Wallet")).toBeInTheDocument();
      expect(within(header).getByText(/Multi-chain/)).toBeInTheDocument();
    });

    it("shows online status when node is healthy", async () => {
      await renderWallet(MOCK_DID);
      const header = screen.getByTestId("page-header");
      expect(header).toHaveAttribute("data-status", "online");
    });

    it("shows offline status when node health fails", async () => {
      mockHealth.mockRejectedValue(new Error("offline"));
      await renderWallet(MOCK_DID);
      const header = screen.getByTestId("page-header");
      expect(header).toHaveAttribute("data-status", "offline");
    });

    it("renders three tab buttons", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText("balances")).toBeInTheDocument();
      expect(screen.getByText("invoices")).toBeInTheDocument();
      expect(screen.getByText("escrow")).toBeInTheDocument();
    });

    it("defaults to balances tab", async () => {
      await renderWallet(MOCK_DID);
      const balancesBtn = screen.getByText("balances");
      expect(balancesBtn.className).toContain("d4af37");
    });
  });

  // ── Balances tab ───────────────────────────────────────────────────────

  describe("Balances tab", () => {
    it("renders balance cards for each token", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText("ETH")).toBeInTheDocument();
      expect(screen.getByText("1.5")).toBeInTheDocument();
      expect(screen.getByText("NOUS")).toBeInTheDocument();
      expect(screen.getByText("42000")).toBeInTheDocument();
      expect(screen.getByText("USDC")).toBeInTheDocument();
      expect(screen.getByText("250.75")).toBeInTheDocument();
    });

    it("renders WalletChart", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByTestId("wallet-chart")).toBeInTheDocument();
    });

    it("renders Send, Receive, Swap buttons", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText("Send")).toBeInTheDocument();
      expect(screen.getByText("Receive")).toBeInTheDocument();
      expect(screen.getByText("Swap")).toBeInTheDocument();
    });

    it("renders Transactions section", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText("Transactions")).toBeInTheDocument();
    });

    it("shows transaction history", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText(/Sent 0.5 ETH/)).toBeInTheDocument();
      expect(screen.getByText(/Received 1000 NOUS/)).toBeInTheDocument();
    });

    it("shows transaction memos", async () => {
      await renderWallet(MOCK_DID);
      expect(screen.getByText("Payment for services")).toBeInTheDocument();
    });

    it("shows transaction status", async () => {
      await renderWallet(MOCK_DID);
      const confirmed = screen.getAllByText("confirmed");
      expect(confirmed.length).toBeGreaterThanOrEqual(2);
    });

    it("shows no wallet empty state when wallet does not exist", async () => {
      mockGetWallet.mockRejectedValue(new Error("not found"));
      mockGetTransactions.mockRejectedValue(new Error("not found"));
      await renderWallet(MOCK_DID);
      expect(screen.getByText("No wallet found")).toBeInTheDocument();
      expect(screen.getByText("Create Wallet")).toBeInTheDocument();
    });

    it("shows empty transactions state when no transactions", async () => {
      mockGetTransactions.mockResolvedValue([]);
      await renderWallet(MOCK_DID);
      expect(screen.getByText("No transactions yet")).toBeInTheDocument();
    });
  });

  // ── Create wallet ──────────────────────────────────────────────────────

  describe("Create wallet", () => {
    it("calls createWallet API on Create Wallet click", async () => {
      mockGetWallet.mockRejectedValue(new Error("not found"));
      mockGetTransactions.mockRejectedValue(new Error("not found"));
      await renderWallet(MOCK_DID);
      const user = userEvent.setup();
      await user.click(screen.getByText("Create Wallet"));
      await waitFor(() => {
        expect(mockCreateWallet).toHaveBeenCalledWith(MOCK_DID);
      });
    });

    it("shows success toast on wallet creation", async () => {
      mockGetWallet.mockRejectedValue(new Error("not found"));
      mockGetTransactions.mockRejectedValue(new Error("not found"));
      await renderWallet(MOCK_DID);
      const user = userEvent.setup();
      await user.click(screen.getByText("Create Wallet"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Wallet created", variant: "success" }),
        );
      });
    });
  });

  // ── Send modal ─────────────────────────────────────────────────────────

  describe("Send modal", () => {
    it("opens send modal on Send click", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));
      expect(screen.getByTestId("dialog")).toBeInTheDocument();
      expect(screen.getByText("Transfer tokens to another identity")).toBeInTheDocument();
    });

    it("renders send form fields", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));
      expect(screen.getByLabelText("Recipient DID")).toBeInTheDocument();
      expect(screen.getByLabelText("Amount")).toBeInTheDocument();
      expect(screen.getByLabelText("Token")).toBeInTheDocument();
      expect(screen.getByLabelText("Memo")).toBeInTheDocument();
    });

    it("renders Confirm Send and Cancel buttons", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));
      expect(screen.getByText("Confirm Send")).toBeInTheDocument();
      // Cancel appears in the dialog
      expect(screen.getAllByText("Cancel").length).toBeGreaterThanOrEqual(1);
    });

    it("calls transfer API on Confirm Send", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));

      await user.type(screen.getByLabelText("Recipient DID"), OTHER_DID);
      await user.type(screen.getByLabelText("Amount"), "0.1");

      await user.click(screen.getByText("Confirm Send"));
      await waitFor(() => {
        expect(mockTransfer).toHaveBeenCalledWith(
          expect.objectContaining({
            from_did: MOCK_DID,
            to_did: OTHER_DID,
            token: "ETH",
            amount: 0.1,
          }),
        );
      });
    });

    it("shows success toast on successful transfer", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));
      await user.type(screen.getByLabelText("Recipient DID"), OTHER_DID);
      await user.type(screen.getByLabelText("Amount"), "0.1");
      await user.click(screen.getByText("Confirm Send"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Transfer sent", variant: "success" }),
        );
      });
    });

    it("shows error toast on transfer failure", async () => {
      mockTransfer.mockRejectedValue(new Error("Insufficient funds"));
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("Send"));
      await user.type(screen.getByLabelText("Recipient DID"), OTHER_DID);
      await user.type(screen.getByLabelText("Amount"), "999");
      await user.click(screen.getByText("Confirm Send"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Transfer failed", variant: "error" }),
        );
      });
    });
  });

  // ── Invoices tab ───────────────────────────────────────────────────────

  describe("Invoices tab", () => {
    it("switches to invoices tab", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      expect(screen.getByText("Invoices")).toBeInTheDocument();
    });

    it("shows invoice list", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      expect(screen.getByText("500 NOUS")).toBeInTheDocument();
      expect(screen.getByText("1.0 ETH")).toBeInTheDocument();
    });

    it("shows invoice status", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      const pendingTexts = screen.getAllByText("pending");
      expect(pendingTexts.length).toBeGreaterThanOrEqual(2);
    });

    it("shows invoice memo", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      expect(screen.getByText("Consulting work")).toBeInTheDocument();
    });

    it("shows Cancel button for invoices user issued", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      // inv-1 is from MOCK_DID (issuer), so Cancel should appear
      const cancelButtons = screen.getAllByText("Cancel");
      expect(cancelButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("shows Pay button for invoices addressed to user", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      // inv-2 is from OTHER_DID to MOCK_DID, so Pay should appear
      expect(screen.getByText("Pay")).toBeInTheDocument();
    });

    it("shows Create Invoice button", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      const createBtns = screen.getAllByText("Create Invoice");
      expect(createBtns.length).toBeGreaterThanOrEqual(1);
    });

    it("shows empty invoices state when none exist", async () => {
      mockListInvoices.mockResolvedValue([]);
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      expect(screen.getByText("No invoices yet")).toBeInTheDocument();
    });

    it("calls payInvoice on Pay click", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      await user.click(screen.getByText("Pay"));
      await waitFor(() => {
        expect(mockPayInvoice).toHaveBeenCalledWith("inv-2");
      });
    });

    it("shows success toast on invoice payment", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      await user.click(screen.getByText("Pay"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Invoice paid", variant: "success" }),
        );
      });
    });

    it("calls cancelInvoice on Cancel click in invoices", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("invoices"));
      // Get the Cancel button that's for invoices (not the dialog cancel)
      const cancelButtons = screen.getAllByText("Cancel");
      // The first Cancel should be the invoice cancel (not dialog)
      await user.click(cancelButtons[0]);
      await waitFor(() => {
        expect(mockCancelInvoice).toHaveBeenCalledWith("inv-1");
      });
    });
  });

  // ── Escrow tab ─────────────────────────────────────────────────────────

  describe("Escrow tab", () => {
    it("switches to escrow tab", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("escrow"));
      expect(screen.getByText("Escrow")).toBeInTheDocument();
    });

    it("shows empty escrow state", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("escrow"));
      expect(screen.getByText("No active escrows")).toBeInTheDocument();
    });

    it("shows Create Escrow button", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("escrow"));
      const createBtns = screen.getAllByText("Create Escrow");
      expect(createBtns.length).toBeGreaterThanOrEqual(1);
    });

    it("opens escrow dialog on Create Escrow click", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("escrow"));
      // Click the first Create Escrow (either link or empty state button)
      const createBtns = screen.getAllByText("Create Escrow");
      await user.click(createBtns[0]);
      expect(screen.getByTestId("dialog")).toBeInTheDocument();
      expect(screen.getByText("New Escrow")).toBeInTheDocument();
    });

    it("renders escrow form fields", async () => {
      const user = userEvent.setup();
      await renderWallet(MOCK_DID);
      await user.click(screen.getByText("escrow"));
      const createBtns = screen.getAllByText("Create Escrow");
      await user.click(createBtns[0]);
      expect(screen.getByLabelText("Seller DID")).toBeInTheDocument();
      expect(screen.getByLabelText("Amount")).toBeInTheDocument();
      expect(screen.getByLabelText("Description")).toBeInTheDocument();
      expect(screen.getByLabelText("Conditions")).toBeInTheDocument();
    });
  });

  // ── API integration ────────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls node.health on mount", async () => {
      await renderWallet(MOCK_DID);
      expect(mockHealth).toHaveBeenCalled();
    });

    it("calls payments.getWallet with DID", async () => {
      await renderWallet(MOCK_DID);
      expect(mockGetWallet).toHaveBeenCalledWith(MOCK_DID);
    });

    it("calls payments.getTransactions with DID and limit", async () => {
      await renderWallet(MOCK_DID);
      expect(mockGetTransactions).toHaveBeenCalledWith(MOCK_DID, 50);
    });

    it("calls payments.listInvoices with DID", async () => {
      await renderWallet(MOCK_DID);
      expect(mockListInvoices).toHaveBeenCalledWith(MOCK_DID);
    });

    it("does not call wallet APIs when no DID", async () => {
      mockGetWallet.mockRejectedValue(new Error("not found"));
      mockGetTransactions.mockRejectedValue(new Error("not found"));
      mockListInvoices.mockResolvedValue([]);
      localStorage.removeItem("nous_did");
      render(<WalletPage />);
      // Wait for loading to finish
      await waitFor(() => {
        expect(mockGetWallet).not.toHaveBeenCalled();
      });
    });

    it("handles wallet API failure gracefully", async () => {
      mockGetWallet.mockRejectedValue(new Error("fail"));
      mockGetTransactions.mockRejectedValue(new Error("fail"));
      await renderWallet(MOCK_DID);
      // Should show create wallet empty state without crashing
      expect(screen.getByText("No wallet found")).toBeInTheDocument();
    });
  });
});
