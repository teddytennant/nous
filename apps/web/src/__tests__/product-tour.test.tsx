import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ProductTour, markTourCompleted, resetTour } from "@/components/product-tour";

// ── Mocks ────────────────────────────────────────────────────────────────

// Mock getBoundingClientRect for target elements
function mockTargetElement(selector: string, rect: DOMRect) {
  const el = document.createElement("div");
  el.setAttribute("data-tour", selector.replace("[data-tour='", "").replace("']", ""));
  el.getBoundingClientRect = () => rect;
  document.body.appendChild(el);
  return el;
}

const defaultRect = new DOMRect(100, 100, 200, 40);

function setupTourTargets() {
  const targets = [
    "sidebar",
    "search",
    "notifications",
    "stats",
    "shortcuts",
    "user",
  ];
  const elements: HTMLElement[] = [];
  for (const t of targets) {
    elements.push(
      mockTargetElement(`[data-tour='${t}']`, defaultRect),
    );
  }
  return elements;
}

// ── Helpers ──────────────────────────────────────────────────────────────

function renderTour() {
  return render(<ProductTour />);
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("ProductTour", () => {
  let tourTargets: HTMLElement[] = [];

  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    localStorage.clear();
    tourTargets = setupTourTargets();
  });

  afterEach(() => {
    vi.useRealTimers();
    // Clean up target elements
    for (const el of tourTargets) {
      el.remove();
    }
    tourTargets = [];
  });

  describe("Auto-start behavior", () => {
    it("starts automatically when tour is not completed", async () => {
      renderTour();

      // Tour starts after 800ms delay
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
    });

    it("does not start when tour is already completed", async () => {
      markTourCompleted();
      renderTour();

      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      expect(screen.queryByText("Navigate Your Node")).not.toBeInTheDocument();
    });

    it("renders nothing before the 800ms delay", () => {
      renderTour();
      expect(screen.queryByText("Navigate Your Node")).not.toBeInTheDocument();
    });
  });

  describe("Step content", () => {
    async function startTour() {
      const result = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
      return result;
    }

    it("shows first step title and description", async () => {
      await startTour();

      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
      expect(
        screen.getByText(/sidebar groups 11 subsystems/),
      ).toBeInTheDocument();
    });

    it("shows step counter as 1/6", async () => {
      await startTour();

      expect(screen.getByText("1/6")).toBeInTheDocument();
    });

    it("shows 6 step indicator dots", async () => {
      const { container } = await startTour();

      // 6 step dots + the counter text
      const dots = container.querySelectorAll("[class*='rounded-full']");
      // Filter to only the step indicator dots (h-1 class)
      const stepDots = Array.from(dots).filter(
        (d) => d.className.includes("h-1"),
      );
      expect(stepDots).toHaveLength(6);
    });

    it("highlights the active step dot in gold", async () => {
      const { container } = await startTour();

      const stepDots = Array.from(
        container.querySelectorAll("[class*='h-1'][class*='rounded-full']"),
      );
      const activeDot = stepDots.find(
        (d) => d.className.includes("w-4") && d.className.includes("bg-[#d4af37]"),
      );
      expect(activeDot).toBeTruthy();
    });

    it("does not show Back button on first step", async () => {
      await startTour();

      expect(screen.queryByText("Back")).not.toBeInTheDocument();
    });

    it("shows Next button on first step", async () => {
      await startTour();

      expect(screen.getByText("Next")).toBeInTheDocument();
    });

    it("shows Close tour button with aria-label", async () => {
      await startTour();

      expect(
        screen.getByRole("button", { name: "Close tour" }),
      ).toBeInTheDocument();
    });
  });

  describe("Navigation — Next and Back", () => {
    async function startTour() {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
      return user;
    }

    it("advances to second step on Next click", async () => {
      const user = await startTour();

      await user.click(screen.getByText("Next"));

      expect(screen.getByText("Command Palette")).toBeInTheDocument();
      expect(screen.getByText("2/6")).toBeInTheDocument();
    });

    it("shows Back button on second step", async () => {
      const user = await startTour();

      await user.click(screen.getByText("Next"));

      expect(screen.getByText("Back")).toBeInTheDocument();
    });

    it("goes back to first step on Back click", async () => {
      const user = await startTour();

      await user.click(screen.getByText("Next"));
      expect(screen.getByText("Command Palette")).toBeInTheDocument();

      await user.click(screen.getByText("Back"));
      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
      expect(screen.getByText("1/6")).toBeInTheDocument();
    });

    it("navigates through all 6 steps", async () => {
      const user = await startTour();

      const expectedTitles = [
        "Navigate Your Node",
        "Command Palette",
        "Notifications",
        "System at a Glance",
        "Quick Actions & Shortcuts",
        "Your Identity",
      ];

      for (let i = 0; i < expectedTitles.length; i++) {
        expect(screen.getByText(expectedTitles[i])).toBeInTheDocument();
        expect(screen.getByText(`${i + 1}/6`)).toBeInTheDocument();

        if (i < expectedTitles.length - 1) {
          await user.click(screen.getByText("Next"));
        }
      }
    });

    it("shows Done button on last step instead of Next", async () => {
      const user = await startTour();

      // Navigate to last step
      for (let i = 0; i < 5; i++) {
        await user.click(screen.getByText("Next"));
      }

      expect(screen.getByText("Your Identity")).toBeInTheDocument();
      expect(screen.queryByText("Next")).not.toBeInTheDocument();
      expect(screen.getByText("Done")).toBeInTheDocument();
    });
  });

  describe("Keyboard navigation", () => {
    async function startTour() {
      renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
    }

    it("advances on ArrowRight", async () => {
      await startTour();

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
        );
      });

      expect(screen.getByText("Command Palette")).toBeInTheDocument();
    });

    it("advances on Enter", async () => {
      await startTour();

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
        );
      });

      expect(screen.getByText("Command Palette")).toBeInTheDocument();
    });

    it("goes back on ArrowLeft", async () => {
      await startTour();

      // Go forward first
      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true }),
        );
      });
      expect(screen.getByText("Command Palette")).toBeInTheDocument();

      // Go back
      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }),
        );
      });

      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
    });

    it("ArrowLeft does nothing on first step", async () => {
      await startTour();

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true }),
        );
      });

      // Still on first step
      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
      expect(screen.getByText("1/6")).toBeInTheDocument();
    });
  });

  describe("Dismissal", () => {
    async function startTour() {
      renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
    }

    it("dismisses on Escape key", async () => {
      await startTour();
      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
        );
      });

      // Exit animation takes 200ms
      await act(async () => {
        vi.advanceTimersByTime(200);
      });

      expect(screen.queryByText("Navigate Your Node")).not.toBeInTheDocument();
    });

    it("marks tour as completed on dismiss", async () => {
      await startTour();

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
        );
      });

      await act(async () => {
        vi.advanceTimersByTime(200);
      });

      expect(localStorage.getItem("nous_tour_completed")).toBe("true");
    });

    it("dismisses on Close button click", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      await startTour();

      await user.click(screen.getByRole("button", { name: "Close tour" }));

      await act(async () => {
        vi.advanceTimersByTime(200);
      });

      expect(screen.queryByText("Navigate Your Node")).not.toBeInTheDocument();
      expect(localStorage.getItem("nous_tour_completed")).toBe("true");
    });

    it("dismisses on backdrop click", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const { container } = renderTour();

      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      // The backdrop is the fixed inset-0 div (click blocker)
      const backdrop = container.querySelector(".fixed.inset-0:not([class*='z-'])");
      if (backdrop) {
        await user.click(backdrop);
      }

      await act(async () => {
        vi.advanceTimersByTime(200);
      });

      expect(localStorage.getItem("nous_tour_completed")).toBe("true");
    });

    it("completes tour on Done click at last step", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderTour();

      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      // Navigate to last step
      for (let i = 0; i < 5; i++) {
        await user.click(screen.getByText("Next"));
      }

      await user.click(screen.getByText("Done"));

      await act(async () => {
        vi.advanceTimersByTime(200);
      });

      expect(localStorage.getItem("nous_tour_completed")).toBe("true");
      expect(screen.queryByText("Your Identity")).not.toBeInTheDocument();
    });
  });

  describe("Spotlight and overlay", () => {
    it("renders an SVG overlay with spotlight mask", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const svg = container.querySelector("svg.absolute");
      expect(svg).toBeInTheDocument();

      const mask = container.querySelector("#tour-spotlight-mask");
      expect(mask).toBeInTheDocument();
    });

    it("renders spotlight glow around target", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const glow = container.querySelector(".tour-spotlight-glow");
      expect(glow).toBeInTheDocument();
    });

    it("applies tour-enter class on mount", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const overlay = container.querySelector(".tour-enter");
      expect(overlay).toBeInTheDocument();
    });

    it("applies tour-exit class on dismiss", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      act(() => {
        window.dispatchEvent(
          new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
        );
      });

      // Before the 200ms timeout, it should have the exit class
      const exitOverlay = container.querySelector(".tour-exit");
      expect(exitOverlay).toBeInTheDocument();
    });
  });

  describe("Popover positioning", () => {
    it("renders popover with fixed positioning", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const popover = container.querySelector(".tour-popover-enter");
      expect(popover).toBeInTheDocument();
    });

    it("popover has correct width", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const popover = container.querySelector(".tour-popover-enter") as HTMLElement;
      expect(popover).toBeInTheDocument();
      expect(popover.style.width).toBe("320px");
    });
  });

  describe("Step descriptions", () => {
    async function goToStep(n: number) {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
      for (let i = 0; i < n; i++) {
        await user.click(screen.getByText("Next"));
      }
      return user;
    }

    it("step 1: Navigate Your Node — sidebar description", async () => {
      await goToStep(0);
      expect(screen.getByText(/sidebar groups 11 subsystems/)).toBeInTheDocument();
    });

    it("step 2: Command Palette — keyboard shortcut description", async () => {
      await goToStep(1);
      expect(screen.getByText("Command Palette")).toBeInTheDocument();
      expect(screen.getByText(/Ctrl\+K/)).toBeInTheDocument();
    });

    it("step 3: Notifications — cross-subsystem activity", async () => {
      await goToStep(2);
      expect(screen.getByText("Notifications")).toBeInTheDocument();
      expect(screen.getByText(/governance votes, new messages/)).toBeInTheDocument();
    });

    it("step 4: System at a Glance — dashboard stats", async () => {
      await goToStep(3);
      expect(screen.getByText("System at a Glance")).toBeInTheDocument();
      expect(screen.getByText(/node health, uptime/)).toBeInTheDocument();
    });

    it("step 5: Quick Actions & Shortcuts — keyboard hint", async () => {
      await goToStep(4);
      expect(screen.getByText("Quick Actions & Shortcuts")).toBeInTheDocument();
      expect(screen.getByText(/Press \? anywhere/)).toBeInTheDocument();
    });

    it("step 6: Your Identity — DID description", async () => {
      await goToStep(5);
      expect(screen.getByText("Your Identity")).toBeInTheDocument();
      expect(screen.getByText(/DID.*Decentralized Identifier/)).toBeInTheDocument();
    });
  });

  describe("markTourCompleted and resetTour", () => {
    it("markTourCompleted sets localStorage flag", () => {
      markTourCompleted();
      expect(localStorage.getItem("nous_tour_completed")).toBe("true");
    });

    it("resetTour clears localStorage flag", () => {
      markTourCompleted();
      expect(localStorage.getItem("nous_tour_completed")).toBe("true");

      resetTour();
      expect(localStorage.getItem("nous_tour_completed")).toBeNull();
    });

    it("tour starts after resetTour", async () => {
      markTourCompleted();

      // Tour should not start
      const { unmount } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
      expect(screen.queryByText("Navigate Your Node")).not.toBeInTheDocument();
      unmount();

      // Reset and re-render
      resetTour();
      renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });
      expect(screen.getByText("Navigate Your Node")).toBeInTheDocument();
    });
  });

  describe("Full-screen overlay", () => {
    it("renders with fixed positioning and z-300", async () => {
      const { container } = renderTour();
      await act(async () => {
        vi.advanceTimersByTime(800);
      });

      const overlay = container.querySelector(".fixed.inset-0.z-\\[300\\]");
      expect(overlay).toBeInTheDocument();
    });
  });
});
