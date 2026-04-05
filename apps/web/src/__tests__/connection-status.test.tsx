import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, waitFor } from "@testing-library/react";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockHealth = vi.fn();

vi.mock("@/lib/api", () => ({
  node: {
    health: () => mockHealth(),
  },
}));

import { ConnectionProvider, useConnection } from "@/components/connection-status";

// ── Helper component to read context ──────────────────────────────────────

function StatusDisplay() {
  const { status, health, retry } = useConnection();
  return (
    <div>
      <span data-testid="status">{status}</span>
      <span data-testid="version">{health?.version ?? "none"}</span>
      <span data-testid="uptime">{health?.uptime_ms ?? "none"}</span>
      <button onClick={retry}>Retry</button>
    </div>
  );
}

function renderWithProvider() {
  return render(
    <ConnectionProvider>
      <StatusDisplay />
    </ConnectionProvider>,
  );
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("ConnectionProvider", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockHealth.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Initial state", () => {
    it("starts in connecting state", () => {
      mockHealth.mockImplementation(() => new Promise(() => {})); // never resolves
      renderWithProvider();
      expect(screen.getByTestId("status")).toHaveTextContent("connecting");
    });

    it("shows no health data initially", () => {
      mockHealth.mockImplementation(() => new Promise(() => {}));
      renderWithProvider();
      expect(screen.getByTestId("version")).toHaveTextContent("none");
      expect(screen.getByTestId("uptime")).toHaveTextContent("none");
    });
  });

  describe("Online state", () => {
    it("transitions to online when health check succeeds", async () => {
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.1.0",
        uptime_ms: 12345,
      });

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("online");
      });
    });

    it("provides health data when online", async () => {
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.2.0",
        uptime_ms: 99999,
      });

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("version")).toHaveTextContent("0.2.0");
        expect(screen.getByTestId("uptime")).toHaveTextContent("99999");
      });
    });
  });

  describe("Offline state", () => {
    it("transitions to offline when health check fails", async () => {
      mockHealth.mockRejectedValue(new Error("Network error"));

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("offline");
      });
    });

    it("clears health data when offline", async () => {
      mockHealth.mockRejectedValue(new Error("Network error"));

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("offline");
      });
      expect(screen.getByTestId("version")).toHaveTextContent("none");
      expect(screen.getByTestId("uptime")).toHaveTextContent("none");
    });
  });

  describe("Polling", () => {
    it("polls health every 15 seconds", async () => {
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.1.0",
        uptime_ms: 1000,
      });

      renderWithProvider();

      await waitFor(() => {
        expect(mockHealth).toHaveBeenCalledTimes(1);
      });

      // Advance past the 15s interval
      await act(async () => {
        vi.advanceTimersByTime(15000);
      });

      expect(mockHealth).toHaveBeenCalledTimes(2);
    });

    it("detects recovery from offline to online", async () => {
      // First call fails
      mockHealth.mockRejectedValueOnce(new Error("down"));

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("offline");
      });

      // Next poll succeeds
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.1.0",
        uptime_ms: 5000,
      });

      await act(async () => {
        vi.advanceTimersByTime(15000);
      });

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("online");
      });
    });
  });

  describe("Retry", () => {
    it("calls health check immediately on retry", async () => {
      mockHealth.mockRejectedValue(new Error("down"));

      renderWithProvider();

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("offline");
      });

      // Now make it succeed and retry
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.1.0",
        uptime_ms: 1000,
      });

      await act(async () => {
        screen.getByText("Retry").click();
      });

      await waitFor(() => {
        expect(screen.getByTestId("status")).toHaveTextContent("online");
      });
    });
  });

  describe("Cleanup", () => {
    it("stops polling on unmount", async () => {
      mockHealth.mockResolvedValue({
        status: "ok",
        version: "0.1.0",
        uptime_ms: 1000,
      });

      const { unmount } = renderWithProvider();

      await waitFor(() => {
        expect(mockHealth).toHaveBeenCalledTimes(1);
      });

      unmount();

      await act(async () => {
        vi.advanceTimersByTime(30000);
      });

      // Should not have been called again after unmount
      expect(mockHealth).toHaveBeenCalledTimes(1);
    });
  });
});

describe("useConnection (default context)", () => {
  it("provides default connecting status outside provider", () => {
    function Bare() {
      const { status, health } = useConnection();
      return (
        <div>
          <span data-testid="status">{status}</span>
          <span data-testid="health">{health ? "yes" : "no"}</span>
        </div>
      );
    }

    render(<Bare />);
    expect(screen.getByTestId("status")).toHaveTextContent("connecting");
    expect(screen.getByTestId("health")).toHaveTextContent("no");
  });
});
