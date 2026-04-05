import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FaqSection } from "@/components/faq";

describe("FaqSection", () => {
  it("renders the section heading", () => {
    render(<FaqSection />);
    expect(screen.getByText("FAQ")).toBeInTheDocument();
    expect(screen.getByText("Answered.")).toBeInTheDocument();
  });

  it("renders category tabs", () => {
    render(<FaqSection />);
    expect(screen.getByRole("tab", { name: /general/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /privacy/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /technical/i })).toBeInTheDocument();
  });

  it("shows General category questions by default", () => {
    render(<FaqSection />);
    expect(screen.getByText("What is Nous?")).toBeInTheDocument();
    expect(screen.getByText("Is Nous free?")).toBeInTheDocument();
  });

  it("marks the first category tab as selected", () => {
    render(<FaqSection />);
    const generalTab = screen.getByRole("tab", { name: /general/i });
    expect(generalTab).toHaveAttribute("aria-selected", "true");
  });

  it("all answers are hidden by default", () => {
    render(<FaqSection />);
    const buttons = screen.getAllByRole("button", { expanded: false });
    // All FAQ question buttons should have aria-expanded=false
    const faqButtons = buttons.filter(
      (b) => b.getAttribute("aria-expanded") === "false",
    );
    expect(faqButtons.length).toBeGreaterThanOrEqual(3);
  });

  it("expands an answer when question is clicked", () => {
    render(<FaqSection />);
    const question = screen.getByText("What is Nous?");
    const button = question.closest("button")!;
    expect(button).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(button);
    expect(button).toHaveAttribute("aria-expanded", "true");

    // Answer text should be in the DOM
    expect(
      screen.getByText(/sovereign everything-app/i),
    ).toBeInTheDocument();
  });

  it("collapses an answer when clicked again", () => {
    render(<FaqSection />);
    const question = screen.getByText("What is Nous?");
    const button = question.closest("button")!;

    fireEvent.click(button); // open
    expect(button).toHaveAttribute("aria-expanded", "true");

    fireEvent.click(button); // close
    expect(button).toHaveAttribute("aria-expanded", "false");
  });

  it("only one answer is open at a time", () => {
    render(<FaqSection />);
    const q1 = screen.getByText("What is Nous?").closest("button")!;
    const q2 = screen.getByText("Is Nous free?").closest("button")!;

    fireEvent.click(q1);
    expect(q1).toHaveAttribute("aria-expanded", "true");
    expect(q2).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(q2);
    expect(q1).toHaveAttribute("aria-expanded", "false");
    expect(q2).toHaveAttribute("aria-expanded", "true");
  });

  it("switches category when tab is clicked", () => {
    render(<FaqSection />);
    const privacyTab = screen.getByRole("tab", { name: /privacy/i });

    fireEvent.click(privacyTab);

    expect(privacyTab).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByText("Who can read my messages?"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Where is my data stored?"),
    ).toBeInTheDocument();
  });

  it("closes open answer when switching categories", () => {
    render(<FaqSection />);

    // Open a question in General
    const q1 = screen.getByText("What is Nous?").closest("button")!;
    fireEvent.click(q1);
    expect(q1).toHaveAttribute("aria-expanded", "true");

    // Switch to Privacy
    fireEvent.click(screen.getByRole("tab", { name: /privacy/i }));

    // All answers in new category should be closed
    const privacyButtons = screen
      .getAllByRole("button")
      .filter((b) => b.getAttribute("aria-expanded") !== null);
    privacyButtons.forEach((b) => {
      expect(b).toHaveAttribute("aria-expanded", "false");
    });
  });

  it("shows Technical category questions", () => {
    render(<FaqSection />);
    fireEvent.click(screen.getByRole("tab", { name: /technical/i }));

    expect(
      screen.getByText("What platforms does Nous run on?"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("What is the tech stack?"),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Can I self-host Nous?"),
    ).toBeInTheDocument();
  });

  it("renders tabpanel with correct aria-label", () => {
    render(<FaqSection />);
    expect(
      screen.getByRole("tabpanel", { name: /general questions/i }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: /privacy/i }));
    expect(
      screen.getByRole("tabpanel", {
        name: /privacy & security questions/i,
      }),
    ).toBeInTheDocument();
  });

  it("renders tablist with correct aria-label", () => {
    render(<FaqSection />);
    expect(
      screen.getByRole("tablist", { name: /faq categories/i }),
    ).toBeInTheDocument();
  });

  it("answer content contains expected detail", () => {
    render(<FaqSection />);
    fireEvent.click(screen.getByText("Is Nous free?").closest("button")!);
    expect(screen.getByText(/MIT license/i)).toBeInTheDocument();
  });

  it("Privacy category answer contains encryption details", () => {
    render(<FaqSection />);
    fireEvent.click(screen.getByRole("tab", { name: /privacy/i }));
    fireEvent.click(
      screen.getByText("Who can read my messages?").closest("button")!,
    );
    expect(screen.getByText(/AES-256-GCM/i)).toBeInTheDocument();
  });
});
