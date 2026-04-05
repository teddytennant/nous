import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { EmptyState } from "@/components/empty-state";

describe("EmptyState", () => {
  it("renders title and description", () => {
    render(
      <EmptyState
        icon={<span data-testid="icon">icon</span>}
        title="No files yet"
        description="Upload your first file."
      />,
    );

    expect(screen.getByText("No files yet")).toBeInTheDocument();
    expect(screen.getByText("Upload your first file.")).toBeInTheDocument();
  });

  it("renders the icon", () => {
    render(
      <EmptyState
        icon={<span data-testid="test-icon">IC</span>}
        title="Empty"
        description="Nothing here."
      />,
    );

    expect(screen.getByTestId("test-icon")).toBeInTheDocument();
  });

  it("renders an action when provided", () => {
    render(
      <EmptyState
        icon={<span>icon</span>}
        title="No items"
        description="Create one."
        action={<button>Create</button>}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Create" }),
    ).toBeInTheDocument();
  });

  it("does not render action wrapper when no action", () => {
    const { container } = render(
      <EmptyState
        icon={<span>icon</span>}
        title="No items"
        description="Nothing here."
      />,
    );

    // The outermost div has the icon div, h3, p — no action div
    const outerDiv = container.firstElementChild!;
    // icon wrapper + h3 + p = 3 children, no action wrapper
    expect(outerDiv.children).toHaveLength(3);
  });

  it("renders action wrapper when action is provided", () => {
    const { container } = render(
      <EmptyState
        icon={<span>icon</span>}
        title="No items"
        description="Nothing here."
        action={<button>Go</button>}
      />,
    );

    const outerDiv = container.firstElementChild!;
    // icon wrapper + h3 + p + action wrapper = 4 children
    expect(outerDiv.children).toHaveLength(4);
  });

  it("applies correct CSS classes for centering", () => {
    const { container } = render(
      <EmptyState
        icon={<span>icon</span>}
        title="Test"
        description="Test desc"
      />,
    );

    const wrapper = container.firstElementChild!;
    expect(wrapper.className).toContain("flex");
    expect(wrapper.className).toContain("flex-col");
    expect(wrapper.className).toContain("items-center");
    expect(wrapper.className).toContain("justify-center");
  });
});
