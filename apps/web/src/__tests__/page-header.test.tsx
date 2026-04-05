import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { PageHeader } from "@/components/page-header";

// Override the default mock per test
const mockUsePathname = vi.fn(() => "/dashboard");
vi.mock("next/navigation", () => ({
  usePathname: () => mockUsePathname(),
}));

describe("PageHeader", () => {
  beforeEach(() => {
    mockUsePathname.mockReturnValue("/dashboard");
  });

  it("renders title and subtitle", () => {
    render(
      <PageHeader title="Dashboard" subtitle="Your sovereign infrastructure." />,
    );

    // Title appears in both breadcrumb and h1 — use heading role to target the h1
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent(
      "Dashboard",
    );
    expect(
      screen.getByText("Your sovereign infrastructure."),
    ).toBeInTheDocument();
  });

  it("renders breadcrumb for known routes", () => {
    mockUsePathname.mockReturnValue("/wallet");
    render(<PageHeader title="Wallet" subtitle="Multi-chain payments." />);

    expect(screen.getByText("Finance")).toBeInTheDocument();
    expect(screen.getByText("/")).toBeInTheDocument();
  });

  it("renders breadcrumb section for social route", () => {
    mockUsePathname.mockReturnValue("/social");
    render(<PageHeader title="Social" subtitle="Decentralized feeds." />);

    expect(screen.getByText("Communication")).toBeInTheDocument();
  });

  it("renders breadcrumb for AI route", () => {
    mockUsePathname.mockReturnValue("/ai");
    render(<PageHeader title="AI" subtitle="Local inference." />);

    expect(screen.getByText("Intelligence")).toBeInTheDocument();
  });

  it("renders breadcrumb for settings route", () => {
    mockUsePathname.mockReturnValue("/settings");
    render(<PageHeader title="Settings" subtitle="Configure." />);

    expect(screen.getByText("Account")).toBeInTheDocument();
  });

  it("does not render breadcrumb for unknown routes", () => {
    mockUsePathname.mockReturnValue("/unknown-page");
    const { container } = render(
      <PageHeader title="Unknown" subtitle="No breadcrumb." />,
    );

    expect(container.querySelector("nav")).toBeNull();
  });

  it("renders online status indicator", () => {
    const { container } = render(
      <PageHeader
        title="Network"
        subtitle="P2P mesh."
        status="online"
      />,
    );

    const indicator = container.querySelector(".bg-emerald-500");
    expect(indicator).toBeInTheDocument();
  });

  it("renders offline status indicator", () => {
    const { container } = render(
      <PageHeader
        title="Network"
        subtitle="P2P mesh."
        status="offline"
      />,
    );

    const indicator = container.querySelector(".bg-red-500");
    expect(indicator).toBeInTheDocument();
  });

  it("does not render status indicator when status is undefined", () => {
    const { container } = render(
      <PageHeader title="Files" subtitle="Storage." />,
    );

    expect(container.querySelector(".bg-emerald-500")).toBeNull();
    expect(container.querySelector(".bg-red-500")).toBeNull();
  });

  it("renders custom actions", () => {
    render(
      <PageHeader
        title="Files"
        subtitle="Storage."
        actions={<button>Upload</button>}
      />,
    );

    expect(
      screen.getByRole("button", { name: "Upload" }),
    ).toBeInTheDocument();
  });

  it("has correct heading level (h1)", () => {
    render(<PageHeader title="Dashboard" subtitle="Overview." />);

    const heading = screen.getByRole("heading", { level: 1 });
    expect(heading).toHaveTextContent("Dashboard");
  });

  it("has accessible breadcrumb navigation", () => {
    mockUsePathname.mockReturnValue("/messages");
    render(<PageHeader title="Messages" subtitle="E2E encrypted." />);

    const nav = screen.getByLabelText("Breadcrumb");
    expect(nav).toBeInTheDocument();
  });
});
