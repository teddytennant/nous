import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { ToastProvider, useToast } from "@/components/toast";

// Test component that triggers toasts
function ToastTrigger({ title, description, variant }: { title: string; description?: string; variant?: "default" | "success" | "error" }) {
  const { toast } = useToast();
  return (
    <button onClick={() => toast({ title, description, variant })}>
      Fire Toast
    </button>
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

    // After 3000ms the toast should be fully removed
    act(() => {
      vi.advanceTimersByTime(3100);
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

    // At 2500ms, toast should be in exiting state (toast-exit class)
    act(() => {
      vi.advanceTimersByTime(2600);
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
});
