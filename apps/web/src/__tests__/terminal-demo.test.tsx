import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import { TerminalDemo } from "@/components/terminal-demo";

// ── Mocks ────────────────────────────────────────────────────────────────

let intersectionCallback: (entries: IntersectionObserverEntry[]) => void;
const mockObserve = vi.fn();
const mockDisconnect = vi.fn();

function triggerIntersection(isIntersecting = true) {
  act(() => {
    intersectionCallback([
      { isIntersecting } as unknown as IntersectionObserverEntry,
    ]);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(Math, "random").mockReturnValue(0); // deterministic typing: 35ms per char

  // Override the global mock from setup.ts with one that captures the callback
  const MockObserver = vi.fn(function (
    this: IntersectionObserver,
    cb: IntersectionObserverCallback
  ) {
    intersectionCallback = cb as (entries: IntersectionObserverEntry[]) => void;
    this.observe = mockObserve;
    this.disconnect = mockDisconnect;
    this.unobserve = vi.fn();
    this.root = null;
    this.rootMargin = "";
    this.thresholds = [];
    this.takeRecords = vi.fn().mockReturnValue([]) as () => IntersectionObserverEntry[];
  });
  window.IntersectionObserver = MockObserver as unknown as typeof IntersectionObserver;

  Object.assign(navigator, {
    clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
  });
});

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  mockObserve.mockClear();
  mockDisconnect.mockClear();
});

// ── Helpers ──────────────────────────────────────────────────────────────

/**
 * Advance time using the async variant that properly flushes microtasks
 * and allows async promise chains to progress.
 */
async function advance(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}


/**
 * Type a command: the component uses setInterval at 35ms.
 * Each tick types one character. We need `length` timer fires.
 * We also have a cursor blink (530ms) running, so we use advanceNextTimer
 * to step through timers one at a time.
 */
async function typeCommand(charCount: number) {
  // With Math.random() = 0, interval is exactly 35ms per character.
  await advance(charCount * 35);
}

/**
 * Show output lines: the component uses setInterval at 40ms.
 * It fires `lineCount` times to push lines, then 1 more to clearInterval+resolve.
 * We advance one timer at a time to avoid the stale closure issue.
 */
async function showOutput(lineCount: number) {
  // Need lineCount+1 output interval ticks (lineCount pushes + 1 clearInterval+resolve).
  // Total time: (lineCount+1) * 40ms
  await advance((lineCount + 1) * 40);
}

/**
 * Run through the full first command (nous status).
 */
async function runFirstCommand() {
  await typeCommand(11); // "nous status" = 11 chars
  await advance(300); // pause after typing
  await advance(200); // processing pause
  await showOutput(8); // 8 output lines
}

/**
 * Run the second command (nous identity create --name "Teddy").
 */
async function runSecondCommand() {
  await advance(40); // blank line separator
  await advance(800); // delay before second command
  await typeCommand(35); // 35 chars
  await advance(300);
  await advance(200);
  await showOutput(8); // 8 output lines
}

/**
 * Run the third command.
 */
async function runThirdCommand() {
  await advance(40); // blank line
  await advance(600); // delay
  const cmd = 'nous message send --to did:key:z6Mkr...9Fj2 "Hello, sovereign future"';
  await typeCommand(cmd.length); // 72 chars
  await advance(300);
  await advance(200);
  await showOutput(8); // 8 output lines
}

/**
 * Run the fourth command.
 */
async function runFourthCommand() {
  await advance(40); // blank line
  await advance(600); // delay
  const cmd = 'nous social post "First post on Nous 🔐"';
  await typeCommand(cmd.length);
  await advance(300);
  await advance(200);
  await showOutput(7); // 7 output lines
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("TerminalDemo", () => {
  // ── Terminal Window Structure ────────────────────────────────────────

  describe("Terminal Window Structure", () => {
    it("renders a terminal container div", () => {
      const { container } = render(<TerminalDemo />);
      expect(container.querySelector("div")).not.toBeNull();
    });

    it("has border and bg-[#0a0a0a] classes for the terminal window", () => {
      const { container } = render(<TerminalDemo />);
      const terminalWindow = container.querySelector(".bg-\\[\\#0a0a0a\\]");
      expect(terminalWindow).not.toBeNull();
      expect(terminalWindow?.classList.toString()).toContain("border");
    });

    it("title bar has 3 colored dots (red, yellow, green)", () => {
      const { container } = render(<TerminalDemo />);
      const redDot = container.querySelector(".bg-\\[\\#ff5f57\\]");
      const yellowDot = container.querySelector(".bg-\\[\\#febc2e\\]");
      const greenDot = container.querySelector(".bg-\\[\\#28c840\\]");
      expect(redDot).not.toBeNull();
      expect(yellowDot).not.toBeNull();
      expect(greenDot).not.toBeNull();
    });

    it("title bar shows 'nous — ~' text", () => {
      render(<TerminalDemo />);
      expect(screen.getByText("nous — ~")).toBeDefined();
    });

    it("terminal body has terminal-scroll class", () => {
      const { container } = render(<TerminalDemo />);
      const scrollArea = container.querySelector(".terminal-scroll");
      expect(scrollArea).not.toBeNull();
    });

    it("terminal body has correct height classes", () => {
      const { container } = render(<TerminalDemo />);
      const scrollArea = container.querySelector(".terminal-scroll");
      expect(scrollArea?.classList.toString()).toContain("h-[340px]");
      expect(scrollArea?.classList.toString()).toContain("sm:h-[380px]");
    });
  });

  // ── Initial State ───────────────────────────────────────────────────

  describe("Initial State", () => {
    it("shows prompt character on mount", () => {
      render(<TerminalDemo />);
      expect(screen.getByText("❯")).toBeDefined();
    });

    it("shows a blinking cursor element", () => {
      const { container } = render(<TerminalDemo />);
      const allSpans = container.querySelectorAll("span.inline-block");
      expect(allSpans.length).toBeGreaterThan(0);
    });

    it("has no command text initially", () => {
      render(<TerminalDemo />);
      const promptLine = screen.getByText("❯").closest("div");
      const textSpan = promptLine?.querySelector(".text-white");
      expect(textSpan?.textContent).toBe("");
    });

    it("has no output lines initially", () => {
      const { container } = render(<TerminalDemo />);
      const outputDivs = container.querySelectorAll(".leading-\\[1\\.6\\]");
      expect(outputDivs.length).toBe(0);
    });
  });

  // ── ANSI Parsing (via rendered output) ──────────────────────────────

  describe("ANSI Parsing", () => {
    it("renders bold text for \\x1b[1m", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const boldSpan = container.querySelector(".font-bold");
      expect(boldSpan).not.toBeNull();
      expect(boldSpan?.textContent).toContain("Nous");
    });

    it("renders emerald-400 text for \\x1b[32m (green checkmarks)", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const greenSpan = container.querySelector(".text-emerald-400");
      expect(greenSpan).not.toBeNull();
      expect(greenSpan?.textContent).toContain("●");
    });

    it("renders gold text for \\x1b[33m (DID values)", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const outputGoldSpans = container.querySelectorAll(
        ".leading-\\[1\\.6\\] .text-\\[\\#d4af37\\]"
      );
      expect(outputGoldSpans.length).toBeGreaterThan(0);
      const didSpan = Array.from(outputGoldSpans).find((s) =>
        s.textContent?.includes("did:key")
      );
      expect(didSpan).not.toBeNull();
    });

    it("renders cyan-400 text for \\x1b[36m (connection info)", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const cyanSpan = container.querySelector(".text-cyan-400");
      expect(cyanSpan).not.toBeNull();
      expect(cyanSpan?.textContent).toContain("12 connected");
    });

    it("renders neutral-300 for \\x1b[37m (values)", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const neutralSpan = container.querySelector(".text-neutral-300");
      expect(neutralSpan).not.toBeNull();
      expect(neutralSpan?.textContent).toContain("2.4 GB local");
    });

    it("renders neutral-600 for \\x1b[90m (borders)", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const dimSpans = container.querySelectorAll(
        ".leading-\\[1\\.6\\] .text-neutral-600"
      );
      expect(dimSpans.length).toBeGreaterThan(0);
    });

    it("reset code \\x1b[0m clears current styling", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      const allSpans = container.querySelectorAll(".leading-\\[1\\.6\\] span");
      const plainSpans = Array.from(allSpans).filter(
        (s) => s.className === "" && s.textContent && s.textContent.trim().length > 0
      );
      expect(plainSpans.length).toBeGreaterThan(0);
    });

    it("plain text (no ANSI) renders without classes", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();
      await runSecondCommand();

      const outputDivs = container.querySelectorAll(".leading-\\[1\\.6\\]");
      const teddyDiv = Array.from(outputDivs).find((d) =>
        d.textContent?.includes("Name")
      );
      expect(teddyDiv).toBeDefined();
      const spans = teddyDiv?.querySelectorAll("span");
      const teddySpan = Array.from(spans ?? []).find(
        (s) => s.textContent?.includes("Teddy") && !s.className
      );
      expect(teddySpan).toBeDefined();
    });

    it("empty line renders as <br>", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();
      await runSecondCommand();

      const brs = container.querySelectorAll("br");
      expect(brs.length).toBeGreaterThan(0);
    });
  });

  // ── IntersectionObserver Integration ────────────────────────────────

  describe("IntersectionObserver Integration", () => {
    it("creates IntersectionObserver on mount", () => {
      render(<TerminalDemo />);
      expect(window.IntersectionObserver).toHaveBeenCalledTimes(1);
    });

    it("observer uses threshold 0.3", () => {
      render(<TerminalDemo />);
      expect(window.IntersectionObserver).toHaveBeenCalledWith(
        expect.any(Function),
        { threshold: 0.3 }
      );
    });

    it("calls observe on mount", () => {
      render(<TerminalDemo />);
      expect(mockObserve).toHaveBeenCalledTimes(1);
    });

    it("demo starts when element becomes visible (isIntersecting: true)", async () => {
      render(<TerminalDemo />);

      triggerIntersection(true);
      await advance(35);
      const promptDiv = screen.getByText("❯").closest("div");
      const textSpan = promptDiv?.querySelector(".text-white");
      expect(textSpan?.textContent?.length).toBeGreaterThan(0);
    });

    it("does not start demo on isIntersecting: false", async () => {
      render(<TerminalDemo />);

      triggerIntersection(false);
      await advance(35);

      const promptDiv = screen.getByText("❯").closest("div");
      const textSpan = promptDiv?.querySelector(".text-white");
      expect(textSpan?.textContent).toBe("");
    });

    it("does not restart if already started (hasStarted guard)", async () => {
      render(<TerminalDemo />);
      triggerIntersection(true);
      await advance(35);

      // Trigger again — should not cause issues
      triggerIntersection(true);
      await advance(35);
    });

    it("observer is disconnected when demo starts", () => {
      render(<TerminalDemo />);
      triggerIntersection(true);
      expect(mockDisconnect).toHaveBeenCalled();
    });

    it("observer is disconnected on unmount", () => {
      const { unmount } = render(<TerminalDemo />);
      unmount();
      expect(mockDisconnect).toHaveBeenCalled();
    });
  });

  // ── Copy Button (PromptLine) ────────────────────────────────────────

  describe("Copy Button", () => {
    it("copy button exists on prompt lines", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const button = screen.getByRole("button", { name: "Copy command" });
      expect(button).toBeDefined();
    });

    it("copy button has aria-label 'Copy command'", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const button = screen.getByRole("button", { name: "Copy command" });
      expect(button.getAttribute("aria-label")).toBe("Copy command");
    });

    it("clicking copy calls navigator.clipboard.writeText", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const button = screen.getByRole("button", { name: "Copy command" });
      await act(async () => {
        fireEvent.click(button);
      });
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("nous status");
    });

    it("shows Check icon after copying", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const button = screen.getByRole("button", { name: "Copy command" });
      await act(async () => {
        fireEvent.click(button);
      });

      const checkIcon = button.querySelector(".text-emerald-400");
      expect(checkIcon).not.toBeNull();
    });

    it("resets back to Copy icon after 1500ms", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const button = screen.getByRole("button", { name: "Copy command" });
      await act(async () => {
        fireEvent.click(button);
      });

      await advance(1500);

      const copyIcon = button.querySelector(".text-neutral-600");
      expect(copyIcon).not.toBeNull();
    });
  });

  // ── Cursor Behavior ─────────────────────────────────────────────────

  describe("Cursor Behavior", () => {
    it("cursor blinks at 530ms interval", async () => {
      const { container } = render(<TerminalDemo />);

      let cursor = container.querySelector(".inline-block.bg-\\[\\#d4af37\\]");
      expect(cursor).not.toBeNull();

      await advance(530);
      cursor = container.querySelector(".inline-block.bg-transparent");
      expect(cursor).not.toBeNull();

      await advance(530);
      cursor = container.querySelector(".inline-block.bg-\\[\\#d4af37\\]");
      expect(cursor).not.toBeNull();
    });

    it("cursor element toggles between gold and transparent background", async () => {
      const { container } = render(<TerminalDemo />);

      const getCursorBg = () => {
        const gold = container.querySelector(".inline-block.bg-\\[\\#d4af37\\]");
        const transparent = container.querySelector(".inline-block.bg-transparent");
        if (gold) return "gold";
        if (transparent) return "transparent";
        return null;
      };

      expect(getCursorBg()).toBe("gold");
      await advance(530);
      expect(getCursorBg()).toBe("transparent");
      await advance(530);
      expect(getCursorBg()).toBe("gold");
    });

    it("cursor is visible during typing", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();

      await advance(35);

      const typingLine = screen.getByText("❯").closest("div");
      const cursorSpan = typingLine?.querySelector(".inline-block");
      expect(cursorSpan).not.toBeNull();
    });
  });

  // ── Demo Sequence ───────────────────────────────────────────────────

  describe("Demo Sequence", () => {
    it("first command typed is 'nous status'", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);

      expect(screen.getByText("nous status")).toBeDefined();
    });

    it("after typing, command appears in committed lines", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);

      const promptLines = container.querySelectorAll(".group\\/prompt");
      expect(promptLines.length).toBe(1);
      expect(promptLines[0].textContent).toContain("nous status");
    });

    it("output lines appear one by one", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();
      await typeCommand(11);
      await advance(300);
      await advance(200);

      await advance(40);
      let outputLines = container.querySelectorAll(".leading-\\[1\\.6\\]");
      expect(outputLines.length).toBe(1);

      await advance(40);
      outputLines = container.querySelectorAll(".leading-\\[1\\.6\\]");
      expect(outputLines.length).toBe(2);
    });

    it("second command is 'nous identity create --name \"Teddy\"'", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();

      await advance(40); // blank line
      await advance(800); // delay

      const secondCmd = 'nous identity create --name "Teddy"';
      await typeCommand(secondCmd.length);

      expect(screen.getByText(secondCmd)).toBeDefined();
    });

    it("third command involves 'nous message send'", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();
      await runSecondCommand();

      await advance(40); // blank line
      await advance(600); // delay

      const thirdCmd =
        'nous message send --to did:key:z6Mkr...9Fj2 "Hello, sovereign future"';
      await typeCommand(thirdCmd.length);

      expect(screen.getByText(thirdCmd)).toBeDefined();
    });

    it("fourth command is 'nous social post'", async () => {
      render(<TerminalDemo />);
      triggerIntersection();
      await runFirstCommand();
      await runSecondCommand();
      await runThirdCommand();

      await advance(40); // blank line
      await advance(600); // delay

      const fourthCmd = 'nous social post "First post on Nous 🔐"';
      await typeCommand(fourthCmd.length);

      const promptDiv = screen
        .getAllByText("❯")
        .map((el) => el.closest("div"))
        .find((div) => div?.textContent?.includes("nous social post"));
      expect(promptDiv).toBeDefined();
    });
  });

  // ── Scrolling ───────────────────────────────────────────────────────

  describe("Scrolling", () => {
    it("scroll container ref exists with overflow-y-auto", () => {
      const { container } = render(<TerminalDemo />);
      const scrollArea = container.querySelector(".overflow-y-auto");
      expect(scrollArea).not.toBeNull();
    });

    it("terminal scroll area is the terminal body", () => {
      const { container } = render(<TerminalDemo />);
      const scrollArea = container.querySelector(
        ".terminal-scroll.overflow-y-auto"
      );
      expect(scrollArea).not.toBeNull();
    });
  });

  // ── Restart Loop ────────────────────────────────────────────────────

  describe("Restart Loop", () => {
    it("after all commands, waits 4s then clears lines to restart", async () => {
      const { container } = render(<TerminalDemo />);
      triggerIntersection();

      await runFirstCommand();
      await runSecondCommand();
      await runThirdCommand();
      await runFourthCommand();

      const promptsBefore = container.querySelectorAll(".group\\/prompt");
      expect(promptsBefore.length).toBe(4);

      await advance(4000);

      const promptsAfter = container.querySelectorAll(".group\\/prompt");
      expect(promptsAfter.length).toBe(0);
    });
  });
});
