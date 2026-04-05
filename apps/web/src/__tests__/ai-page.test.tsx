import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockListAgents = vi.fn();
const mockCreateAgent = vi.fn();
const mockDeleteAgent = vi.fn();
const mockChat = vi.fn();
const mockListConversations = vi.fn();
const mockGetConversation = vi.fn();

vi.mock("@/lib/api", () => ({
  ai: {
    listAgents: () => mockListAgents(),
    createAgent: (data: Record<string, unknown>) => mockCreateAgent(data),
    deleteAgent: (id: string) => mockDeleteAgent(id),
    chat: (data: Record<string, unknown>) => mockChat(data),
    listConversations: (params?: Record<string, unknown>) => mockListConversations(params),
    getConversation: (id: string) => mockGetConversation(id),
  },
}));

vi.mock("@/components/toast", () => {
  const toastFn = vi.fn(() => "toast-id");
  return {
    useToast: () => ({ toast: toastFn, dismiss: vi.fn() }),
    ToastProvider: ({ children }: { children: React.ReactNode }) => children,
    __mockToast: toastFn,
  };
});

vi.mock("@/components/page-header", () => ({
  PageHeader: ({ title, subtitle }: { title: string; subtitle: string }) => (
    <div data-testid="page-header">
      <h1>{title}</h1>
      <p>{subtitle}</p>
    </div>
  ),
}));

vi.mock("@/components/empty-state", () => ({
  EmptyState: ({ title, description, action }: { title: string; description: string; action?: React.ReactNode }) => (
    <div data-testid="empty-state">
      <p>{title}</p>
      <p>{description}</p>
      {action}
    </div>
  ),
  AIIllustration: () => <svg data-testid="ai-illustration" />,
  ChatIllustration: () => <svg data-testid="chat-illustration" />,
  ConversationsIllustration: () => <svg data-testid="conversations-illustration" />,
}));

vi.mock("@/components/avatar", () => ({
  Avatar: ({ did }: { did: string }) => <div data-testid="avatar">{did}</div>,
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock("@/components/markdown-renderer", () => ({
  MarkdownRenderer: ({ content }: { content: string }) => <span>{content}</span>,
}));

vi.mock("@/components/keyboard-shortcuts", () => ({
  usePageShortcuts: vi.fn(),
}));

import AIPage from "@/app/(app)/ai/page";

// ── Fixtures ─────────────────────────────────────────────────────────────

const testAgent = {
  id: "agent-1",
  name: "Research Assistant",
  system_prompt: "You are a helpful research assistant.",
  model: "llama3.1",
  temperature: 0.7,
  capabilities: ["web-search", "code"],
};

const testAgent2 = {
  id: "agent-2",
  name: "Code Bot",
  system_prompt: "",
  model: "mistral",
  temperature: 0.3,
  capabilities: [],
};

const testConversation = {
  id: "conv-1",
  agent_id: "agent-1",
  message_count: 5,
  created_at: "2026-04-05T10:00:00Z",
  updated_at: "2026-04-05T12:30:00Z",
};

// ── Helpers ──────────────────────────────────────────────────────────────

function renderAI() {
  return render(<AIPage />);
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("AI page", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    localStorage.clear();
    localStorage.setItem("nous_did", "did:nous:user1");

    mockListAgents.mockResolvedValue({
      agents: [testAgent, testAgent2],
      count: 2,
    });
    mockListConversations.mockResolvedValue([testConversation]);
    mockGetConversation.mockResolvedValue([]);
    mockCreateAgent.mockResolvedValue({
      id: "agent-new",
      name: "New Agent",
      system_prompt: "custom",
      model: "llama3.1",
      temperature: 0.7,
      capabilities: [],
    });
    mockDeleteAgent.mockResolvedValue({ deleted: true });
    mockChat.mockResolvedValue({
      conversation_id: "conv-new",
      response: "Hello! How can I help?",
      role: "assistant",
      message_count: 1,
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── Page structure ──────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders the page header with title", async () => {
      renderAI();
      expect(screen.getByText("AI")).toBeInTheDocument();
    });

    it("renders the subtitle", () => {
      renderAI();
      expect(
        screen.getByText(/Local-first inference/)
      ).toBeInTheDocument();
    });

    it("renders three navigation tabs", () => {
      renderAI();
      expect(screen.getByText("chat")).toBeInTheDocument();
      expect(screen.getByText("agents")).toBeInTheDocument();
      expect(screen.getByText("conversations")).toBeInTheDocument();
    });

    it("defaults to chat tab", () => {
      renderAI();
      const chatTab = screen.getByText("chat");
      expect(chatTab.className).toContain("d4af37"); // gold = active
    });
  });

  // ── Chat view ───────────────────────────────────────────────────────

  describe("Chat view", () => {
    it("shows agent selector when agents are loaded", async () => {
      renderAI();
      await waitFor(() => {
        expect(screen.getByText("Agent")).toBeInTheDocument();
      });
    });

    it("shows 'New chat' button", () => {
      renderAI();
      expect(screen.getByText("New chat")).toBeInTheDocument();
    });

    it("shows empty state when no messages", async () => {
      renderAI();
      await waitFor(() => {
        expect(screen.getByTestId("empty-state")).toBeInTheDocument();
        expect(screen.getByText(/Chat with Research Assistant/)).toBeInTheDocument();
      });
    });

    it("renders message input textarea", async () => {
      renderAI();
      await waitFor(() => {
        expect(screen.getByPlaceholderText("Type a message...")).toBeInTheDocument();
      });
    });

    it("renders send button", () => {
      renderAI();
      // Send button is an icon button
      const buttons = screen.getAllByRole("button");
      const sendBtn = buttons.find((btn) => btn.querySelector("svg"));
      expect(sendBtn).toBeDefined();
    });

    it("shows keyboard hint", () => {
      renderAI();
      expect(screen.getByText(/send/)).toBeInTheDocument();
    });

    it("shows agent name and model in footer", async () => {
      renderAI();
      await waitFor(() => {
        // Multiple elements may match — select option + footer text
        const matches = screen.getAllByText(/Research Assistant/);
        expect(matches.length).toBeGreaterThanOrEqual(1);
      });
    });

    it("sends a message and shows response", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();

      await waitFor(() => {
        expect(screen.getByPlaceholderText("Type a message...")).toBeEnabled();
      });

      const input = screen.getByPlaceholderText("Type a message...");
      await user.type(input, "What is quantum computing?");

      // Find and click the send button (button with send icon)
      const sendButtons = screen.getAllByRole("button");
      const sendBtn = sendButtons.find(
        (btn) => !btn.textContent && btn.querySelector("svg")
      );
      if (sendBtn) {
        await user.click(sendBtn);
      }

      await waitFor(() => {
        expect(mockChat).toHaveBeenCalledWith(
          expect.objectContaining({
            agent_id: "agent-1",
            message: "What is quantum computing?",
          })
        );
      });

      await waitFor(() => {
        expect(screen.getByText("Hello! How can I help?")).toBeInTheDocument();
      });
    });

    it("shows typing indicator while sending", async () => {
      // Make chat hang to see the indicator
      mockChat.mockReturnValue(new Promise(() => {}));
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();

      await waitFor(() => {
        expect(screen.getByPlaceholderText("Type a message...")).toBeEnabled();
      });

      const input = screen.getByPlaceholderText("Type a message...");
      await user.type(input, "Hello");

      const sendButtons = screen.getAllByRole("button");
      const sendBtn = sendButtons.find(
        (btn) => !btn.textContent && btn.querySelector("svg")
      );
      if (sendBtn) {
        await user.click(sendBtn);
      }

      await waitFor(() => {
        const dots = document.querySelectorAll(".typing-dot");
        expect(dots.length).toBe(3);
      });
    });

    it("shows 'No agents yet' when agent list is empty", async () => {
      mockListAgents.mockResolvedValue({ agents: [], count: 0 });
      renderAI();
      await waitFor(() => {
        expect(screen.getByText("No agents yet")).toBeInTheDocument();
      });
    });

    it("disables input when no agent selected", async () => {
      mockListAgents.mockResolvedValue({ agents: [], count: 0 });
      renderAI();
      await waitFor(() => {
        expect(screen.getByPlaceholderText("Create an agent first")).toBeDisabled();
      });
    });
  });

  // ── Agents view ─────────────────────────────────────────────────────

  describe("Agents view", () => {
    it("switches to agents tab", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(screen.getByText("2 agents")).toBeInTheDocument();
      });
    });

    it("shows agent names in list", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(screen.getByText("Research Assistant")).toBeInTheDocument();
        expect(screen.getByText("Code Bot")).toBeInTheDocument();
      });
    });

    it("shows agent model and temperature", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(screen.getByText(/llama3\.1 \/ temp 0\.7/)).toBeInTheDocument();
      });
    });

    it("shows system prompt for agents that have one", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(
          screen.getByText("You are a helpful research assistant.")
        ).toBeInTheDocument();
      });
    });

    it("shows capabilities as tags", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(screen.getByText("web-search")).toBeInTheDocument();
        expect(screen.getByText("code")).toBeInTheDocument();
      });
    });

    it("shows Chat and Delete buttons for each agent", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        const chatButtons = screen.getAllByText("Chat");
        const deleteButtons = screen.getAllByText("Delete");
        expect(chatButtons.length).toBe(2);
        expect(deleteButtons.length).toBe(2);
      });
    });

    it("shows 'New agent' button", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      expect(screen.getByText("New agent")).toBeInTheDocument();
    });

    it("shows empty state when no agents", async () => {
      mockListAgents.mockResolvedValue({ agents: [], count: 0 });
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await waitFor(() => {
        expect(screen.getByText("No agents yet")).toBeInTheDocument();
        expect(screen.getByText("Create Agent")).toBeInTheDocument();
      });
    });

    it("shows create agent form when New agent clicked", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await user.click(screen.getByText("New agent"));
      expect(screen.getByPlaceholderText("Agent name")).toBeInTheDocument();
      expect(
        screen.getByPlaceholderText("System prompt (optional)")
      ).toBeInTheDocument();
    });

    it("creates a new agent", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));
      await user.click(screen.getByText("New agent"));

      await user.type(screen.getByPlaceholderText("Agent name"), "New Agent");
      await user.type(
        screen.getByPlaceholderText("System prompt (optional)"),
        "custom"
      );
      await user.click(screen.getByText("Create"));

      await waitFor(() => {
        expect(mockCreateAgent).toHaveBeenCalledWith(
          expect.objectContaining({ name: "New Agent", system_prompt: "custom" })
        );
      });
    });

    it("deletes an agent", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("agents"));

      await waitFor(() => {
        expect(screen.getAllByText("Delete").length).toBe(2);
      });

      await user.click(screen.getAllByText("Delete")[0]);

      await waitFor(() => {
        expect(mockDeleteAgent).toHaveBeenCalledWith("agent-1");
      });
    });
  });

  // ── Conversations view ──────────────────────────────────────────────

  describe("Conversations view", () => {
    it("switches to conversations tab", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("conversations"));
      await waitFor(() => {
        expect(screen.getByText("1 conversation")).toBeInTheDocument();
      });
    });

    it("shows conversation agent name", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("conversations"));
      await waitFor(() => {
        expect(screen.getByText("Research Assistant")).toBeInTheDocument();
      });
    });

    it("shows message count", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("conversations"));
      await waitFor(() => {
        expect(screen.getByText("5 messages")).toBeInTheDocument();
      });
    });

    it("shows Refresh button", async () => {
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("conversations"));
      expect(screen.getByText("Refresh")).toBeInTheDocument();
    });

    it("shows empty state when no conversations", async () => {
      mockListConversations.mockResolvedValue([]);
      const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
      renderAI();
      await user.click(screen.getByText("conversations"));
      await waitFor(() => {
        expect(screen.getByText("No conversations yet")).toBeInTheDocument();
      });
    });
  });

  // ── API integration ─────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls listAgents on mount", async () => {
      renderAI();
      await waitFor(() => {
        expect(mockListAgents).toHaveBeenCalled();
      });
    });

    it("calls listConversations on mount", async () => {
      renderAI();
      await waitFor(() => {
        expect(mockListConversations).toHaveBeenCalled();
      });
    });

    it("handles listAgents failure gracefully", async () => {
      mockListAgents.mockRejectedValue(new Error("Network error"));
      renderAI();
      // Page should still render without crashing
      await waitFor(() => {
        expect(screen.getByText("AI")).toBeInTheDocument();
      });
    });

    it("handles listConversations failure gracefully", async () => {
      mockListConversations.mockRejectedValue(new Error("fail"));
      renderAI();
      await waitFor(() => {
        expect(screen.getByText("AI")).toBeInTheDocument();
      });
    });
  });
});
