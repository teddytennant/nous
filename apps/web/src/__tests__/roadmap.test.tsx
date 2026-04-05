import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { RoadmapSection } from "@/components/roadmap";

describe("RoadmapSection", () => {
  it("renders the section heading", () => {
    render(<RoadmapSection />);
    expect(screen.getByText("Roadmap")).toBeInTheDocument();
    expect(screen.getByText("Transparently.")).toBeInTheDocument();
  });

  it("renders all four phase cards", () => {
    render(<RoadmapSection />);
    expect(screen.getByText("Foundation")).toBeInTheDocument();
    // Platform appears both in card and panel heading
    expect(screen.getAllByText("Platform")).toHaveLength(2);
    expect(screen.getByText("Intelligence")).toBeInTheDocument();
    expect(screen.getByText("Network")).toBeInTheDocument();
  });

  it("renders phase quarter labels", () => {
    render(<RoadmapSection />);
    expect(screen.getByText("Q1 2026")).toBeInTheDocument();
    expect(screen.getByText("Q2 2026")).toBeInTheDocument();
    expect(screen.getByText("Q3 2026")).toBeInTheDocument();
    expect(screen.getByText("Q4 2026")).toBeInTheDocument();
  });

  it("defaults to Phase 2 (Platform) as active", () => {
    render(<RoadmapSection />);
    // Platform should be visible in the items panel
    expect(
      screen.getByText("Decentralized social feeds"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Android native app"),
    ).toBeInTheDocument();
  });

  it("switches to Foundation phase when clicked", () => {
    render(<RoadmapSection />);
    // Click the Foundation phase card
    const foundationButton = screen
      .getByText("Foundation")
      .closest("button")!;
    fireEvent.click(foundationButton);

    expect(
      screen.getByText("20-crate Rust workspace"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("DID:key identity system"),
    ).toBeInTheDocument();
  });

  it("switches to Intelligence phase when clicked", () => {
    render(<RoadmapSection />);
    const intelligenceButton = screen
      .getByText("Intelligence")
      .closest("button")!;
    fireEvent.click(intelligenceButton);

    expect(
      screen.getByText("Local LLM inference"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Agent framework for task automation"),
    ).toBeInTheDocument();
  });

  it("switches to Network phase when clicked", () => {
    render(<RoadmapSection />);
    const networkButton = screen
      .getByText("Network")
      .closest("button")!;
    fireEvent.click(networkButton);

    expect(
      screen.getByText("Federated node discovery"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Plugin / extension system"),
    ).toBeInTheDocument();
  });

  it("shows shipped/progress/planned count summary", () => {
    render(<RoadmapSection />);
    // Platform phase: 5 done, 2 in-progress, 1 planned
    expect(screen.getByText("5 shipped")).toBeInTheDocument();
    expect(screen.getByText("2 in progress")).toBeInTheDocument();
    expect(screen.getByText("1 planned")).toBeInTheDocument();
  });

  it("marks in-progress items with Active label", () => {
    render(<RoadmapSection />);
    // Platform has in-progress items
    const activeLabels = screen.getAllByText("Active");
    expect(activeLabels.length).toBeGreaterThanOrEqual(1);
  });

  it("shows progress bar with shipped count", () => {
    render(<RoadmapSection />);
    // Foundation phase: 7/7 shipped
    expect(screen.getByText("7/7 shipped")).toBeInTheDocument();
  });

  it("shows phase description in the card", () => {
    render(<RoadmapSection />);
    expect(
      screen.getByText(
        "Core architecture, crypto primitives, and local-first storage",
      ),
    ).toBeInTheDocument();
  });

  it("renders roadmap footer with timestamp", () => {
    render(<RoadmapSection />);
    expect(
      screen.getByText(/Roadmap updated April 2026/i),
    ).toBeInTheDocument();
  });

  it("Foundation phase shows all items as done", () => {
    render(<RoadmapSection />);
    const foundationButton = screen
      .getByText("Foundation")
      .closest("button")!;
    fireEvent.click(foundationButton);

    // Should show "7 shipped" in summary and no planned/in-progress counts
    expect(screen.getByText("7 shipped")).toBeInTheDocument();
    // No "planned" or "in progress" should be in the summary for Foundation
    expect(screen.queryByText(/\d+ planned/)).not.toBeInTheDocument();
  });

  it("navigates phases with ArrowDown key", () => {
    render(<RoadmapSection />);
    // Platform (index 1) is default active — focus its button
    const platformButton = screen
      .getAllByText("Platform")[0]
      .closest("button")!;
    platformButton.focus();

    fireEvent.keyDown(platformButton, { key: "ArrowDown" });

    // Intelligence should now be active
    expect(
      screen.getByText("Local LLM inference"),
    ).toBeInTheDocument();
  });

  it("wraps from last phase to first with ArrowDown", () => {
    render(<RoadmapSection />);
    // Navigate to Network (last phase)
    const networkButton = screen
      .getByText("Network")
      .closest("button")!;
    fireEvent.click(networkButton);
    networkButton.focus();

    fireEvent.keyDown(networkButton, { key: "ArrowDown" });

    // Foundation should now be active
    expect(
      screen.getByText("20-crate Rust workspace"),
    ).toBeInTheDocument();
  });

  it("navigates phases with Home and End keys", () => {
    render(<RoadmapSection />);
    const platformButton = screen
      .getAllByText("Platform")[0]
      .closest("button")!;
    platformButton.focus();

    // Home → Foundation
    fireEvent.keyDown(platformButton, { key: "Home" });
    expect(
      screen.getByText("20-crate Rust workspace"),
    ).toBeInTheDocument();

    // End → Network
    const foundationButton = screen
      .getAllByText("Foundation")[0]
      .closest("button")!;
    foundationButton.focus();
    fireEvent.keyDown(foundationButton, { key: "End" });
    expect(
      screen.getByText("Federated node discovery"),
    ).toBeInTheDocument();
  });
});
