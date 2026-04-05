import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

let mockStatus: "connecting" | "online" | "offline" = "offline";
const mockRetry = vi.fn();

vi.mock("@/components/connection-status", () => ({
  ConnectionProvider: ({ children }: { children: React.ReactNode }) => children,
  useConnection: () => ({ status: mockStatus, health: null, retry: mockRetry }),
}));

import { OfflineState, ConnectingState } from "@/components/offline-state";

// ── Tests ────────────────────────────────────────────────────────────────

describe("OfflineState", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockStatus = "offline";
    mockRetry.mockReset();
    // Set NEXT_PUBLIC_API_URL for consistent test output
    vi.stubEnv("NEXT_PUBLIC_API_URL", "http://localhost:8080/api/v1");
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllEnvs();
  });

  describe("Rendering", () => {
    it("renders the disconnected illustration", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      expect(svg).toBeInTheDocument();
    });

    it("renders illustration as aria-hidden", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      expect(svg).toHaveAttribute("aria-hidden", "true");
    });

    it("shows the title with Nous highlighted", () => {
      render(<OfflineState />);
      expect(screen.getByText("Unable to reach")).toBeInTheDocument();
      expect(screen.getByText("Nous")).toBeInTheDocument();
    });

    it("shows the description about API server", () => {
      render(<OfflineState />);
      expect(
        screen.getByText(/API server isn't responding/),
      ).toBeInTheDocument();
    });

    it("shows the API URL", () => {
      render(<OfflineState />);
      expect(
        screen.getByText("http://localhost:8080/api/v1"),
      ).toBeInTheDocument();
    });

    it("shows the Retry Now button", () => {
      render(<OfflineState />);
      expect(
        screen.getByRole("button", { name: /retry now/i }),
      ).toBeInTheDocument();
    });

    it("shows auto-retry countdown", () => {
      render(<OfflineState />);
      expect(screen.getByText(/Auto-retry in 15s/)).toBeInTheDocument();
    });

    it("shows the quick fix code hint", () => {
      render(<OfflineState />);
      expect(screen.getByText("Quick fix")).toBeInTheDocument();
      expect(screen.getByText("cargo run --bin nous-api")).toBeInTheDocument();
    });

    it("renders with offline-state-enter animation class", () => {
      const { container } = render(<OfflineState />);
      const wrapper = container.querySelector(".offline-state-enter");
      expect(wrapper).toBeInTheDocument();
    });

    it("has centered layout with min-height", () => {
      const { container } = render(<OfflineState />);
      const wrapper = container.firstElementChild;
      expect(wrapper?.className).toContain("flex");
      expect(wrapper?.className).toContain("flex-col");
      expect(wrapper?.className).toContain("items-center");
      expect(wrapper?.className).toContain("justify-center");
    });
  });

  describe("Disconnected illustration SVG", () => {
    it("renders central gold node", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      // Central node has cx=80 cy=80 with gold stroke
      const circles = svg?.querySelectorAll("circle");
      expect(circles?.length).toBeGreaterThanOrEqual(8); // center + 6 outer + center fill
    });

    it("renders dashed connection lines", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      const dashedLines = svg?.querySelectorAll('line[stroke-dasharray="3 5"]');
      expect(dashedLines?.length).toBe(6); // 6 broken connections
    });

    it("renders disconnect X mark in gold", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      const goldLines = svg?.querySelectorAll('line[stroke="#d4af37"]');
      expect(goldLines?.length).toBe(2); // two lines forming the X
    });

    it("renders broken orbit ring", () => {
      const { container } = render(<OfflineState />);
      const svg = container.querySelector("svg.offline-illustration");
      const orbit = svg?.querySelector('circle[r="56"]');
      expect(orbit).toBeInTheDocument();
      expect(orbit).toHaveAttribute("stroke-dasharray", "8 12");
    });
  });

  describe("Retry behavior", () => {
    it("calls retry on button click", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<OfflineState />);

      await user.click(screen.getByRole("button", { name: /retry now/i }));

      expect(mockRetry).toHaveBeenCalledOnce();
    });

    it("shows Reconnecting... text while retrying", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<OfflineState />);

      await user.click(screen.getByRole("button", { name: /retry now/i }));

      expect(screen.getByText("Reconnecting...")).toBeInTheDocument();
    });

    it("disables button while retrying", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<OfflineState />);

      await user.click(screen.getByRole("button", { name: /retry now/i }));

      const button = screen.getByRole("button");
      expect(button).toBeDisabled();
    });

    it("shows spinning RefreshCw icon while retrying", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const { container } = render(<OfflineState />);

      await user.click(screen.getByRole("button", { name: /retry now/i }));

      const icon = container.querySelector("svg.animate-spin");
      expect(icon).toBeInTheDocument();
    });

    it("resets retrying state after 1500ms", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<OfflineState />);

      await user.click(screen.getByRole("button", { name: /retry now/i }));
      expect(screen.getByText("Reconnecting...")).toBeInTheDocument();

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });

      expect(screen.getByText("Retry Now")).toBeInTheDocument();
      expect(screen.getByRole("button")).not.toBeDisabled();
    });
  });

  describe("Auto-retry countdown", () => {
    it("starts at 15 seconds", () => {
      render(<OfflineState />);
      expect(screen.getByText(/Auto-retry in 15s/)).toBeInTheDocument();
    });

    it("counts down every second", async () => {
      render(<OfflineState />);

      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      expect(screen.getByText(/Auto-retry in 14s/)).toBeInTheDocument();

      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      expect(screen.getByText(/Auto-retry in 13s/)).toBeInTheDocument();
    });

    it("resets to 15 when reaching 0", async () => {
      render(<OfflineState />);

      // Advance through full cycle (15 seconds)
      await act(async () => {
        vi.advanceTimersByTime(15000);
      });

      expect(screen.getByText(/Auto-retry in 15s/)).toBeInTheDocument();
    });

    it("shows pulsing status dot", () => {
      const { container } = render(<OfflineState />);
      const dot = container.querySelector(".offline-pulse");
      expect(dot).toBeInTheDocument();
    });

    it("resets countdown on manual retry", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      render(<OfflineState />);

      // Count down a bit
      await act(async () => {
        vi.advanceTimersByTime(5000);
      });
      expect(screen.getByText(/Auto-retry in 10s/)).toBeInTheDocument();

      // Manual retry resets after 1500ms
      await user.click(screen.getByRole("button", { name: /retry now/i }));

      await act(async () => {
        vi.advanceTimersByTime(1500);
      });

      expect(screen.getByText(/Auto-retry in 15s/)).toBeInTheDocument();
    });
  });
});

describe("ConnectingState", () => {
  it("renders connecting text", () => {
    render(<ConnectingState />);
    expect(screen.getByText("Connecting to Nous...")).toBeInTheDocument();
  });

  it("renders pulsing gold dot", () => {
    const { container } = render(<ConnectingState />);
    const dot = container.querySelector(".connecting-pulse");
    expect(dot).toBeInTheDocument();
  });

  it("has gold background on the pulse dot", () => {
    const { container } = render(<ConnectingState />);
    const dot = container.querySelector(".connecting-pulse");
    expect(dot?.className).toContain("bg-[#d4af37]");
  });

  it("renders with offline-state-enter animation class", () => {
    const { container } = render(<ConnectingState />);
    const wrapper = container.querySelector(".offline-state-enter");
    expect(wrapper).toBeInTheDocument();
  });

  it("has centered layout", () => {
    const { container } = render(<ConnectingState />);
    const wrapper = container.firstElementChild;
    expect(wrapper?.className).toContain("items-center");
    expect(wrapper?.className).toContain("justify-center");
  });

  it("renders the dot at 3x3 size", () => {
    const { container } = render(<ConnectingState />);
    const dot = container.querySelector(".connecting-pulse");
    expect(dot?.className).toContain("w-3");
    expect(dot?.className).toContain("h-3");
  });
});
