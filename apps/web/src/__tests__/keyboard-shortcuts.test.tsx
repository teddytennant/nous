import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, renderHook } from "@testing-library/react";
import {
  KeyboardShortcutsModal,
  KeyboardShortcutsProvider,
  useKeyboardShortcuts,
  usePageShortcuts,
  useListNavigation,
} from "@/components/keyboard-shortcuts";

// Mock useRouter
const pushMock = vi.fn();
vi.mock("next/navigation", async () => {
  const actual = await vi.importActual("next/navigation");
  return {
    ...actual,
    useRouter: () => ({
      push: pushMock,
      replace: vi.fn(),
      back: vi.fn(),
      forward: vi.fn(),
      refresh: vi.fn(),
      prefetch: vi.fn(),
    }),
    usePathname: () => "/dashboard",
  };
});

describe("KeyboardShortcutsModal", () => {
  it("renders when open is true", () => {
    render(<KeyboardShortcutsModal open={true} onClose={vi.fn()} />);

    expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
    expect(screen.getByText("General")).toBeInTheDocument();
    expect(screen.getByText("Navigation")).toBeInTheDocument();
    expect(screen.getByText("Lists")).toBeInTheDocument();
  });

  it("does not render when open is false", () => {
    render(<KeyboardShortcutsModal open={false} onClose={vi.fn()} />);

    expect(screen.queryByText("Keyboard Shortcuts")).not.toBeInTheDocument();
  });

  it("shows shortcut labels", () => {
    render(<KeyboardShortcutsModal open={true} onClose={vi.fn()} />);

    expect(screen.getByText("Command palette")).toBeInTheDocument();
    expect(screen.getByText("Keyboard shortcuts")).toBeInTheDocument();
    expect(screen.getByText("Go to Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Go to Messages")).toBeInTheDocument();
  });

  it("calls onClose when Escape is pressed", () => {
    const onClose = vi.fn();
    render(<KeyboardShortcutsModal open={true} onClose={onClose} />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      );
    });

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("calls onClose when backdrop is clicked", () => {
    const onClose = vi.fn();
    render(<KeyboardShortcutsModal open={true} onClose={onClose} />);

    const backdrop = document.querySelector(".cmd-backdrop-enter");
    expect(backdrop).not.toBeNull();

    act(() => {
      backdrop!.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).toHaveBeenCalled();
  });

  it("shows ESC key hint", () => {
    render(<KeyboardShortcutsModal open={true} onClose={vi.fn()} />);

    // ESC key indicator in header
    const escKeys = screen.getAllByText("ESC");
    expect(escKeys.length).toBeGreaterThanOrEqual(1);
  });
});

describe("useKeyboardShortcuts", () => {
  beforeEach(() => {
    pushMock.mockClear();
  });

  it("opens help modal on ? key", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "?", bubbles: true }),
      );
    });

    expect(onOpenHelp).toHaveBeenCalledTimes(1);
  });

  it("navigates to dashboard on G then D", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <div>Test</div>;
    }

    render(<TestComponent />);

    // Press G
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "g", bubbles: true }),
      );
    });

    // Then press D
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "d", bubbles: true }),
      );
    });

    expect(pushMock).toHaveBeenCalledWith("/dashboard");
  });

  it("navigates to messages on G then M", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "g", bubbles: true }),
      );
    });

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "m", bubbles: true }),
      );
    });

    expect(pushMock).toHaveBeenCalledWith("/messages");
  });

  it("navigates to wallet on G then W", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "g", bubbles: true }),
      );
    });

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "w", bubbles: true }),
      );
    });

    expect(pushMock).toHaveBeenCalledWith("/wallet");
  });

  it("ignores shortcuts when input is focused", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <input data-testid="input" />;
    }

    render(<TestComponent />);

    const input = screen.getByTestId("input");
    input.focus();

    act(() => {
      input.dispatchEvent(
        new KeyboardEvent("keydown", { key: "?", bubbles: true }),
      );
    });

    expect(onOpenHelp).not.toHaveBeenCalled();
  });

  it("ignores shortcuts with meta key held", () => {
    const onOpenHelp = vi.fn();

    function TestComponent() {
      useKeyboardShortcuts(onOpenHelp);
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "?", metaKey: true, bubbles: true }),
      );
    });

    expect(onOpenHelp).not.toHaveBeenCalled();
  });
});

describe("usePageShortcuts", () => {
  it("calls handler for matching key", () => {
    const handler = vi.fn();

    function TestComponent() {
      usePageShortcuts({ n: handler });
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "n", bubbles: true }),
      );
    });

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("ignores non-matching keys", () => {
    const handler = vi.fn();

    function TestComponent() {
      usePageShortcuts({ n: handler });
      return <div>Test</div>;
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "x", bubbles: true }),
      );
    });

    expect(handler).not.toHaveBeenCalled();
  });

  it("ignores when input is focused", () => {
    const handler = vi.fn();

    function TestComponent() {
      usePageShortcuts({ n: handler });
      return <input data-testid="input" />;
    }

    render(<TestComponent />);

    const input = screen.getByTestId("input");
    input.focus();

    act(() => {
      input.dispatchEvent(
        new KeyboardEvent("keydown", { key: "n", bubbles: true }),
      );
    });

    expect(handler).not.toHaveBeenCalled();
  });
});

describe("useListNavigation", () => {
  it("starts with no selection", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 5 });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2, 3, 4].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);
    expect(screen.getByTestId("index").textContent).toBe("-1");
  });

  it("moves down on J key", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 5 });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2, 3, 4].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "j", bubbles: true }),
      );
    });

    expect(screen.getByTestId("index").textContent).toBe("0");

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "j", bubbles: true }),
      );
    });

    expect(screen.getByTestId("index").textContent).toBe("1");
  });

  it("moves up on K key", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 5 });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2, 3, 4].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);

    // Move down twice
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
    });
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true }));
    });

    expect(screen.getByTestId("index").textContent).toBe("1");

    // Move up
    act(() => {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", bubbles: true }));
    });

    expect(screen.getByTestId("index").textContent).toBe("0");
  });

  it("wraps around at boundaries", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 3 });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);

    // Move to end
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });

    expect(screen.getByTestId("index").textContent).toBe("2");

    // One more J should wrap to 0
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });

    expect(screen.getByTestId("index").textContent).toBe("0");
  });

  it("calls onActivate on Enter", () => {
    const onActivate = vi.fn();

    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({
        itemCount: 3,
        onActivate,
      });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);

    // Select first item
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });

    // Press Enter
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })); });

    expect(onActivate).toHaveBeenCalledWith(0);
  });

  it("clears selection on Escape", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 3 });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
          {[0, 1, 2].map((i) => (
            <div key={i} data-list-item>
              Item {i}
            </div>
          ))}
        </div>
      );
    }

    render(<TestComponent />);

    // Select first item
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });
    expect(screen.getByTestId("index").textContent).toBe("0");

    // Escape to clear
    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
    expect(screen.getByTestId("index").textContent).toBe("-1");
  });

  it("does not respond when disabled", () => {
    function TestComponent() {
      const { selectedIndex, containerRef } = useListNavigation({ itemCount: 3, enabled: false });
      return (
        <div ref={containerRef}>
          <span data-testid="index">{selectedIndex}</span>
        </div>
      );
    }

    render(<TestComponent />);

    act(() => { window.dispatchEvent(new KeyboardEvent("keydown", { key: "j", bubbles: true })); });
    expect(screen.getByTestId("index").textContent).toBe("-1");
  });
});

describe("KeyboardShortcutsProvider", () => {
  it("renders without crashing", () => {
    render(<KeyboardShortcutsProvider />);
    // Modal should be closed by default
    expect(screen.queryByText("Keyboard Shortcuts")).not.toBeInTheDocument();
  });

  it("opens help modal on ? key press", () => {
    render(<KeyboardShortcutsProvider />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "?", bubbles: true }),
      );
    });

    expect(screen.getByText("Keyboard Shortcuts")).toBeInTheDocument();
  });
});
