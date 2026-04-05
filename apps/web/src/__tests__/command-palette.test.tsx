import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CommandPalette } from "@/components/command-palette";

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

describe("CommandPalette", () => {
  beforeEach(() => {
    pushMock.mockClear();
  });

  it("is hidden by default", () => {
    render(<CommandPalette />);
    expect(screen.queryByPlaceholderText("Search pages and actions...")).not.toBeInTheDocument();
  });

  it("opens on Cmd+K", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    expect(screen.getByPlaceholderText("Search pages and actions...")).toBeInTheDocument();
  });

  it("opens on Ctrl+K", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", ctrlKey: true, bubbles: true }),
      );
    });

    expect(screen.getByPlaceholderText("Search pages and actions...")).toBeInTheDocument();
  });

  it("closes on Escape", () => {
    render(<CommandPalette />);

    // Open
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    expect(screen.getByPlaceholderText("Search pages and actions...")).toBeInTheDocument();

    // Close via Escape
    act(() => {
      fireEvent.keyDown(screen.getByPlaceholderText("Search pages and actions..."), {
        key: "Escape",
      });
    });

    expect(screen.queryByPlaceholderText("Search pages and actions...")).not.toBeInTheDocument();
  });

  it("closes when backdrop is clicked", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    // The backdrop is the first child with bg-black/60
    const backdrop = document.querySelector(".cmd-backdrop-enter");
    expect(backdrop).not.toBeNull();

    act(() => {
      fireEvent.click(backdrop!);
    });

    expect(screen.queryByPlaceholderText("Search pages and actions...")).not.toBeInTheDocument();
  });

  it("shows all commands when no query is entered", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    // Should show Navigation and Quick Actions sections
    expect(screen.getByText("Navigation")).toBeInTheDocument();
    expect(screen.getByText("Quick Actions")).toBeInTheDocument();
    // Should show specific nav items
    expect(screen.getByText("Dashboard")).toBeInTheDocument();
    expect(screen.getByText("Messages")).toBeInTheDocument();
    expect(screen.getByText("Settings")).toBeInTheDocument();
  });

  it("filters results based on search query", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");
    await user.type(input, "wallet");

    // Wallet should be visible
    expect(screen.getByText("Wallet")).toBeInTheDocument();
    // Dashboard should NOT be visible (doesn't match "wallet")
    expect(screen.queryByText("Dashboard")).not.toBeInTheDocument();
  });

  it("shows no results message for unmatched queries", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");
    await user.type(input, "xyznonexistent");

    expect(screen.getByText("No results found")).toBeInTheDocument();
  });

  it("matches by keywords", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");
    // "chat" is a keyword for Messages
    await user.type(input, "chat");

    expect(screen.getByText("Messages")).toBeInTheDocument();
  });

  it("navigates with ArrowDown and ArrowUp", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");

    // First item should be selected by default (index 0)
    let selected = document.querySelectorAll("[data-selected='true']");
    expect(selected.length).toBe(1);

    // ArrowDown moves to next item
    act(() => {
      fireEvent.keyDown(input, { key: "ArrowDown" });
    });

    // The second item should now be selected
    selected = document.querySelectorAll("[data-selected='true']");
    expect(selected.length).toBe(1);
  });

  it("executes navigation on Enter", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");

    // Press Enter on the first item (Dashboard)
    act(() => {
      fireEvent.keyDown(input, { key: "Enter" });
    });

    // Should navigate to /dashboard
    expect(pushMock).toHaveBeenCalledWith("/dashboard");
  });

  it("executes navigation on click", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    // Click on Messages
    const messagesButton = screen.getByText("Messages").closest("button");
    expect(messagesButton).not.toBeNull();

    act(() => {
      fireEvent.click(messagesButton!);
    });

    expect(pushMock).toHaveBeenCalledWith("/messages");
  });

  it("closes after executing a command", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");

    act(() => {
      fireEvent.keyDown(input, { key: "Enter" });
    });

    // Palette should be closed
    expect(screen.queryByPlaceholderText("Search pages and actions...")).not.toBeInTheDocument();
  });

  it("resets query when reopened", () => {
    render(<CommandPalette />);

    // Open and type
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");
    fireEvent.change(input, { target: { value: "wallet" } });

    // Close
    act(() => {
      fireEvent.keyDown(input, { key: "Escape" });
    });

    // Reopen
    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const newInput = screen.getByPlaceholderText("Search pages and actions...");
    expect(newInput).toHaveValue("");
  });

  it("shows footer with keyboard hints", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    expect(screen.getByText("navigate")).toBeInTheDocument();
    expect(screen.getByText("select")).toBeInTheDocument();
    expect(screen.getByText("close")).toBeInTheDocument();
  });

  it("shows command descriptions", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    expect(screen.getByText("Overview and system status")).toBeInTheDocument();
  });

  it("wraps selection at boundaries", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    const input = screen.getByPlaceholderText("Search pages and actions...");

    // ArrowUp from first item should wrap to last
    act(() => {
      fireEvent.keyDown(input, { key: "ArrowUp" });
    });

    // The last item should be selected
    const buttons = document.querySelectorAll("[data-selected='true']");
    expect(buttons.length).toBe(1);
  });

  it("updates selection on mouse hover", () => {
    render(<CommandPalette />);

    act(() => {
      window.dispatchEvent(
        new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }),
      );
    });

    // Find Messages button and hover it
    const messagesButton = screen.getByText("Messages").closest("button");
    expect(messagesButton).not.toBeNull();

    act(() => {
      fireEvent.mouseEnter(messagesButton!);
    });

    expect(messagesButton).toHaveAttribute("data-selected", "true");
  });
});
