import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Onboarding } from "@/components/onboarding";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockIdentityCreate = vi.fn();

vi.mock("@/lib/api", () => ({
  identity: {
    create: (name?: string) => mockIdentityCreate(name),
  },
}));

// ── Helpers ──────────────────────────────────────────────────────────────

function renderOnboarding(onComplete = vi.fn()) {
  return { onComplete, ...render(<Onboarding onComplete={onComplete} />) };
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Onboarding", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockIdentityCreate.mockReset();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("Welcome step", () => {
    it("renders welcome screen with title and description", () => {
      renderOnboarding();
      expect(screen.getByText("Welcome to Nous")).toBeInTheDocument();
      expect(
        screen.getByText("Your sovereign digital infrastructure."),
      ).toBeInTheDocument();
    });

    it("shows Get Started button", () => {
      renderOnboarding();
      expect(
        screen.getByRole("button", { name: /get started/i }),
      ).toBeInTheDocument();
    });

    it("shows time estimate", () => {
      renderOnboarding();
      expect(
        screen.getByText("TAKES LESS THAN 30 SECONDS"),
      ).toBeInTheDocument();
    });

    it("renders the Nous logo SVG", () => {
      const { container } = renderOnboarding();
      const svg = container.querySelector("svg.onboarding-logo");
      expect(svg).toBeInTheDocument();
    });

    it("advances to identity step on Get Started click", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderOnboarding();

      await user.click(screen.getByRole("button", { name: /get started/i }));

      expect(screen.getByText("Create Your Identity")).toBeInTheDocument();
    });
  });

  describe("Identity step", () => {
    async function goToIdentityStep() {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const result = renderOnboarding();
      await user.click(screen.getByRole("button", { name: /get started/i }));
      return { user, ...result };
    }

    it("renders identity creation form", async () => {
      await goToIdentityStep();
      expect(screen.getByText("Create Your Identity")).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText("Display name (optional)"),
      ).toBeInTheDocument();
    });

    it("shows Generate Identity button", async () => {
      await goToIdentityStep();
      expect(
        screen.getByRole("button", { name: /generate identity/i }),
      ).toBeInTheDocument();
    });

    it("shows crypto primitives footer", async () => {
      await goToIdentityStep();
      expect(screen.getByText("ED25519")).toBeInTheDocument();
      expect(screen.getByText("X25519")).toBeInTheDocument();
      expect(screen.getByText("LOCAL ONLY")).toBeInTheDocument();
    });

    it("auto-focuses display name input", async () => {
      await goToIdentityStep();
      const input = screen.getByPlaceholderText("Display name (optional)");
      expect(input).toHaveFocus();
    });

    it("creates identity with display name on button click", async () => {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkTest123" });
      const { user } = await goToIdentityStep();

      await user.type(
        screen.getByPlaceholderText("Display name (optional)"),
        "Alice",
      );
      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(mockIdentityCreate).toHaveBeenCalledWith("Alice");
      });
    });

    it("creates identity without display name when empty", async () => {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkTest456" });
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(mockIdentityCreate).toHaveBeenCalledWith(undefined);
      });
    });

    it("saves DID to localStorage on success", async () => {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkSaved" });
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(localStorage.getItem("nous_did")).toBe("did:key:z6MkSaved");
      });
    });

    it("shows loading state during creation", async () => {
      let resolve: (v: { did: string }) => void;
      mockIdentityCreate.mockReturnValue(
        new Promise((r) => {
          resolve = r;
        }),
      );
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      expect(screen.getByText("Generating keys...")).toBeInTheDocument();

      resolve!({ did: "did:key:z6MkDone" });
      await waitFor(() => {
        expect(screen.queryByText("Generating keys...")).not.toBeInTheDocument();
      });
    });

    it("shows error on API failure", async () => {
      mockIdentityCreate.mockRejectedValue(new Error("Network timeout"));
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(screen.getByText("Network timeout")).toBeInTheDocument();
      });
    });

    it("shows generic error for non-Error throws", async () => {
      mockIdentityCreate.mockRejectedValue("unknown");
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(
          screen.getByText("Failed to create identity"),
        ).toBeInTheDocument();
      });
    });

    it("advances to tour step after successful creation", async () => {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkTour" });
      const { user } = await goToIdentityStep();

      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );

      await waitFor(() => {
        expect(screen.getByText("Everything You Need")).toBeInTheDocument();
      });
    });

    it("supports Enter key to submit", async () => {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkEnter" });
      const { user } = await goToIdentityStep();

      const input = screen.getByPlaceholderText("Display name (optional)");
      await user.type(input, "Bob{Enter}");

      await waitFor(() => {
        expect(mockIdentityCreate).toHaveBeenCalledWith("Bob");
      });
    });
  });

  describe("Tour step", () => {
    async function goToTourStep() {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkTour" });
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const result = renderOnboarding();

      await user.click(screen.getByRole("button", { name: /get started/i }));
      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );
      await waitFor(() => {
        expect(screen.getByText("Everything You Need")).toBeInTheDocument();
      });

      return { user, ...result };
    }

    it("renders all 8 feature cards", async () => {
      await goToTourStep();

      const expectedFeatures = [
        "Social",
        "Messages",
        "Wallet",
        "AI",
        "Governance",
        "Files",
        "Network",
        "Identity",
      ];

      for (const name of expectedFeatures) {
        expect(screen.getByText(name)).toBeInTheDocument();
      }
    });

    it("shows feature descriptions", async () => {
      await goToTourStep();
      expect(
        screen.getByText("Decentralized feed on the Nostr protocol"),
      ).toBeInTheDocument();
      expect(
        screen.getByText("End-to-end encrypted, peer-to-peer"),
      ).toBeInTheDocument();
    });

    it("shows subtitle", async () => {
      await goToTourStep();
      expect(
        screen.getByText(
          "Eight modules, one protocol. All running locally on your node.",
        ),
      ).toBeInTheDocument();
    });

    it("shows Enter Nous button", async () => {
      await goToTourStep();
      expect(
        screen.getByRole("button", { name: /enter nous/i }),
      ).toBeInTheDocument();
    });

    it("advances to ready step on Enter Nous click", async () => {
      const { user } = await goToTourStep();

      await user.click(screen.getByRole("button", { name: /enter nous/i }));

      expect(screen.getByText("You're All Set")).toBeInTheDocument();
    });
  });

  describe("Ready step", () => {
    async function goToReadyStep() {
      mockIdentityCreate.mockResolvedValue({ did: "did:key:z6MkReady" });
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      const onComplete = vi.fn();
      render(<Onboarding onComplete={onComplete} />);

      await user.click(screen.getByRole("button", { name: /get started/i }));
      await user.click(
        screen.getByRole("button", { name: /generate identity/i }),
      );
      await waitFor(() => {
        expect(screen.getByText("Everything You Need")).toBeInTheDocument();
      });
      await user.click(screen.getByRole("button", { name: /enter nous/i }));

      return { user, onComplete };
    }

    it("shows success message", async () => {
      await goToReadyStep();
      expect(screen.getByText("You're All Set")).toBeInTheDocument();
    });

    it("shows welcome message", async () => {
      await goToReadyStep();
      expect(
        screen.getByText(
          "Your identity is live. Your node is yours. Welcome to the sovereign web.",
        ),
      ).toBeInTheDocument();
    });

    it("displays the DID", async () => {
      await goToReadyStep();
      expect(screen.getByText("did:key:z6MkReady")).toBeInTheDocument();
    });

    it("auto-completes after 2.5 seconds", async () => {
      const { onComplete } = await goToReadyStep();

      expect(onComplete).not.toHaveBeenCalled();

      vi.advanceTimersByTime(2500);

      await waitFor(() => {
        expect(onComplete).toHaveBeenCalledTimes(1);
      });
    });
  });

  describe("Step indicator", () => {
    it("renders 4 step indicators", () => {
      const { container } = renderOnboarding();
      // Step indicators are div elements with h-px class inside the indicator container
      const indicators = container.querySelectorAll(
        ".flex.items-center.gap-2 > div",
      );
      expect(indicators.length).toBe(4);
    });

    it("highlights current step in gold", () => {
      const { container } = renderOnboarding();
      const indicators = container.querySelectorAll(
        ".flex.items-center.gap-2 > div",
      );
      // First indicator should be active (gold) on welcome step
      expect(indicators[0].className).toContain("bg-[#d4af37]");
    });

    it("shows unhighlighted steps for future steps", () => {
      const { container } = renderOnboarding();
      const indicators = container.querySelectorAll(
        ".flex.items-center.gap-2 > div",
      );
      // Second indicator should be inactive on welcome step
      expect(indicators[1].className).toContain("bg-white/[0.08]");
    });
  });

  describe("Full-screen overlay", () => {
    it("renders as fixed overlay at z-200", () => {
      const { container } = renderOnboarding();
      const overlay = container.firstElementChild;
      expect(overlay?.className).toContain("fixed");
      expect(overlay?.className).toContain("inset-0");
      expect(overlay?.className).toContain("z-[200]");
    });

    it("has black background", () => {
      const { container } = renderOnboarding();
      const overlay = container.firstElementChild;
      expect(overlay?.className).toContain("bg-black");
    });
  });
});
