import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockIdentityCreate = vi.fn();
const mockIdentityGet = vi.fn();
const mockListCredentials = vi.fn();
const mockGetReputation = vi.fn();

vi.mock("@/lib/api", () => ({
  identity: {
    create: (name?: string) => mockIdentityCreate(name),
    get: (did: string) => mockIdentityGet(did),
    listCredentials: (did: string) => mockListCredentials(did),
    getReputation: (did: string) => mockGetReputation(did),
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
  CredentialIllustration: () => <div data-testid="credential-illustration" />,
  IdentityKeyIllustration: () => <div data-testid="identity-key-illustration" />,
}));

vi.mock("@/components/did-avatar", () => ({
  DidAvatar: ({ did }: { did: string }) => (
    <div data-testid="did-avatar" data-did={did} />
  ),
  DidAvatarLarge: ({ did }: { did: string }) => (
    <div data-testid="did-avatar-large" data-did={did} />
  ),
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({
    children,
    content,
  }: {
    children: React.ReactNode;
    content: unknown;
  }) => (
    <span title={typeof content === "string" ? content : "tooltip"}>
      {children}
    </span>
  ),
}));

// ── Test data ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

const MOCK_IDENTITY = {
  did: MOCK_DID,
  display_name: "Teddy",
  signing_key_type: "Ed25519",
  exchange_key_type: "X25519",
};

const MOCK_CREDENTIALS = [
  {
    id: "cred-1",
    credential_type: ["VerifiableCredential", "EmailCredential"],
    issuer: "did:key:z6MkissuerAbcdef1234567890abcdef1234567890abc",
    subject: MOCK_DID,
    issuance_date: "2026-01-15T00:00:00Z",
    expiration_date: "2027-01-15T00:00:00Z",
    expired: false,
    claims: { email: "teddy@nous.sh", verified: true },
  },
  {
    id: "cred-2",
    credential_type: ["VerifiableCredential", "AgeCredential"],
    issuer: "did:key:z6MkissuerXyz0987654321xyz0987654321xyz09876",
    subject: MOCK_DID,
    issuance_date: "2025-06-01T00:00:00Z",
    expiration_date: "2025-12-01T00:00:00Z",
    expired: true,
    claims: { over_18: true },
  },
];

const MOCK_REPUTATION = {
  did: MOCK_DID,
  total_score: 42,
  scores: { governance: 15, social: 12, commerce: 10, identity: 5 },
  event_count: 27,
};

// ── Helpers ──────────────────────────────────────────────────────────────

import IdentityPage from "@/app/(app)/identity/page";

function setupDefaults(hasDid = true, displayName: string | null = "Teddy") {
  mockIdentityGet.mockResolvedValue(MOCK_IDENTITY);
  mockListCredentials.mockResolvedValue(MOCK_CREDENTIALS);
  mockGetReputation.mockResolvedValue(MOCK_REPUTATION);
  mockIdentityCreate.mockResolvedValue(MOCK_IDENTITY);

  Object.defineProperty(window, "localStorage", {
    value: {
      getItem: vi.fn((key: string) => {
        if (key === "nous_did") return hasDid ? MOCK_DID : null;
        if (key === "nous_display_name") return displayName;
        return null;
      }),
      setItem: vi.fn(),
      removeItem: vi.fn(),
    },
    writable: true,
  });
}

async function renderIdentity(hasDid = true) {
  setupDefaults(hasDid);
  render(<IdentityPage />);
  await waitFor(() => {
    expect(screen.getByTestId("page-header")).toBeInTheDocument();
  });
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Identity page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Mock clipboard
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      writable: true,
      configurable: true,
    });
  });

  // ── Page structure ────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header", async () => {
      await renderIdentity();
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Identity")).toBeInTheDocument();
      expect(
        within(header).getByText(
          "Self-sovereign. Your keys, your identity."
        )
      ).toBeInTheDocument();
    });
  });

  // ── No identity state ─────────────────────────────────────────────────

  describe("No identity", () => {
    it("shows generate identity form when no DID", async () => {
      await renderIdentity(false);
      expect(screen.getByText("Generate your identity")).toBeInTheDocument();
    });

    it("shows display name input", async () => {
      await renderIdentity(false);
      expect(
        screen.getByPlaceholderText("Display name (optional)")
      ).toBeInTheDocument();
    });

    it("shows Generate DID button", async () => {
      await renderIdentity(false);
      expect(screen.getByText("Generate DID")).toBeInTheDocument();
    });

    it("calls identity.create on Generate DID click", async () => {
      const user = userEvent.setup();
      await renderIdentity(false);
      await user.click(screen.getByText("Generate DID"));
      await waitFor(() => {
        expect(mockIdentityCreate).toHaveBeenCalledWith(undefined);
      });
    });

    it("calls identity.create with display name", async () => {
      const user = userEvent.setup();
      await renderIdentity(false);
      const input = screen.getByPlaceholderText("Display name (optional)");
      await user.type(input, "Alice");
      await user.click(screen.getByText("Generate DID"));
      await waitFor(() => {
        expect(mockIdentityCreate).toHaveBeenCalledWith("Alice");
      });
    });

    it("shows success toast after creating identity", async () => {
      const user = userEvent.setup();
      await renderIdentity(false);
      await user.click(screen.getByText("Generate DID"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Identity created",
            variant: "success",
          })
        );
      });
    });

    it("shows error toast on create failure", async () => {
      const user = userEvent.setup();
      setupDefaults(false);
      mockIdentityCreate.mockRejectedValue(new Error("Crypto error"));
      render(<IdentityPage />);
      await waitFor(() => {
        expect(screen.getByText("Generate DID")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Generate DID"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Failed to create identity",
            variant: "error",
          })
        );
      });
    });
  });

  // ── Profile card ──────────────────────────────────────────────────────

  describe("Profile card", () => {
    it("shows display name", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Teddy")).toBeInTheDocument();
      });
    });

    it("shows avatar", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByTestId("did-avatar-large")).toBeInTheDocument();
      });
    });

    it("shows truncated DID", async () => {
      await renderIdentity();
      await waitFor(() => {
        // Truncated DID should be present
        const code = document.querySelector("code");
        expect(code?.textContent).toContain("did:key:");
        expect(code?.textContent).toContain("...");
      });
    });

    it("renders Share DID button", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Share DID")).toBeInTheDocument();
      });
    });

    it("renders Switch Identity button", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Switch Identity")).toBeInTheDocument();
      });
    });

    it("shows toast on Share DID click", async () => {
      const user = userEvent.setup();
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Share DID")).toBeInTheDocument();
      });
      await user.click(screen.getByText("Share DID"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "DID copied to clipboard" })
        );
      });
    });

    it("shows edit display name button", async () => {
      await renderIdentity();
      await waitFor(() => {
        const editBtn = screen.getByLabelText("Edit display name");
        expect(editBtn).toBeInTheDocument();
      });
    });
  });

  // ── Stats ─────────────────────────────────────────────────────────────

  describe("Stats overview", () => {
    it("shows reputation score", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("42")).toBeInTheDocument();
      });
    });

    it("shows event count", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("27 events")).toBeInTheDocument();
      });
    });

    it("shows credential count", async () => {
      await renderIdentity();
      await waitFor(() => {
        // 2 credentials total, 1 active
        expect(screen.getByText("1 active")).toBeInTheDocument();
      });
    });

    it("shows key pair count", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("signing + exchange")).toBeInTheDocument();
      });
    });
  });

  // ── Key pairs ─────────────────────────────────────────────────────────

  describe("Key pairs", () => {
    it("shows Key Pairs section heading", async () => {
      await renderIdentity();
      await waitFor(() => {
        // "Key Pairs" appears both as section heading and stat label; verify section exists
        const keyPairsTexts = screen.getAllByText("Key Pairs");
        expect(keyPairsTexts.length).toBeGreaterThanOrEqual(1);
      });
    });

    it("shows signing key type", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Ed25519")).toBeInTheDocument();
      });
    });

    it("shows exchange key type", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("X25519")).toBeInTheDocument();
      });
    });

    it("shows Signing and Exchange labels", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Signing")).toBeInTheDocument();
        expect(screen.getByText("Exchange")).toBeInTheDocument();
      });
    });
  });

  // ── Reputation breakdown ──────────────────────────────────────────────

  describe("Reputation breakdown", () => {
    it("shows breakdown section", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Reputation Breakdown")).toBeInTheDocument();
      });
    });

    it("shows reputation categories", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("governance")).toBeInTheDocument();
        expect(screen.getByText("social")).toBeInTheDocument();
        expect(screen.getByText("commerce")).toBeInTheDocument();
        expect(screen.getByText("identity")).toBeInTheDocument();
      });
    });

    it("shows category scores", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("15")).toBeInTheDocument();
        expect(screen.getByText("12")).toBeInTheDocument();
        expect(screen.getByText("10")).toBeInTheDocument();
        expect(screen.getByText("5")).toBeInTheDocument();
      });
    });
  });

  // ── Credentials ───────────────────────────────────────────────────────

  describe("Credentials", () => {
    it("shows Verifiable Credentials section", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(
          screen.getByText("Verifiable Credentials")
        ).toBeInTheDocument();
      });
    });

    it("shows credential types", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(
          screen.getByText("VerifiableCredential, EmailCredential")
        ).toBeInTheDocument();
        expect(
          screen.getByText("VerifiableCredential, AgeCredential")
        ).toBeInTheDocument();
      });
    });

    it("shows valid/expired status", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("valid")).toBeInTheDocument();
        expect(screen.getByText("expired")).toBeInTheDocument();
      });
    });

    it("shows credential claims", async () => {
      await renderIdentity();
      await waitFor(() => {
        // Claims are rendered as JSON
        expect(screen.getByText(/"email": "teddy@nous.sh"/)).toBeInTheDocument();
      });
    });

    it("shows empty credentials state when none", async () => {
      setupDefaults();
      mockListCredentials.mockResolvedValue([]);
      render(<IdentityPage />);
      await waitFor(() => {
        expect(screen.getByText("No credentials yet")).toBeInTheDocument();
      });
    });

    it("shows learn about credentials link", async () => {
      setupDefaults();
      mockListCredentials.mockResolvedValue([]);
      render(<IdentityPage />);
      await waitFor(() => {
        expect(
          screen.getByText("Learn about credentials")
        ).toBeInTheDocument();
      });
    });
  });

  // ── DID document ──────────────────────────────────────────────────────

  describe("DID document", () => {
    it("shows Full Identifier section", async () => {
      await renderIdentity();
      await waitFor(() => {
        expect(screen.getByText("Full Identifier")).toBeInTheDocument();
      });
    });

    it("shows full DID string", async () => {
      await renderIdentity();
      await waitFor(() => {
        const didText = screen.getByText(MOCK_DID);
        expect(didText).toBeInTheDocument();
      });
    });
  });

  // ── API integration ───────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls identity.get with stored DID", async () => {
      await renderIdentity();
      expect(mockIdentityGet).toHaveBeenCalledWith(MOCK_DID);
    });

    it("calls identity.listCredentials with stored DID", async () => {
      await renderIdentity();
      expect(mockListCredentials).toHaveBeenCalledWith(MOCK_DID);
    });

    it("calls identity.getReputation with stored DID", async () => {
      await renderIdentity();
      expect(mockGetReputation).toHaveBeenCalledWith(MOCK_DID);
    });

    it("does not call APIs when no DID stored", async () => {
      await renderIdentity(false);
      expect(mockIdentityGet).not.toHaveBeenCalled();
      expect(mockListCredentials).not.toHaveBeenCalled();
      expect(mockGetReputation).not.toHaveBeenCalled();
    });

    it("handles identity.get failure gracefully", async () => {
      setupDefaults();
      mockIdentityGet.mockRejectedValue(new Error("Not found"));
      render(<IdentityPage />);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Failed to load identity",
            variant: "error",
          })
        );
      });
    });
  });
});
