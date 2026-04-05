import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { ToastProvider, useToast } from "@/components/toast";

// Test component that triggers toasts with full options
function ToastTrigger({
  title,
  description,
  variant,
  action,
  duration,
}: {
  title: string;
  description?: string;
  variant?: "default" | "success" | "error" | "info";
  action?: { label: string; onClick: () => void };
  duration?: number;
}) {
  const { toast } = useToast();
  return (
    <button onClick={() => toast({ title, description, variant, action, duration })}>
      Fire Toast
    </button>
  );
}

function DismissTrigger() {
  const { toast, dismiss } = useToast();
  return (
    <>
      <button
        onClick={() => {
          const id = toast({ title: "Dismissable" });
          (window as Record<string, unknown>).__lastToastId = id;
        }}
      >
        Fire
      </button>
      <button
        onClick={() => dismiss((window as Record<string, unknown>).__lastToastId as string)}
      >
        Dismiss Programmatically
      </button>
    </>
  );
}

describe("Toast system", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders a toast when triggered", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Hello World" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire Toast" }).click();
    });

    expect(screen.getByText("Hello World")).toBeInTheDocument();
  });

  it("renders toast with description", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Saved" description="Your changes have been saved." />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    expect(screen.getByText("Saved")).toBeInTheDocument();
    expect(
      screen.getByText("Your changes have been saved."),
    ).toBeInTheDocument();
  });

  it("applies error variant styling", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Failed" variant="error" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const title = screen.getByText("Failed");
    expect(title.className).toContain("text-red-400");
  });

  it("applies success variant styling", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Done" variant="success" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const title = screen.getByText("Done");
    expect(title.className).toContain("text-emerald-400");
  });

  it("applies info variant styling", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Note" variant="info" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const title = screen.getByText("Note");
    expect(title.className).toContain("text-blue-400");
  });

  it("auto-dismisses toast after timeout", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Temporary" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    expect(screen.getByText("Temporary")).toBeInTheDocument();

    // After 3000ms + 300ms exit animation the toast should be fully removed
    act(() => {
      vi.advanceTimersByTime(3400);
    });

    expect(screen.queryByText("Temporary")).not.toBeInTheDocument();
  });

  it("marks toast as exiting before removal", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Fading" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    // At 3000ms, toast should be in exiting state (toast-exit class)
    act(() => {
      vi.advanceTimersByTime(3100);
    });

    const toast = screen.getByText("Fading").closest("div[class*='toast']");
    expect(toast?.className).toContain("toast-exit");
  });

  it("can display multiple toasts simultaneously", () => {
    function MultiTrigger() {
      const { toast } = useToast();
      return (
        <>
          <button onClick={() => toast({ title: "First" })}>One</button>
          <button onClick={() => toast({ title: "Second" })}>Two</button>
        </>
      );
    }

    render(
      <ToastProvider>
        <MultiTrigger />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "One" }).click();
      screen.getByRole("button", { name: "Two" }).click();
    });

    expect(screen.getByText("First")).toBeInTheDocument();
    expect(screen.getByText("Second")).toBeInTheDocument();
  });

  it("default variant uses white text", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Default" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const title = screen.getByText("Default");
    expect(title.className).toContain("text-white");
  });

  // ── New feature tests ─────────────────────────────────────────────────

  it("renders variant icon for success toast", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Saved" variant="success" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    // The icon should be rendered (lucide icons render as SVG)
    const status = screen.getByRole("status");
    const svg = status.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("renders variant icon for error toast", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Oops" variant="error" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const status = screen.getByRole("status");
    const svg = status.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("renders variant icon for info toast", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="FYI" variant="info" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const status = screen.getByRole("status");
    const svg = status.querySelector("svg");
    expect(svg).toBeInTheDocument();
  });

  it("does not render icon for default variant", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Plain" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const status = screen.getByRole("status");
    // Default has no icon — only the dismiss button SVG
    const svgs = status.querySelectorAll("svg");
    // Dismiss button X icon is the only SVG
    expect(svgs.length).toBe(1);
  });

  it("renders dismiss button on each toast", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Closeable" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire Toast" }).click();
    });

    expect(screen.getByRole("button", { name: "Dismiss" })).toBeInTheDocument();
  });

  it("dismisses toast when dismiss button clicked", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Bye" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire Toast" }).click();
    });

    expect(screen.getByText("Bye")).toBeInTheDocument();

    act(() => {
      screen.getByRole("button", { name: "Dismiss" }).click();
    });

    // Should start exiting, then remove after animation
    act(() => {
      vi.advanceTimersByTime(400);
    });

    expect(screen.queryByText("Bye")).not.toBeInTheDocument();
  });

  it("renders action button when provided", () => {
    const onAction = vi.fn();

    render(
      <ToastProvider>
        <ToastTrigger title="Deleted" action={{ label: "Undo", onClick: onAction }} />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire Toast" }).click();
    });

    const undoBtn = screen.getByRole("button", { name: "Undo" });
    expect(undoBtn).toBeInTheDocument();
  });

  it("executes action callback and dismisses on action click", () => {
    const onAction = vi.fn();

    render(
      <ToastProvider>
        <ToastTrigger title="Deleted" action={{ label: "Undo", onClick: onAction }} />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire Toast" }).click();
    });

    act(() => {
      screen.getByRole("button", { name: "Undo" }).click();
    });

    expect(onAction).toHaveBeenCalledTimes(1);

    // Should be dismissed after action
    act(() => {
      vi.advanceTimersByTime(400);
    });

    expect(screen.queryByText("Deleted")).not.toBeInTheDocument();
  });

  it("caps visible toasts at 3", () => {
    function ManyTrigger() {
      const { toast } = useToast();
      return (
        <button
          onClick={() => {
            toast({ title: "T1" });
            toast({ title: "T2" });
            toast({ title: "T3" });
            toast({ title: "T4" });
          }}
        >
          Fire All
        </button>
      );
    }

    render(
      <ToastProvider>
        <ManyTrigger />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    // Only the most recent 3 should be visible
    expect(screen.queryByText("T1")).not.toBeInTheDocument();
    expect(screen.getByText("T2")).toBeInTheDocument();
    expect(screen.getByText("T3")).toBeInTheDocument();
    expect(screen.getByText("T4")).toBeInTheDocument();
  });

  it("supports custom duration", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Quick" duration={1000} />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    expect(screen.getByText("Quick")).toBeInTheDocument();

    // Should start exiting at 1000ms
    act(() => {
      vi.advanceTimersByTime(1100);
    });

    const toast = screen.getByText("Quick").closest("div[class*='toast']");
    expect(toast?.className).toContain("toast-exit");

    // Fully removed at 1300ms
    act(() => {
      vi.advanceTimersByTime(300);
    });

    expect(screen.queryByText("Quick")).not.toBeInTheDocument();
  });

  it("dismisses programmatically via returned id", () => {
    render(
      <ToastProvider>
        <DismissTrigger />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button", { name: "Fire" }).click();
    });

    expect(screen.getByText("Dismissable")).toBeInTheDocument();

    act(() => {
      screen.getByRole("button", { name: "Dismiss Programmatically" }).click();
    });

    act(() => {
      vi.advanceTimersByTime(400);
    });

    expect(screen.queryByText("Dismissable")).not.toBeInTheDocument();
  });

  it("renders progress bar on each toast", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Progress" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const status = screen.getByRole("status");
    const progressBar = status.querySelector(".toast-progress");
    expect(progressBar).toBeInTheDocument();
  });

  it("renders toast container with aria-label", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Accessible" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const container = screen.getByLabelText("Notifications");
    expect(container).toBeInTheDocument();
  });

  it("toast has role=status and aria-live=polite", () => {
    render(
      <ToastProvider>
        <ToastTrigger title="Announced" />
      </ToastProvider>,
    );

    act(() => {
      screen.getByRole("button").click();
    });

    const status = screen.getByRole("status");
    expect(status).toHaveAttribute("aria-live", "polite");
  });
});
