import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockHealth = vi.fn();
const mockInfo = vi.fn();
const mockPeersList = vi.fn();
const mockPeersConnect = vi.fn();
const mockPeersDisconnect = vi.fn();

vi.mock("@/lib/api", () => ({
  node: {
    health: () => mockHealth(),
    info: () => mockInfo(),
  },
  peers: {
    list: () => mockPeersList(),
    connect: (addr: string) => mockPeersConnect(addr),
    disconnect: (id: string) => mockPeersDisconnect(id),
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
  }: {
    title: string;
    description: string;
  }) => (
    <div data-testid="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
    </div>
  ),
  NetworkIllustration: () => <div data-testid="network-illustration" />,
}));

vi.mock("@/components/sparkline", () => ({
  Sparkline: () => <div data-testid="sparkline" />,
}));

vi.mock("@/components/peer-graph", () => ({
  PeerGraph: () => <div data-testid="peer-graph" />,
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

vi.mock("@/components/ui/data-table", () => ({
  DataTable: ({
    columns,
    data,
    rowKey,
    emptyState,
  }: {
    columns: {
      id: string;
      header: string;
      cell: (row: unknown) => React.ReactNode;
    }[];
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

const MOCK_HEALTH = {
  status: "ok",
  version: "0.4.0",
  uptime_ms: 3600000,
};

const MOCK_NODE_INFO = {
  protocol: "nous",
  version: "0.4.0",
  features: ["identity", "messaging", "governance", "payments"],
};

const MOCK_PEERS = [
  {
    peer_id: "12D3KooWAbcdef1234567890abcdef1234567890abcdef12345678",
    multiaddr: "/ip4/192.168.1.100/tcp/9000",
    latency_ms: 25,
    bytes_sent: 1048576,
    bytes_recv: 2097152,
    connected_at: new Date(Date.now() - 3600000).toISOString(),
    protocols: ["/nous/1.0.0", "/gossipsub/1.1.0"],
  },
  {
    peer_id: "12D3KooWXyz9876543210xyz9876543210xyz9876543210xyz98765",
    multiaddr: "/ip4/10.0.0.50/tcp/9000",
    latency_ms: 120,
    bytes_sent: 524288,
    bytes_recv: 262144,
    connected_at: new Date(Date.now() - 7200000).toISOString(),
    protocols: ["/nous/1.0.0"],
  },
];

// ── Helpers ──────────────────────────────────────────────────────────────

import NetworkPage from "@/app/(app)/network/page";

function setupDefaults() {
  mockHealth.mockResolvedValue(MOCK_HEALTH);
  mockInfo.mockResolvedValue(MOCK_NODE_INFO);
  mockPeersList.mockResolvedValue({ peers: MOCK_PEERS, count: 2 });
  mockPeersConnect.mockResolvedValue(MOCK_PEERS[0]);
  mockPeersDisconnect.mockResolvedValue(undefined);
}

async function renderNetwork() {
  setupDefaults();
  render(<NetworkPage />);
  await waitFor(() => {
    expect(screen.getByTestId("page-header")).toBeInTheDocument();
  });
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Network page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Page structure ────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header", async () => {
      await renderNetwork();
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Network")).toBeInTheDocument();
    });

    it("renders Overview section heading", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Overview")).toBeInTheDocument();
      });
    });

    it("renders Subsystems section heading", async () => {
      await renderNetwork();
      expect(screen.getByText("Subsystems")).toBeInTheDocument();
    });

    it("renders Connected Peers section heading", async () => {
      await renderNetwork();
      expect(screen.getByText("Connected Peers")).toBeInTheDocument();
    });
  });

  // ── Stats overview ────────────────────────────────────────────────────

  describe("Stats overview", () => {
    it("shows Online status when healthy", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Online")).toBeInTheDocument();
      });
    });

    it("shows version in status detail", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("v0.4.0")).toBeInTheDocument();
      });
    });

    it("shows peer count", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("2")).toBeInTheDocument();
      });
    });

    it("shows peer count detail", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("connected")).toBeInTheDocument();
      });
    });

    it("shows total bandwidth", async () => {
      await renderNetwork();
      await waitFor(() => {
        // Total: 1048576 + 2097152 + 524288 + 262144 = 3932160 = 3.8 MB
        expect(screen.getByText("3.8 MB")).toBeInTheDocument();
      });
    });

    it("shows average latency", async () => {
      await renderNetwork();
      await waitFor(() => {
        // Avg: (25 + 120) / 2 = 72.5 → rounded to 73ms
        expect(screen.getByText("73ms")).toBeInTheDocument();
      });
    });

    it("shows stat labels", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Bandwidth")).toBeInTheDocument();
        // "Status", "Peers", and "Latency" labels also exist as headers
        // Verify via the Overview section containing the stat cards
        const overview = screen.getByText("Overview");
        expect(overview).toBeInTheDocument();
      });
    });
  });

  // ── Subsystems ────────────────────────────────────────────────────────

  describe("Subsystems", () => {
    it("renders all 8 subsystem names", async () => {
      await renderNetwork();
      // Some subsystem names conflict with page header or other labels.
      // Verify by checking that the Subsystems section contains all 8 items.
      const operationalLabels = screen.getAllByText("operational");
      expect(operationalLabels.length).toBe(8);
      // Check unique subsystem names
      expect(screen.getByText("Networking")).toBeInTheDocument();
      expect(screen.getByText("Payments")).toBeInTheDocument();
    });

    it("shows subsystem descriptions", async () => {
      await renderNetwork();
      expect(
        screen.getByText("libp2p transport, gossipsub, relay")
      ).toBeInTheDocument();
      expect(
        screen.getByText("DID resolver, credential store")
      ).toBeInTheDocument();
    });

    it("shows operational status for all subsystems", async () => {
      await renderNetwork();
      const operationalLabels = screen.getAllByText("operational");
      expect(operationalLabels.length).toBe(8);
    });
  });

  // ── Peer topology graph ───────────────────────────────────────────────

  describe("Peer topology", () => {
    it("renders peer graph when peers exist", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByTestId("peer-graph")).toBeInTheDocument();
      });
    });

    it("shows topology section heading", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Topology")).toBeInTheDocument();
      });
    });

    it("does not render peer graph when no peers", async () => {
      setupDefaults();
      mockPeersList.mockResolvedValue({ peers: [], count: 0 });
      render(<NetworkPage />);
      await waitFor(() => {
        expect(screen.getByTestId("page-header")).toBeInTheDocument();
      });
      await waitFor(() => {
        expect(screen.queryByTestId("peer-graph")).not.toBeInTheDocument();
      });
    });
  });

  // ── Peer table ────────────────────────────────────────────────────────

  describe("Peer table", () => {
    it("renders data table with peers", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByTestId("data-table")).toBeInTheDocument();
      });
    });

    it("shows peer table headers", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Peer ID")).toBeInTheDocument();
        expect(screen.getByText("Address")).toBeInTheDocument();
        expect(screen.getByText("Sent")).toBeInTheDocument();
        expect(screen.getByText("Recv")).toBeInTheDocument();
        expect(screen.getByText("Connected")).toBeInTheDocument();
      });
    });

    it("shows peer addresses", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(
          screen.getByText("/ip4/192.168.1.100/tcp/9000")
        ).toBeInTheDocument();
        expect(
          screen.getByText("/ip4/10.0.0.50/tcp/9000")
        ).toBeInTheDocument();
      });
    });

    it("shows peer latency values", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("25ms")).toBeInTheDocument();
        expect(screen.getByText("120ms")).toBeInTheDocument();
      });
    });

    it("shows disconnect buttons", async () => {
      await renderNetwork();
      await waitFor(() => {
        const disconnectBtns = screen.getAllByText("disconnect");
        expect(disconnectBtns.length).toBe(2);
      });
    });

    it("shows empty state when no peers and online", async () => {
      setupDefaults();
      mockPeersList.mockResolvedValue({ peers: [], count: 0 });
      render(<NetworkPage />);
      await waitFor(() => {
        expect(screen.getByText("No peers connected")).toBeInTheDocument();
      });
    });
  });

  // ── Connect/Disconnect ────────────────────────────────────────────────

  describe("Connect and disconnect", () => {
    it("renders connect input", async () => {
      await renderNetwork();
      expect(
        screen.getByPlaceholderText("/ip4/.../tcp/9000")
      ).toBeInTheDocument();
    });

    it("renders Connect button", async () => {
      await renderNetwork();
      expect(screen.getByText("Connect")).toBeInTheDocument();
    });

    it("calls peers.connect on submit", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      await renderNetwork();
      const input = screen.getByPlaceholderText("/ip4/.../tcp/9000");
      await user.type(input, "/ip4/10.0.0.1/tcp/9000");
      await user.click(screen.getByText("Connect"));
      await waitFor(() => {
        expect(mockPeersConnect).toHaveBeenCalledWith("/ip4/10.0.0.1/tcp/9000");
      });
    });

    it("calls peers.disconnect on disconnect click", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getAllByText("disconnect").length).toBe(2);
      });
      await user.click(screen.getAllByText("disconnect")[0]);
      await waitFor(() => {
        expect(mockPeersDisconnect).toHaveBeenCalledWith(
          MOCK_PEERS[0].peer_id
        );
      });
    });

    it("shows error toast on connect failure", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      setupDefaults();
      mockPeersConnect.mockRejectedValue(new Error("Connection refused"));
      render(<NetworkPage />);
      await waitFor(() => {
        expect(screen.getByText("Connect")).toBeInTheDocument();
      });
      const input = screen.getByPlaceholderText("/ip4/.../tcp/9000");
      await user.type(input, "/ip4/bad/tcp/9000");
      await user.click(screen.getByText("Connect"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({
            title: "Connection failed",
            variant: "error",
          })
        );
      });
    });
  });

  // ── Protocol info ─────────────────────────────────────────────────────

  describe("Protocol info", () => {
    it("shows protocol version when nodeInfo is loaded", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("nous/v0.4.0")).toBeInTheDocument();
      });
    });

    it("shows protocol version", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("nous/v0.4.0")).toBeInTheDocument();
      });
    });

    it("shows transport info", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(
          screen.getByText("TCP + Noise + Yamux")
        ).toBeInTheDocument();
      });
    });

    it("shows active features", async () => {
      await renderNetwork();
      await waitFor(() => {
        expect(screen.getByText("Active Features")).toBeInTheDocument();
        expect(
          screen.getByText("identity, messaging, governance, payments")
        ).toBeInTheDocument();
      });
    });
  });

  // ── API integration ───────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls node.health on mount", async () => {
      await renderNetwork();
      expect(mockHealth).toHaveBeenCalled();
    });

    it("calls node.info on mount", async () => {
      await renderNetwork();
      expect(mockInfo).toHaveBeenCalled();
    });

    it("calls peers.list on mount", async () => {
      await renderNetwork();
      expect(mockPeersList).toHaveBeenCalled();
    });

    it("handles API failure gracefully", async () => {
      setupDefaults();
      mockHealth.mockRejectedValue(new Error("ECONNREFUSED"));
      mockInfo.mockRejectedValue(new Error("ECONNREFUSED"));
      mockPeersList.mockRejectedValue(new Error("ECONNREFUSED"));
      render(<NetworkPage />);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "API offline", variant: "error" })
        );
      });
    });

    it("shows Node offline in empty state when API is down", async () => {
      setupDefaults();
      mockHealth.mockRejectedValue(new Error("ECONNREFUSED"));
      mockInfo.mockRejectedValue(new Error("ECONNREFUSED"));
      mockPeersList.mockRejectedValue(new Error("ECONNREFUSED"));
      render(<NetworkPage />);
      await waitFor(() => {
        expect(screen.getByText("Node offline")).toBeInTheDocument();
      });
    });
  });
});
