import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockHealth = vi.fn();
const mockIdentityGet = vi.fn();
const mockListCredentials = vi.fn();
const mockGetReputation = vi.fn();
const mockGetDocument = vi.fn();

vi.mock("@/lib/api", () => ({
  node: { health: () => mockHealth() },
  identity: {
    get: (did: string) => mockIdentityGet(did),
    listCredentials: (did: string) => mockListCredentials(did),
    getReputation: (did: string) => mockGetReputation(did),
    getDocument: (did: string) => mockGetDocument(did),
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

const mockResetTour = vi.fn();
vi.mock("@/components/product-tour", () => ({
  resetTour: () => mockResetTour(),
  ProductTour: () => null,
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children, content }: { children: React.ReactNode; content: string }) => (
    <span data-testid="tooltip" data-content={content}>{children}</span>
  ),
}));

import SettingsPage from "@/app/(app)/settings/page";

// ── Fixtures ────────────────────────────────────────────────────────────

const MOCK_HEALTH = { status: "healthy", version: "0.4.0", uptime_ms: 360000 };
const MOCK_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc";
const MOCK_IDENTITY = {
  did: MOCK_DID,
  display_name: "Alice",
  signing_key_type: "Ed25519",
  exchange_key_type: "X25519",
};
const MOCK_CREDENTIALS = [
  {
    id: "cred-1",
    credential_type: ["VerifiableCredential", "MembershipCredential"],
    issuer: "did:key:z6MkpTHR8VNs0xo2UQc5bgdXKPaeq9a3gLsLe2QHMHogNxRR",
    subject: MOCK_DID,
    issuance_date: "2026-01-15T00:00:00Z",
    expiration_date: "2027-01-15T00:00:00Z",
    expired: false,
    claims: { role: "admin", level: "3" },
  },
  {
    id: "cred-2",
    credential_type: ["VerifiableCredential", "AgeCredential"],
    issuer: "did:key:z6MkrZ7USjdrPj9oCCz8X9p2vWtdHfG3w5VU",
    subject: MOCK_DID,
    issuance_date: "2025-06-01T00:00:00Z",
    expiration_date: "2025-12-01T00:00:00Z",
    expired: true,
    claims: { over18: "true" },
  },
];
const MOCK_REPUTATION = {
  did: MOCK_DID,
  total_score: 1250,
  scores: { governance: 400, social: 500, marketplace: 350 },
  event_count: 42,
};

// ── Helpers ──────────────────────────────────────────────────────────────

function setupDefaultMocks() {
  mockHealth.mockResolvedValue(MOCK_HEALTH);
  mockIdentityGet.mockResolvedValue(MOCK_IDENTITY);
  mockListCredentials.mockResolvedValue(MOCK_CREDENTIALS);
  mockGetReputation.mockResolvedValue(MOCK_REPUTATION);
  mockGetDocument.mockResolvedValue({ did: MOCK_DID, document: { id: MOCK_DID, verificationMethod: [] } });
}

function setupEmptyMocks() {
  mockHealth.mockResolvedValue(MOCK_HEALTH);
  mockIdentityGet.mockRejectedValue(new Error("not found"));
  mockListCredentials.mockRejectedValue(new Error("not found"));
  mockGetReputation.mockRejectedValue(new Error("not found"));
}

async function renderSettings(did?: string) {
  if (did) {
    localStorage.setItem("nous_did", did);
    localStorage.setItem("nous_display_name", "Alice");
  }
  const result = render(<SettingsPage />);
  // Wait for loading to finish — the "Identity" section heading only appears after load
  await screen.findByText("Identity");
  return result;
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Settings page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setupDefaultMocks();
  });

  // ── Loading state ──────────────────────────────────────────────────────

  describe("Loading state", () => {
    it("shows skeleton screens while loading", () => {
      mockHealth.mockReturnValue(new Promise(() => {})); // never resolves
      render(<SettingsPage />);
      const skeletons = document.querySelectorAll('[class*="animate-pulse"], [data-slot="skeleton"]');
      expect(skeletons.length).toBeGreaterThan(0);
    });

    it("shows page header during loading", () => {
      mockHealth.mockReturnValue(new Promise(() => {}));
      render(<SettingsPage />);
      expect(screen.getByText("Settings")).toBeInTheDocument();
    });
  });

  // ── Page header ────────────────────────────────────────────────────────

  describe("Page header", () => {
    it("renders title and subtitle", async () => {
      await renderSettings();
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Settings")).toBeInTheDocument();
      expect(within(header).getByText("Identity, credentials, and preferences")).toBeInTheDocument();
    });

    it("shows online status when node is healthy", async () => {
      await renderSettings();
      const header = screen.getByTestId("page-header");
      expect(header).toHaveAttribute("data-status", "online");
    });

    it("shows offline status when node health fails", async () => {
      mockHealth.mockRejectedValue(new Error("connection refused"));
      await renderSettings();
      const header = screen.getByTestId("page-header");
      expect(header).toHaveAttribute("data-status", "offline");
    });
  });

  // ── Identity section ───────────────────────────────────────────────────

  describe("Identity section", () => {
    it("renders Identity heading", async () => {
      await renderSettings();
      expect(screen.getByText("Identity")).toBeInTheDocument();
    });

    it("renders DID input with placeholder", async () => {
      await renderSettings();
      expect(screen.getByPlaceholderText("did:key:z...")).toBeInTheDocument();
    });

    it("renders Display Name input with placeholder", async () => {
      await renderSettings();
      expect(screen.getByPlaceholderText("Anonymous")).toBeInTheDocument();
    });

    it("populates DID from localStorage", async () => {
      await renderSettings(MOCK_DID);
      const input = screen.getByPlaceholderText("did:key:z...");
      expect(input).toHaveValue(MOCK_DID);
    });

    it("populates display name from localStorage", async () => {
      await renderSettings(MOCK_DID);
      const input = screen.getByPlaceholderText("Anonymous");
      expect(input).toHaveValue("Alice");
    });

    it("shows identity verified text when identity exists", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Identity verified on node")).toBeInTheDocument();
    });

    it("shows signing and exchange key types", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Ed25519")).toBeInTheDocument();
      expect(screen.getByText("X25519")).toBeInTheDocument();
    });

    it("shows Export DID Document button when DID is set", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Export DID Document")).toBeInTheDocument();
    });

    it("hides Export DID Document button when no DID", async () => {
      await renderSettings();
      expect(screen.queryByText("Export DID Document")).not.toBeInTheDocument();
    });

    it("calls identity.get with the stored DID", async () => {
      await renderSettings(MOCK_DID);
      expect(mockIdentityGet).toHaveBeenCalledWith(MOCK_DID);
    });

    it("does not call identity.get when no DID stored", async () => {
      await renderSettings();
      expect(mockIdentityGet).not.toHaveBeenCalled();
    });

    it("exports DID document on button click", async () => {
      const user = userEvent.setup();
      const mockClick = vi.fn();
      const mockCreateObjectURL = vi.fn(() => "blob:test");
      const mockRevokeObjectURL = vi.fn();
      const origCreateElement = document.createElement.bind(document);
      vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
        const el = origCreateElement(tag);
        if (tag === "a") {
          Object.defineProperty(el, "click", { value: mockClick });
        }
        return el;
      });
      vi.spyOn(URL, "createObjectURL").mockImplementation(mockCreateObjectURL);
      vi.spyOn(URL, "revokeObjectURL").mockImplementation(mockRevokeObjectURL);

      await renderSettings(MOCK_DID);
      await user.click(screen.getByText("Export DID Document"));

      await waitFor(() => {
        expect(mockGetDocument).toHaveBeenCalledWith(MOCK_DID);
        expect(mockClick).toHaveBeenCalled();
        expect(mockRevokeObjectURL).toHaveBeenCalled();
      });

      vi.restoreAllMocks();
    });
  });

  // ── Reputation section ─────────────────────────────────────────────────

  describe("Reputation section", () => {
    it("renders Reputation heading when data exists", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Reputation")).toBeInTheDocument();
    });

    it("shows total score and event count", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("1250")).toBeInTheDocument();
      expect(screen.getByText(/42 events/)).toBeInTheDocument();
    });

    it("shows category scores", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("governance")).toBeInTheDocument();
      expect(screen.getByText("400")).toBeInTheDocument();
      expect(screen.getByText("social")).toBeInTheDocument();
      expect(screen.getByText("500")).toBeInTheDocument();
      expect(screen.getByText("marketplace")).toBeInTheDocument();
      expect(screen.getByText("350")).toBeInTheDocument();
    });

    it("hides Reputation section when no data", async () => {
      setupEmptyMocks();
      await renderSettings(MOCK_DID);
      expect(screen.queryByText("Reputation")).not.toBeInTheDocument();
    });

    it("hides Reputation section when event_count is 0", async () => {
      mockGetReputation.mockResolvedValue({ ...MOCK_REPUTATION, event_count: 0 });
      await renderSettings(MOCK_DID);
      expect(screen.queryByText("Reputation")).not.toBeInTheDocument();
    });
  });

  // ── Credentials section ────────────────────────────────────────────────

  describe("Credentials section", () => {
    it("renders Credentials heading with count", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Credentials")).toBeInTheDocument();
      expect(screen.getByText("2")).toBeInTheDocument();
    });

    it("shows credential types", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("VerifiableCredential, MembershipCredential")).toBeInTheDocument();
      expect(screen.getByText("VerifiableCredential, AgeCredential")).toBeInTheDocument();
    });

    it("shows Valid status for non-expired credentials", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Valid")).toBeInTheDocument();
    });

    it("shows Expired status for expired credentials", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("Expired")).toBeInTheDocument();
    });

    it("shows credential claims", async () => {
      await renderSettings(MOCK_DID);
      expect(screen.getByText("role:")).toBeInTheDocument();
      expect(screen.getByText("admin")).toBeInTheDocument();
      expect(screen.getByText("level:")).toBeInTheDocument();
      expect(screen.getByText("3")).toBeInTheDocument();
    });

    it("shows issuer with tooltip for long DIDs", async () => {
      await renderSettings(MOCK_DID);
      const tooltips = screen.getAllByTestId("tooltip");
      const issuerTooltips = tooltips.filter(
        (t) => t.getAttribute("data-content")?.startsWith("did:key:"),
      );
      expect(issuerTooltips.length).toBeGreaterThan(0);
    });

    it("hides Credentials section when no credentials", async () => {
      mockListCredentials.mockResolvedValue([]);
      await renderSettings(MOCK_DID);
      expect(screen.queryByText("Credentials")).not.toBeInTheDocument();
    });
  });

  // ── Appearance section ─────────────────────────────────────────────────

  describe("Appearance section", () => {
    it("renders Appearance heading", async () => {
      await renderSettings();
      expect(screen.getByText("Appearance")).toBeInTheDocument();
    });

    it("shows dark and light theme buttons", async () => {
      await renderSettings();
      expect(screen.getByText("dark")).toBeInTheDocument();
      expect(screen.getByText("light")).toBeInTheDocument();
    });

    it("defaults to dark theme", async () => {
      await renderSettings();
      const darkBtn = screen.getByText("dark");
      expect(darkBtn.className).toContain("d4af37");
    });

    it("switches to light theme on click", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("light"));
      const lightBtn = screen.getByText("light");
      expect(lightBtn.className).toContain("d4af37");
    });
  });

  // ── Notifications section ──────────────────────────────────────────────

  describe("Notifications section", () => {
    it("renders Notifications heading", async () => {
      await renderSettings();
      expect(screen.getByText("Notifications")).toBeInTheDocument();
    });

    it("shows all 6 notification categories", async () => {
      await renderSettings();
      // Some labels (Social, Messages, etc.) appear in keyboard shortcuts too, so check toggles
      const switches = screen.getAllByRole("switch");
      expect(switches).toHaveLength(6);
      expect(screen.getByLabelText("Toggle Social notifications")).toBeInTheDocument();
      expect(screen.getByLabelText("Toggle Governance notifications")).toBeInTheDocument();
      expect(screen.getByLabelText("Toggle Payments notifications")).toBeInTheDocument();
      expect(screen.getByLabelText("Toggle Marketplace notifications")).toBeInTheDocument();
      expect(screen.getByLabelText("Toggle Messages notifications")).toBeInTheDocument();
      expect(screen.getByLabelText("Toggle System notifications")).toBeInTheDocument();
    });

    it("shows category descriptions", async () => {
      await renderSettings();
      expect(screen.getByText("Posts, follows, reactions, and mentions")).toBeInTheDocument();
      expect(screen.getByText("Connection status, updates, and errors")).toBeInTheDocument();
    });

    it("renders toggle switches for each category", async () => {
      await renderSettings();
      const switches = screen.getAllByRole("switch");
      expect(switches).toHaveLength(6);
    });

    it("all toggles default to enabled", async () => {
      await renderSettings();
      const switches = screen.getAllByRole("switch");
      for (const s of switches) {
        expect(s).toHaveAttribute("aria-checked", "true");
      }
    });

    it("toggles a category off on click", async () => {
      const user = userEvent.setup();
      await renderSettings();
      const socialToggle = screen.getByLabelText("Toggle Social notifications");
      expect(socialToggle).toHaveAttribute("aria-checked", "true");
      await user.click(socialToggle);
      expect(socialToggle).toHaveAttribute("aria-checked", "false");
    });

    it("shows Mute all button when all enabled", async () => {
      await renderSettings();
      expect(screen.getByText("Mute all")).toBeInTheDocument();
    });

    it("shows Unmute all after muting all", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("Mute all"));
      expect(screen.getByText("Unmute all")).toBeInTheDocument();
    });

    it("mute all disables all toggles", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("Mute all"));
      const switches = screen.getAllByRole("switch");
      for (const s of switches) {
        expect(s).toHaveAttribute("aria-checked", "false");
      }
    });
  });

  // ── Keyboard Shortcuts section ─────────────────────────────────────────

  describe("Keyboard Shortcuts section", () => {
    it("renders Keyboard Shortcuts heading", async () => {
      await renderSettings();
      expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
    });

    it("shows the ? hint text", async () => {
      await renderSettings();
      expect(screen.getByText(/anywhere/)).toBeInTheDocument();
    });

    it("shows General shortcuts section", async () => {
      await renderSettings();
      expect(screen.getByText("General")).toBeInTheDocument();
      expect(screen.getByText("Command palette")).toBeInTheDocument();
      expect(screen.getByText("Keyboard shortcuts")).toBeInTheDocument();
      expect(screen.getByText("Close modal / dismiss")).toBeInTheDocument();
    });

    it("shows Navigation shortcuts section", async () => {
      await renderSettings();
      expect(screen.getByText("Navigation")).toBeInTheDocument();
      expect(screen.getByText("Dashboard")).toBeInTheDocument();
      // "Social", "Messages", "Wallet", "Governance" also appear in notification categories
      // so use getAllByText and verify count >= 1
      expect(screen.getAllByText("Messages").length).toBeGreaterThanOrEqual(2); // notif + shortcut
      expect(screen.getAllByText("Wallet").length).toBeGreaterThanOrEqual(1);
    });

    it("shows Lists shortcuts section", async () => {
      await renderSettings();
      expect(screen.getByText("Lists")).toBeInTheDocument();
      expect(screen.getByText("Next item")).toBeInTheDocument();
      expect(screen.getByText("Previous item")).toBeInTheDocument();
      expect(screen.getByText("Activate selected item")).toBeInTheDocument();
    });

    it("renders kbd elements for shortcut keys", async () => {
      await renderSettings();
      const kbds = document.querySelectorAll("kbd");
      expect(kbds.length).toBeGreaterThan(10);
    });
  });

  // ── About section ──────────────────────────────────────────────────────

  describe("About section", () => {
    it("renders About heading", async () => {
      await renderSettings();
      expect(screen.getByText("About")).toBeInTheDocument();
    });

    it("shows Nous name and description", async () => {
      await renderSettings();
      expect(screen.getByText("Nous")).toBeInTheDocument();
      expect(screen.getByText("Decentralized social operating system")).toBeInTheDocument();
    });

    it("shows version badge", async () => {
      await renderSettings();
      expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    });

    it("shows tech stack details", async () => {
      await renderSettings();
      expect(screen.getByText("Runtime")).toBeInTheDocument();
      expect(screen.getByText("Frontend")).toBeInTheDocument();
      expect(screen.getByText("Crypto")).toBeInTheDocument();
      // "Network" appears both as tech stack row and as section heading
      expect(screen.getAllByText("Network").length).toBeGreaterThanOrEqual(2);
      expect(screen.getByText("Storage")).toBeInTheDocument();
    });

    it("shows runtime version from node when online", async () => {
      await renderSettings();
      expect(screen.getByText(/Rust · v0\.4\.0/)).toBeInTheDocument();
    });

    it("shows just Rust when offline", async () => {
      mockHealth.mockRejectedValue(new Error("offline"));
      await renderSettings();
      const runtimeValues = screen.getAllByText("Rust");
      expect(runtimeValues.length).toBeGreaterThan(0);
    });

    it("shows GitHub link", async () => {
      await renderSettings();
      const ghLink = screen.getByText("GitHub");
      expect(ghLink).toHaveAttribute("href", "https://github.com/teddytennant/nous");
      expect(ghLink).toHaveAttribute("target", "_blank");
    });

    it("shows Releases link", async () => {
      await renderSettings();
      const releasesLink = screen.getByText("Releases");
      expect(releasesLink).toHaveAttribute("href", "https://github.com/teddytennant/nous/releases");
    });

    it("shows Docs link", async () => {
      await renderSettings();
      const docsLink = screen.getByText("Docs");
      expect(docsLink).toHaveAttribute("href", "https://github.com/teddytennant/nous/blob/main/docs/ARCHITECTURE.md");
    });

    it("renders Restart Tour button", async () => {
      await renderSettings();
      expect(screen.getByText("Restart Tour")).toBeInTheDocument();
    });

    it("calls resetTour and shows toast on Restart Tour click", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("Restart Tour"));
      expect(mockResetTour).toHaveBeenCalled();
      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({ title: "Tour reset" }),
      );
    });
  });

  // ── Network section ────────────────────────────────────────────────────

  describe("Network section", () => {
    it("renders Network heading", async () => {
      await renderSettings();
      // "Network" appears as section heading and in tech stack details
      const networkTexts = screen.getAllByText("Network");
      expect(networkTexts.length).toBeGreaterThanOrEqual(2);
      // Check that one is a heading (h2)
      const headings = networkTexts.filter((el) => el.tagName === "H2");
      expect(headings).toHaveLength(1);
    });

    it("shows API Endpoint input with default value", async () => {
      await renderSettings();
      const input = screen.getByDisplayValue("http://localhost:8080/api/v1");
      expect(input).toBeInTheDocument();
    });

    it("shows node status when online", async () => {
      await renderSettings();
      expect(screen.getByText("healthy")).toBeInTheDocument();
    });

    it("shows node version when online", async () => {
      await renderSettings();
      expect(screen.getByText("0.4.0")).toBeInTheDocument();
    });

    it("shows node uptime when online", async () => {
      await renderSettings();
      expect(screen.getByText("360s")).toBeInTheDocument();
    });

    it("hides node info when offline", async () => {
      mockHealth.mockRejectedValue(new Error("offline"));
      await renderSettings();
      expect(screen.queryByText("healthy")).not.toBeInTheDocument();
    });
  });

  // ── Save & Clear actions ───────────────────────────────────────────────

  describe("Save and Clear actions", () => {
    it("renders Save Settings button", async () => {
      await renderSettings();
      expect(screen.getByText("Save Settings")).toBeInTheDocument();
    });

    it("renders Clear Local Data button", async () => {
      await renderSettings();
      expect(screen.getByText("Clear Local Data")).toBeInTheDocument();
    });

    it("saves all settings to localStorage on Save", async () => {
      const user = userEvent.setup();
      await renderSettings(MOCK_DID);

      // Change display name
      const nameInput = screen.getByPlaceholderText("Anonymous");
      await user.clear(nameInput);
      await user.type(nameInput, "Bob");

      await user.click(screen.getByText("Save Settings"));

      expect(localStorage.getItem("nous_did")).toBe(MOCK_DID);
      expect(localStorage.getItem("nous_display_name")).toBe("Bob");
      expect(localStorage.getItem("nous_theme")).toBe("dark");
      expect(localStorage.getItem("nous_api_url")).toBe("http://localhost:8080/api/v1");
    });

    it("shows success toast on save", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("Save Settings"));
      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({ title: "Settings saved", variant: "success" }),
      );
    });

    it("shows Saved text temporarily after saving", async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      await renderSettings();
      await user.click(screen.getByText("Save Settings"));
      expect(screen.getByText("Saved")).toBeInTheDocument();
      vi.useRealTimers();
    });

    it("clears all data on Clear Local Data", async () => {
      const user = userEvent.setup();
      localStorage.setItem("nous_did", MOCK_DID);
      localStorage.setItem("nous_display_name", "Alice");
      localStorage.setItem("nous_theme", "light");
      localStorage.setItem("nous_api_url", "http://custom:9999");
      localStorage.setItem("nous_notif_prefs", JSON.stringify({ social: false }));

      await renderSettings(MOCK_DID);
      await user.click(screen.getByText("Clear Local Data"));

      expect(localStorage.getItem("nous_did")).toBeNull();
      expect(localStorage.getItem("nous_display_name")).toBeNull();
      expect(localStorage.getItem("nous_theme")).toBeNull();
      expect(localStorage.getItem("nous_api_url")).toBeNull();
      expect(localStorage.getItem("nous_notif_prefs")).toBeNull();
    });

    it("shows toast on clear", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("Clear Local Data"));
      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({ title: "Data cleared" }),
      );
    });

    it("resets form fields after clear", async () => {
      const user = userEvent.setup();
      await renderSettings(MOCK_DID);
      await user.click(screen.getByText("Clear Local Data"));

      const didInput = screen.getByPlaceholderText("did:key:z...");
      expect(didInput).toHaveValue("");
      const nameInput = screen.getByPlaceholderText("Anonymous");
      expect(nameInput).toHaveValue("");
    });

    it("persists theme selection through save", async () => {
      const user = userEvent.setup();
      await renderSettings();
      await user.click(screen.getByText("light"));
      await user.click(screen.getByText("Save Settings"));
      expect(localStorage.getItem("nous_theme")).toBe("light");
    });

    it("persists notification preferences through save", async () => {
      const user = userEvent.setup();
      await renderSettings();
      const socialToggle = screen.getByLabelText("Toggle Social notifications");
      await user.click(socialToggle);
      await user.click(screen.getByText("Save Settings"));
      const prefs = JSON.parse(localStorage.getItem("nous_notif_prefs")!);
      expect(prefs.social).toBe(false);
    });
  });

  // ── API integration ────────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls node.health on mount", async () => {
      await renderSettings();
      expect(mockHealth).toHaveBeenCalled();
    });

    it("calls identity.get when DID is stored", async () => {
      await renderSettings(MOCK_DID);
      expect(mockIdentityGet).toHaveBeenCalledWith(MOCK_DID);
    });

    it("calls identity.listCredentials when DID is stored", async () => {
      await renderSettings(MOCK_DID);
      expect(mockListCredentials).toHaveBeenCalledWith(MOCK_DID);
    });

    it("calls identity.getReputation when DID is stored", async () => {
      await renderSettings(MOCK_DID);
      expect(mockGetReputation).toHaveBeenCalledWith(MOCK_DID);
    });

    it("handles identity.get failure gracefully", async () => {
      mockIdentityGet.mockRejectedValue(new Error("not found"));
      await renderSettings(MOCK_DID);
      // Should render without crashing, no verified text
      expect(screen.queryByText("Identity verified on node")).not.toBeInTheDocument();
    });

    it("handles reputation failure gracefully", async () => {
      mockGetReputation.mockRejectedValue(new Error("fail"));
      await renderSettings(MOCK_DID);
      expect(screen.queryByText("Reputation")).not.toBeInTheDocument();
    });

    it("handles credentials failure gracefully", async () => {
      mockListCredentials.mockRejectedValue(new Error("fail"));
      await renderSettings(MOCK_DID);
      expect(screen.queryByText("Credentials")).not.toBeInTheDocument();
    });
  });
});
