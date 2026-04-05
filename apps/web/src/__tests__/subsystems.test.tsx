import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockSubsystems = vi.fn();

vi.mock("@/lib/api", () => ({
  node: {
    subsystems: () => mockSubsystems(),
  },
}));

import { SubsystemsWidget, SubsystemsSkeleton } from "@/components/subsystems";

// ── Test data ────────────────────────────────────────────────────────────

const MOCK_SUBSYSTEMS = {
  overall: "healthy" as const,
  subsystems: [
    { name: "identity", status: "healthy" as const, active_count: 3, message: null },
    { name: "messaging", status: "healthy" as const, active_count: 12, message: null },
    { name: "governance", status: "healthy" as const, active_count: 2, message: null },
    { name: "payments", status: "degraded" as const, active_count: 0, message: "High latency" },
    { name: "social", status: "healthy" as const, active_count: 8, message: null },
    { name: "storage", status: "healthy" as const, active_count: 45, message: null },
    { name: "ai", status: "down" as const, active_count: 0, message: "No models loaded" },
    { name: "network", status: "healthy" as const, active_count: 6, message: null },
  ],
};

// ── Tests ────────────────────────────────────────────────────────────────

describe("SubsystemsSkeleton", () => {
  it("renders 8 skeleton rows", () => {
    const { container } = render(<SubsystemsSkeleton />);
    const rows = container.querySelectorAll(".animate-pulse");
    // Each row has 3 animated elements (dot + name + status)
    expect(rows.length).toBe(24); // 8 rows * 3
  });

  it("renders status dot placeholders", () => {
    const { container } = render(<SubsystemsSkeleton />);
    const dots = container.querySelectorAll(".rounded-full");
    expect(dots.length).toBe(8);
  });
});

describe("SubsystemsWidget", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockSubsystems.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Loading state", () => {
    it("shows skeleton while loading", () => {
      mockSubsystems.mockImplementation(() => new Promise(() => {}));
      const { container } = render(<SubsystemsWidget />);
      const skeletons = container.querySelectorAll(".animate-pulse");
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });

  describe("Healthy state", () => {
    it("shows 'All systems operational' when overall is healthy", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("All systems operational")).toBeInTheDocument();
      });
    });

    it("renders all subsystem names", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("identity")).toBeInTheDocument();
      });
      expect(screen.getByText("messaging")).toBeInTheDocument();
      expect(screen.getByText("governance")).toBeInTheDocument();
      expect(screen.getByText("payments")).toBeInTheDocument();
      expect(screen.getByText("social")).toBeInTheDocument();
      expect(screen.getByText("storage")).toBeInTheDocument();
      expect(screen.getByText("ai")).toBeInTheDocument();
      expect(screen.getByText("network")).toBeInTheDocument();
    });

    it("shows active count for subsystems with active_count > 0", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("12")).toBeInTheDocument(); // messaging
      });
      expect(screen.getByText("3")).toBeInTheDocument(); // identity
      expect(screen.getByText("45")).toBeInTheDocument(); // storage
    });

    it("shows status labels", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        const healthyLabels = screen.getAllByText("healthy");
        expect(healthyLabels.length).toBe(6);
      });
      expect(screen.getByText("degraded")).toBeInTheDocument();
      expect(screen.getByText("down")).toBeInTheDocument();
    });
  });

  describe("Degraded state", () => {
    it("shows 'Some systems degraded' when overall is degraded", async () => {
      mockSubsystems.mockResolvedValue({
        ...MOCK_SUBSYSTEMS,
        overall: "degraded",
      });
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("Some systems degraded")).toBeInTheDocument();
      });
    });
  });

  describe("Down state", () => {
    it("shows 'Systems down' when overall is down", async () => {
      mockSubsystems.mockResolvedValue({
        ...MOCK_SUBSYSTEMS,
        overall: "down",
      });
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("Systems down")).toBeInTheDocument();
      });
    });
  });

  describe("Messages", () => {
    it("shows subsystem message when present", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("High latency")).toBeInTheDocument();
      });
      expect(screen.getByText("No models loaded")).toBeInTheDocument();
    });
  });

  describe("Error state", () => {
    it("shows error message on fetch failure", async () => {
      mockSubsystems.mockRejectedValue(new Error("Network error"));
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("Unable to fetch subsystems")).toBeInTheDocument();
      });
    });
  });

  describe("Status dot colors", () => {
    it("uses gold dot for healthy subsystems", async () => {
      mockSubsystems.mockResolvedValue({
        overall: "healthy",
        subsystems: [
          { name: "identity", status: "healthy", active_count: 1, message: null },
        ],
      });
      const { container } = render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("identity")).toBeInTheDocument();
      });

      // Find the status dot (small rounded circle) in the subsystem row
      const row = screen.getByText("identity").closest("div")!.parentElement!;
      const dot = row.querySelector(".bg-\\[\\#d4af37\\]");
      expect(dot).toBeInTheDocument();
    });

    it("uses amber dot for degraded subsystems", async () => {
      mockSubsystems.mockResolvedValue({
        overall: "degraded",
        subsystems: [
          { name: "payments", status: "degraded", active_count: 0, message: null },
        ],
      });
      const { container } = render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("payments")).toBeInTheDocument();
      });

      const row = screen.getByText("payments").closest("div")!.parentElement!;
      const dot = row.querySelector(".bg-amber-500");
      expect(dot).toBeInTheDocument();
    });
  });

  describe("Polling", () => {
    it("refreshes data every 10 seconds", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(mockSubsystems).toHaveBeenCalledTimes(1);
      });

      await act(async () => {
        vi.advanceTimersByTime(10000);
      });

      expect(mockSubsystems).toHaveBeenCalledTimes(2);
    });

    it("stops polling on unmount", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      const { unmount } = render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(mockSubsystems).toHaveBeenCalledTimes(1);
      });

      unmount();

      await act(async () => {
        vi.advanceTimersByTime(20000);
      });

      expect(mockSubsystems).toHaveBeenCalledTimes(1);
    });
  });

  describe("Overall status indicator", () => {
    it("renders a gold status dot for overall healthy", async () => {
      mockSubsystems.mockResolvedValue(MOCK_SUBSYSTEMS);
      const { container } = render(<SubsystemsWidget />);

      await waitFor(() => {
        expect(screen.getByText("All systems operational")).toBeInTheDocument();
      });

      // The overall status dot is the first one in the header area
      const headerDot = container.querySelector(".w-2.h-2.rounded-full");
      expect(headerDot).toBeInTheDocument();
      expect(headerDot!.className).toContain("bg-[#d4af37]");
    });
  });
});
