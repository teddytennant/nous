import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockListDaos = vi.fn();
const mockCreateDao = vi.fn();
const mockGetDao = vi.fn();
const mockListProposals = vi.fn();
const mockCreateProposal = vi.fn();
const mockGetTally = vi.fn();
const mockVote = vi.fn();
const mockListDelegations = vi.fn();
const mockDelegate = vi.fn();
const mockRevokeDelegation = vi.fn();
const mockGetPower = vi.fn();
const mockIdentityCreate = vi.fn();

vi.mock("@/lib/api", () => ({
  governance: {
    listDaos: () => mockListDaos(),
    createDao: (did: string, name: string, desc: string) => mockCreateDao(did, name, desc),
    getDao: (id: string) => mockGetDao(id),
    listProposals: (daoId?: string) => mockListProposals(daoId),
    createProposal: (daoId: string, data: unknown) => mockCreateProposal(daoId, data),
    getTally: (id: string) => mockGetTally(id),
    vote: (id: string, data: unknown) => mockVote(id, data),
  },
  delegation: {
    listDelegations: (did: string) => mockListDelegations(did),
    delegate: (data: unknown) => mockDelegate(data),
    revoke: (id: string, did: string) => mockRevokeDelegation(id, did),
    getPower: (daoId: string) => mockGetPower(daoId),
  },
  identity: {
    create: (name?: string) => mockIdentityCreate(name),
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
  EmptyState: ({ title, description, action }: { title: string; description: string; action?: React.ReactNode }) => (
    <div data-testid="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  ),
  GovernanceIllustration: () => <div data-testid="governance-illustration" />,
  DelegationIllustration: () => <div data-testid="delegation-illustration" />,
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
  Avatar: ({ did }: { did: string }) => <div data-testid="avatar" data-did={did} />,
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/governance-analytics", () => ({
  GovernanceAnalytics: () => <div data-testid="governance-analytics">Analytics</div>,
}));

vi.mock("@/components/proposal-detail-sheet", () => ({
  ProposalDetailSheet: () => null,
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

import GovernancePage from "@/app/(app)/governance/page";

// ── Fixtures ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc";

const MOCK_DAOS = [
  {
    id: "dao-1",
    name: "Nous Core",
    description: "The core governance DAO",
    founder_did: MOCK_DID,
    member_count: 42,
    created_at: "2026-01-01T00:00:00Z",
  },
  {
    id: "dao-2",
    name: "Community Fund",
    description: "Treasury DAO for community projects",
    founder_did: "did:key:z6Mk999",
    member_count: 100,
    created_at: "2026-02-01T00:00:00Z",
  },
];

const MOCK_PROPOSALS = [
  {
    id: "prop-1",
    dao_id: "dao-1",
    title: "Increase member cap",
    description: "Raise the member limit from 100 to 500",
    proposer_did: MOCK_DID,
    status: "active",
    created_at: "2026-03-01T00:00:00Z",
    voting_starts: "2026-03-01T00:00:00Z",
    voting_ends: "2026-04-01T00:00:00Z",
    quorum: 10,
  },
  {
    id: "prop-2",
    dao_id: "dao-1",
    title: "Treasury allocation Q2",
    description: "Allocate 10K NOUS for Q2 grants",
    proposer_did: "did:key:z6Mk999",
    status: "passed",
    created_at: "2026-02-01T00:00:00Z",
    voting_starts: "2026-02-01T00:00:00Z",
    voting_ends: "2026-03-01T00:00:00Z",
    quorum: 10,
  },
];

const MOCK_TALLY = {
  proposal_id: "prop-1",
  votes_for: 25,
  votes_against: 3,
  votes_abstain: 2,
  total_voters: 30,
  passed: false,
};

// ── Helpers ──────────────────────────────────────────────────────────────

function setupMocks() {
  mockListDaos.mockResolvedValue({ daos: MOCK_DAOS, count: 2 });
  mockListProposals.mockResolvedValue({ proposals: MOCK_PROPOSALS, count: 2 });
  mockGetTally.mockResolvedValue(MOCK_TALLY);
  mockCreateDao.mockResolvedValue({ id: "dao-new", name: "New DAO" });
  mockCreateProposal.mockResolvedValue({ id: "prop-new", title: "New Proposal" });
  mockVote.mockResolvedValue({ success: true, message: "Vote recorded" });
  mockGetDao.mockResolvedValue({ ...MOCK_DAOS[0], members: [], default_quorum: 10, default_threshold: 50 });
  mockListDelegations.mockResolvedValue({ delegations: [], count: 0 });
  mockGetPower.mockResolvedValue([]);
  mockIdentityCreate.mockResolvedValue({ did: MOCK_DID, display_name: "Nous User", signing_key_type: "Ed25519", exchange_key_type: "X25519" });
}

async function renderGovernance(did?: string) {
  if (did) localStorage.setItem("nous_did", did);
  render(<GovernancePage />);
  // Wait for loading to finish
  await screen.findByText("Governance");
}

// ── Tests ───────────────────────────────────────────────────────────────��

describe("Governance page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setupMocks();
  });

  // ── Page structure ────────────────────────────��────────────────────────

  describe("Page structure", () => {
    it("renders page header", async () => {
      await renderGovernance(MOCK_DID);
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Governance")).toBeInTheDocument();
    });

    it("renders four tab buttons", async () => {
      await renderGovernance(MOCK_DID);
      expect(screen.getByText("analytics")).toBeInTheDocument();
      expect(screen.getByText("proposals")).toBeInTheDocument();
      expect(screen.getByText("daos")).toBeInTheDocument();
      expect(screen.getByText("delegation")).toBeInTheDocument();
    });

    it("defaults to analytics tab", async () => {
      await renderGovernance(MOCK_DID);
      const analyticsBtn = screen.getByText("analytics");
      expect(analyticsBtn.className).toContain("d4af37");
    });

    it("renders GovernanceAnalytics on analytics tab", async () => {
      await renderGovernance(MOCK_DID);
      expect(screen.getByTestId("governance-analytics")).toBeInTheDocument();
    });
  });

  // ── Proposals tab ─────────────────────────────���────────────────────────

  describe("Proposals tab", () => {
    it("switches to proposals tab", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("Increase member cap")).toBeInTheDocument();
    });

    it("shows proposal titles", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("Increase member cap")).toBeInTheDocument();
      expect(screen.getByText("Treasury allocation Q2")).toBeInTheDocument();
    });

    it("shows proposal status", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("active")).toBeInTheDocument();
      expect(screen.getByText("passed")).toBeInTheDocument();
    });

    it("shows status filter buttons", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("All")).toBeInTheDocument();
      expect(screen.getByText("Active")).toBeInTheDocument();
      expect(screen.getByText("Passed")).toBeInTheDocument();
      expect(screen.getByText("Rejected")).toBeInTheDocument();
    });

    it("filters proposals by Active status", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      await user.click(screen.getByText("Active"));
      expect(screen.getByText("Increase member cap")).toBeInTheDocument();
      expect(screen.queryByText("Treasury allocation Q2")).not.toBeInTheDocument();
    });

    it("shows New Proposal button", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("New Proposal")).toBeInTheDocument();
    });

    it("shows empty state when no proposals", async () => {
      mockListProposals.mockResolvedValue({ proposals: [], count: 0 });
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      expect(screen.getByText("No proposals yet")).toBeInTheDocument();
    });

    it("shows tally vote counts in proposals", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      // Wait for tallies to load async
      await waitFor(() => {
        expect(screen.getAllByText(/25 for/).length).toBeGreaterThanOrEqual(1);
      });
      expect(screen.getAllByText(/3 against/).length).toBeGreaterThanOrEqual(1);
    });

    it("shows voter count in proposals", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      await waitFor(() => {
        expect(screen.getAllByText(/30 voters/).length).toBeGreaterThanOrEqual(1);
      });
    });
  });

  // ── DAOs tab ───────────────────────────────────────────────────────────

  describe("DAOs tab", () => {
    it("switches to daos tab", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText("Nous Core")).toBeInTheDocument();
    });

    it("shows DAO list", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText("Nous Core")).toBeInTheDocument();
      expect(screen.getByText("Community Fund")).toBeInTheDocument();
    });

    it("shows DAO descriptions", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText("The core governance DAO")).toBeInTheDocument();
    });

    it("shows member counts", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText(/42 member/)).toBeInTheDocument();
      expect(screen.getByText(/100 member/)).toBeInTheDocument();
    });

    it("shows Create DAO button", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText("Create DAO")).toBeInTheDocument();
    });

    it("opens DAO creation dialog", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      await user.click(screen.getByText("Create DAO"));
      expect(screen.getByTestId("dialog")).toBeInTheDocument();
    });

    it("shows empty state when no DAOs", async () => {
      mockListDaos.mockResolvedValue({ daos: [], count: 0 });
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("daos"));
      expect(screen.getByText("No DAOs yet")).toBeInTheDocument();
    });
  });

  // ── Delegation tab ─────────────────────────────────────────────────────

  describe("Delegation tab", () => {
    it("switches to delegation tab", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("delegation"));
      // Should show delegation content
      expect(screen.getByTestId("empty-state")).toBeInTheDocument();
    });

    it("shows empty delegation state when no delegations", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("delegation"));
      expect(screen.getByText("No active delegations")).toBeInTheDocument();
    });
  });

  // ── Create proposal dialog ─────────────────────────────────────────────

  describe("Create proposal dialog", () => {
    it("opens proposal dialog from proposals tab", async () => {
      const user = userEvent.setup();
      await renderGovernance(MOCK_DID);
      await user.click(screen.getByText("proposals"));
      await user.click(screen.getByText("New Proposal"));
      expect(screen.getByTestId("dialog")).toBeInTheDocument();
    });
  });

  // ── API integration ────────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls listDaos on mount", async () => {
      await renderGovernance(MOCK_DID);
      expect(mockListDaos).toHaveBeenCalled();
    });

    it("calls listProposals on mount", async () => {
      await renderGovernance(MOCK_DID);
      expect(mockListProposals).toHaveBeenCalled();
    });

    it("calls getTally for each proposal", async () => {
      await renderGovernance(MOCK_DID);
      await waitFor(() => {
        expect(mockGetTally).toHaveBeenCalledWith("prop-1");
        expect(mockGetTally).toHaveBeenCalledWith("prop-2");
      });
    });

    it("handles proposals API failure with toast", async () => {
      mockListProposals.mockRejectedValue(new Error("Server error"));
      await renderGovernance(MOCK_DID);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Failed to load proposals", variant: "error" }),
        );
      });
    });

    it("handles DAOs API failure gracefully", async () => {
      mockListDaos.mockRejectedValue(new Error("fail"));
      await renderGovernance(MOCK_DID);
      // Should still render without crashing
      expect(screen.getByTestId("page-header")).toBeInTheDocument();
    });
  });
});
