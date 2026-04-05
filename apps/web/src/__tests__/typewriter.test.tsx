import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { TypeWriter } from "@/components/typewriter";

beforeEach(() => {
  vi.useFakeTimers();
  // Default: no reduced motion
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("TypeWriter", () => {
  it("renders nothing when phrases array is empty", () => {
    const { container } = render(<TypeWriter phrases={[]} />);
    expect(container.innerHTML).toBe("");
  });

  it("starts typing the first phrase", () => {
    render(<TypeWriter phrases={["Hello world"]} typeSpeed={50} />);
    // Initial state: empty text + cursor, aria-label set to current phrase
    const el = screen.getByLabelText("Hello world");
    expect(el).toBeDefined();
    expect(el.getAttribute("aria-live")).toBe("polite");
  });

  it("types characters one at a time", () => {
    render(<TypeWriter phrases={["ABC"]} typeSpeed={50} />);

    // After first tick, should have "A"
    act(() => { vi.advanceTimersByTime(50); });
    expect(screen.getByLabelText("ABC").textContent).toContain("A");

    // After second tick, should have "AB"
    act(() => { vi.advanceTimersByTime(50); });
    expect(screen.getByLabelText("ABC").textContent).toContain("AB");

    // After third tick, should have "ABC"
    act(() => { vi.advanceTimersByTime(50); });
    expect(screen.getByLabelText("ABC").textContent).toContain("ABC");
  });

  it("shows blinking cursor", () => {
    render(<TypeWriter phrases={["Test"]} />);
    act(() => { vi.advanceTimersByTime(50); });
    const cursor = document.querySelector(".typewriter-cursor");
    expect(cursor).not.toBeNull();
    expect(cursor?.textContent).toBe("|");
    expect(cursor?.getAttribute("aria-hidden")).toBe("true");
  });

  it("erases after typing is complete", () => {
    render(
      <TypeWriter
        phrases={["Hi", "Bye"]}
        typeSpeed={50}
        eraseSpeed={30}
        pauseAfterType={100}
        pauseAfterErase={100}
      />
    );

    // Type "H" then "Hi" — each char triggers a separate timeout
    act(() => { vi.advanceTimersByTime(50); }); // "H"
    act(() => { vi.advanceTimersByTime(50); }); // "Hi"
    expect(screen.getByLabelText("Hi").textContent).toContain("Hi");

    // Wait for pauseAfterType — triggers erase mode
    act(() => { vi.advanceTimersByTime(100); });

    // Erase "Hi" → "H" → ""
    act(() => { vi.advanceTimersByTime(30); }); // "H"
    act(() => { vi.advanceTimersByTime(30); }); // ""

    // Text should be erased (just cursor remains)
    const el = screen.getByLabelText("Hi");
    expect(el.textContent).toBe("|");
  });

  it("moves to the next phrase after erasing", () => {
    render(
      <TypeWriter
        phrases={["A", "B"]}
        typeSpeed={50}
        eraseSpeed={30}
        pauseAfterType={100}
        pauseAfterErase={100}
      />
    );

    // Type "A" (1 char × 50ms)
    act(() => { vi.advanceTimersByTime(50); });
    expect(screen.getByLabelText("A").textContent).toContain("A");

    // pauseAfterType
    act(() => { vi.advanceTimersByTime(100); });

    // Erase "A" (1 char × 30ms)
    act(() => { vi.advanceTimersByTime(30); });

    // pauseAfterErase — triggers next phrase
    act(() => { vi.advanceTimersByTime(100); });

    // Now typing "B" (1 char × 50ms)
    act(() => { vi.advanceTimersByTime(50); });
    expect(screen.getByLabelText("B").textContent).toContain("B");
  });

  it("applies custom className", () => {
    render(<TypeWriter phrases={["Test"]} className="my-custom-class" />);
    const el = screen.getByLabelText("Test");
    expect(el.classList.contains("my-custom-class")).toBe(true);
  });

  it("has aria-live for accessibility", () => {
    render(<TypeWriter phrases={["Hello"]} />);
    const el = screen.getByLabelText("Hello");
    expect(el.getAttribute("aria-live")).toBe("polite");
  });
});

describe("TypeWriter — reduced motion", () => {
  beforeEach(() => {
    Object.defineProperty(window, "matchMedia", {
      writable: true,
      value: vi.fn().mockImplementation((query: string) => ({
        matches: query === "(prefers-reduced-motion: reduce)",
        media: query,
        onchange: null,
        addListener: vi.fn(),
        removeListener: vi.fn(),
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });
  });

  it("shows phrases statically without typing animation", () => {
    render(<TypeWriter phrases={["Static phrase"]} />);
    // With reduced motion, the full phrase should be shown immediately
    expect(screen.getByText("Static phrase")).toBeDefined();
  });

  it("does not show cursor in reduced motion mode", () => {
    render(<TypeWriter phrases={["No cursor"]} />);
    const cursor = document.querySelector(".typewriter-cursor");
    expect(cursor).toBeNull();
  });

  it("crossfades between phrases over time", () => {
    render(<TypeWriter phrases={["First", "Second"]} />);
    expect(screen.getByText("First")).toBeDefined();

    // Advance past the crossfade interval (3000ms)
    act(() => { vi.advanceTimersByTime(3000); });
    expect(screen.getByText("Second")).toBeDefined();
  });

  it("has aria-live in reduced motion mode", () => {
    render(<TypeWriter phrases={["Accessible"]} />);
    const el = screen.getByText("Accessible");
    expect(el.getAttribute("aria-live")).toBe("polite");
  });
});
