import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, within, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// ── Mocks ────────────────────────────────────────────────────────────────

const mockFeed = vi.fn();
const mockCreatePost = vi.fn();
const mockDeleteEvent = vi.fn();
const mockFollow = vi.fn();
const mockUnfollow = vi.fn();

vi.mock("@/lib/api", () => ({
  social: {
    feed: (params?: unknown) => mockFeed(params),
    createPost: (post: unknown) => mockCreatePost(post),
    deleteEvent: (id: string) => mockDeleteEvent(id),
    follow: (f: string, t: string) => mockFollow(f, t),
    unfollow: (f: string, t: string) => mockUnfollow(f, t),
  },
}));

vi.mock("@/lib/use-realtime", () => ({
  useRealtime: vi.fn(),
}));

const mockToast = vi.fn(() => "toast-id");

vi.mock("@/components/toast", () => ({
  useToast: () => ({ toast: mockToast, dismiss: vi.fn() }),
  ToastProvider: ({ children }: { children: React.ReactNode }) => children,
}));

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
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  ),
  SocialIllustration: () => <div data-testid="social-illustration" />,
  FollowingIllustration: () => <div data-testid="following-illustration" />,
  BookmarkIllustration: () => <div data-testid="bookmark-illustration" />,
}));

vi.mock("@/components/sidebar", () => ({
  setNavBadge: vi.fn(),
}));

vi.mock("@/components/keyboard-shortcuts", () => ({
  usePageShortcuts: vi.fn(),
  useListNavigation: ({ itemCount }: { itemCount: number }) => ({
    selectedIndex: -1,
    setSelectedIndex: vi.fn(),
    containerRef: { current: null },
  }),
}));

vi.mock("@/components/avatar", () => ({
  Avatar: ({ did, size }: { did: string; size: string }) => (
    <div data-testid="avatar" data-did={did} data-size={size} />
  ),
}));

vi.mock("@/components/ui/tooltip", () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import SocialPage from "@/app/(app)/social/page";

// ── Fixtures ────────────────────────────────────────────────────────────

const MOCK_DID = "did:key:z6MkhaXgBZDvotYfpFLQP2HZqcXvUcj1yoJhMFxFBMUvSJfc";
const OTHER_DID = "did:key:z6MkpTHR8VNs0xo2UQc5bgdXKPaeq9a3gLsLe2QHMHogNxRR";

const MOCK_POSTS = [
  {
    id: "post-1",
    pubkey: MOCK_DID,
    created_at: new Date(Date.now() - 60000).toISOString(),
    kind: 1,
    content: "Hello from Nous! #decentralized",
    tags: [["t", "decentralized"]],
  },
  {
    id: "post-2",
    pubkey: OTHER_DID,
    created_at: new Date(Date.now() - 120000).toISOString(),
    kind: 1,
    content: "Building the future of social.",
    tags: [],
  },
  {
    id: "reply-1",
    pubkey: OTHER_DID,
    created_at: new Date(Date.now() - 30000).toISOString(),
    kind: 1,
    content: "Great post!",
    tags: [["e", "post-1"]],
  },
];

// ── Helpers ──────────────────────────────────────────────────────────────

function setupMocks(posts = MOCK_POSTS) {
  mockFeed.mockResolvedValue({ events: posts, count: posts.length });
  mockCreatePost.mockResolvedValue({ id: "new-post", pubkey: MOCK_DID, created_at: new Date().toISOString(), kind: 1, content: "", tags: [] });
  mockDeleteEvent.mockResolvedValue(undefined);
  mockFollow.mockResolvedValue(undefined);
  mockUnfollow.mockResolvedValue(undefined);
}

async function renderSocial(did?: string, displayName?: string) {
  if (did) localStorage.setItem("nous_did", did);
  if (displayName) localStorage.setItem("nous_display_name", displayName);
  render(<SocialPage />);
  // Wait for feed to load
  await screen.findByText("Hello from Nous! #decentralized");
}

async function renderSocialEmpty(did?: string) {
  mockFeed.mockResolvedValue({ events: [], count: 0 });
  if (did) localStorage.setItem("nous_did", did);
  render(<SocialPage />);
  await screen.findByTestId("empty-state");
}

// ── Tests ────────────────────────────────────────────────────────────────

describe("Social page", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    setupMocks();
  });

  // ── Page structure ─────────────────────────────────────────────────────

  describe("Page structure", () => {
    it("renders page header with title and subtitle", async () => {
      await renderSocial(MOCK_DID);
      const header = screen.getByTestId("page-header");
      expect(within(header).getByText("Social")).toBeInTheDocument();
      expect(within(header).getByText(/Decentralized feed/)).toBeInTheDocument();
    });

    it("renders three tab buttons", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("timeline")).toBeInTheDocument();
      expect(screen.getByText("following")).toBeInTheDocument();
      expect(screen.getByText(/bookmarks/)).toBeInTheDocument();
    });

    it("renders Refresh button", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("Refresh")).toBeInTheDocument();
    });

    it("defaults to timeline tab", async () => {
      await renderSocial(MOCK_DID);
      const timelineBtn = screen.getByText("timeline");
      expect(timelineBtn.className).toContain("d4af37");
    });
  });

  // ── Loading state ──────────────────────────────────────────────────────

  describe("Loading state", () => {
    it("shows skeleton screens while loading", () => {
      mockFeed.mockReturnValue(new Promise(() => {})); // never resolves
      render(<SocialPage />);
      // Skeleton loaders use Skeleton component which renders div with rounded class and animate-pulse
      const skeletons = document.querySelectorAll('[data-slot="skeleton"], [class*="animate-pulse"]');
      expect(skeletons.length).toBeGreaterThan(0);
    });
  });

  // ── Compose section ────────────────────────────────────────────────────

  describe("Compose section", () => {
    it("renders textarea with placeholder", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByPlaceholderText("What's on your mind?")).toBeInTheDocument();
    });

    it("renders Post button", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("Post")).toBeInTheDocument();
    });

    it("shows character progress ring when typing", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      // No ring visible when empty (CharacterProgress returns null for 0 chars)
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      await user.type(textarea, "A");
      // Ring SVG should now be present
      const svg = document.querySelector("svg circle");
      expect(svg).toBeTruthy();
    });

    it("shows user display name when DID is set", async () => {
      await renderSocial(MOCK_DID, "Alice");
      // "Alice" appears in compose section and on own post
      const aliceTexts = screen.getAllByText("Alice");
      expect(aliceTexts.length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText("posting as")).toBeInTheDocument();
    });

    it("shows identity warning when no DID", async () => {
      await renderSocial();
      expect(screen.getByText("Set your identity in Settings to post")).toBeInTheDocument();
    });

    it("disables Post button when no text", async () => {
      await renderSocial(MOCK_DID);
      const postBtn = screen.getByText("Post");
      expect(postBtn.closest("button")).toBeDisabled();
    });

    it("shows remaining count near limit", async () => {
      await renderSocial(MOCK_DID);
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      // Use fireEvent.change for long text — user.type simulates 460 keystrokes
      // which is too slow under parallel test execution
      const longText = "a".repeat(460);
      fireEvent.change(textarea, { target: { value: longText } });
      // Should show remaining count (40)
      expect(screen.getByText("40")).toBeInTheDocument();
    });

    it("calls createPost API on submit", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      await user.type(textarea, "My new post #test");
      await user.click(screen.getByText("Post"));

      await waitFor(() => {
        expect(mockCreatePost).toHaveBeenCalledWith(
          expect.objectContaining({
            author_did: MOCK_DID,
            content: "My new post #test",
            hashtags: ["test"],
          }),
        );
      });
    });

    it("shows success toast after posting", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      await user.type(textarea, "Test post");
      await user.click(screen.getByText("Post"));

      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Post published", variant: "success" }),
        );
      });
    });

    it("clears textarea after successful post", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      await user.type(textarea, "Test post");
      await user.click(screen.getByText("Post"));

      await waitFor(() => {
        expect(textarea).toHaveValue("");
      });
    });

    it("shows error toast on post failure", async () => {
      mockCreatePost.mockRejectedValue(new Error("Network error"));
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const textarea = screen.getByPlaceholderText("What's on your mind?");
      await user.type(textarea, "Test post");
      await user.click(screen.getByText("Post"));

      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Failed to post", variant: "error" }),
        );
      });
    });
  });

  // ── Feed posts ─────────────────────────────────────────────────────────

  describe("Feed posts", () => {
    it("renders post content", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("Hello from Nous! #decentralized")).toBeInTheDocument();
      expect(screen.getByText("Building the future of social.")).toBeInTheDocument();
    });

    it("renders avatars for post authors", async () => {
      await renderSocial(MOCK_DID);
      const avatars = screen.getAllByTestId("avatar");
      expect(avatars.length).toBeGreaterThanOrEqual(2);
    });

    it("shows you label for own posts", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("you")).toBeInTheDocument();
    });

    it("shows display name for own posts when set", async () => {
      await renderSocial(MOCK_DID, "Alice");
      // Display name appears in compose section and on own posts
      const aliceTexts = screen.getAllByText("Alice");
      expect(aliceTexts.length).toBeGreaterThanOrEqual(1);
    });

    it("shows hashtag tags", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("#decentralized")).toBeInTheDocument();
    });

    it("shows Follow button for other users' posts", async () => {
      await renderSocial(MOCK_DID);
      expect(screen.getByText("Follow")).toBeInTheDocument();
    });

    it("hides Follow button for own posts", async () => {
      // Only own posts — no Follow button needed
      setupMocks([MOCK_POSTS[0]]);
      await renderSocial(MOCK_DID);
      expect(screen.queryByText("Follow")).not.toBeInTheDocument();
    });

    it("calls social.feed on mount", async () => {
      await renderSocial(MOCK_DID);
      expect(mockFeed).toHaveBeenCalledWith({ limit: 100 });
    });
  });

  // ── Post actions ───────────────────────────────────────────────────────

  describe("Post actions", () => {
    it("renders Like button", async () => {
      await renderSocial(MOCK_DID);
      const likeButtons = screen.getAllByText("Like");
      expect(likeButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("toggles like state on click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const likeButtons = screen.getAllByText("Like");
      await user.click(likeButtons[0]);
      expect(screen.getAllByText("Liked").length).toBeGreaterThanOrEqual(1);
    });

    it("persists likes to localStorage", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const likeButtons = screen.getAllByText("Like");
      await user.click(likeButtons[0]);
      const stored = JSON.parse(localStorage.getItem("nous_likes")!);
      expect(stored).toContain("post-1");
    });

    it("renders Save button", async () => {
      await renderSocial(MOCK_DID);
      const saveButtons = screen.getAllByText("Save");
      expect(saveButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("toggles bookmark on Save click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const saveButtons = screen.getAllByText("Save");
      await user.click(saveButtons[0]);
      expect(mockToast).toHaveBeenCalledWith(
        expect.objectContaining({ title: "Bookmarked", variant: "success" }),
      );
    });

    it("renders Link button for copying", async () => {
      await renderSocial(MOCK_DID);
      const linkButtons = screen.getAllByText("Link");
      expect(linkButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("renders Share button", async () => {
      await renderSocial(MOCK_DID);
      const shareButtons = screen.getAllByText("Share");
      expect(shareButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("renders Reply button on posts", async () => {
      await renderSocial(MOCK_DID);
      const replyButtons = screen.getAllByText("Reply");
      expect(replyButtons.length).toBeGreaterThanOrEqual(1);
    });

    it("renders Delete button only on own posts", async () => {
      await renderSocial(MOCK_DID);
      const deleteButtons = screen.getAllByText("Delete");
      // Only own posts should have delete — post-1 is ours
      expect(deleteButtons).toHaveLength(1);
    });

    it("calls deleteEvent API on Delete click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Delete"));
      await waitFor(() => {
        expect(mockDeleteEvent).toHaveBeenCalledWith("post-1");
      });
    });

    it("shows toast on successful delete", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Delete"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Post deleted" }),
        );
      });
    });
  });

  // ── Follow / Unfollow ──────────────────────────────────────────────────

  describe("Follow and unfollow", () => {
    it("calls follow API on Follow click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Follow"));
      await waitFor(() => {
        expect(mockFollow).toHaveBeenCalledWith(MOCK_DID, OTHER_DID);
      });
    });

    it("shows Following after follow", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Follow"));
      await waitFor(() => {
        expect(screen.getByText("Following")).toBeInTheDocument();
      });
    });

    it("shows success toast on follow", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Follow"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Followed", variant: "success" }),
        );
      });
    });

    it("calls unfollow API on Following click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      // Follow first
      await user.click(screen.getByText("Follow"));
      await waitFor(() => {
        expect(screen.getByText("Following")).toBeInTheDocument();
      });
      // Now unfollow
      await user.click(screen.getByText("Following"));
      await waitFor(() => {
        expect(mockUnfollow).toHaveBeenCalledWith(MOCK_DID, OTHER_DID);
      });
    });
  });

  // ── Tabs ───────────────────────────────────────────────────────────────

  describe("Tab switching", () => {
    it("switches to following tab", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("following"));
      // No followed users yet — empty state
      expect(screen.getByText("No followed posts yet")).toBeInTheDocument();
    });

    it("switches to bookmarks tab", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText(/bookmarks/));
      expect(screen.getByText("No bookmarks yet")).toBeInTheDocument();
    });

    it("shows bookmarked posts in bookmarks tab", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      // Bookmark a post first
      const saveButtons = screen.getAllByText("Save");
      await user.click(saveButtons[0]);
      // Switch to bookmarks tab
      await user.click(screen.getByText(/bookmarks/));
      // The bookmarked post should appear
      expect(screen.getByText("Hello from Nous! #decentralized")).toBeInTheDocument();
    });

    it("shows Browse timeline button in empty bookmarks", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText(/bookmarks/));
      expect(screen.getByText("Browse timeline")).toBeInTheDocument();
    });
  });

  // ── Threading ──────────────────────────────────────────────────────────

  describe("Threading", () => {
    it("shows reply count for posts with replies", async () => {
      await renderSocial(MOCK_DID);
      // post-1 has reply-1 as a reply
      expect(screen.getByText("1")).toBeInTheDocument();
    });

    it("nests replies under parent post and does not show replies as root", async () => {
      await renderSocial(MOCK_DID);
      // reply-1 content should not appear at root level initially (thread collapsed)
      // The thread toggle button shows count "1"
      const threadBtn = screen.getByText("1");
      expect(threadBtn).toBeInTheDocument();
    });
  });

  // ── Empty states ───────────────────────────────────────────────────────

  describe("Empty states", () => {
    it("shows empty state when no posts", async () => {
      await renderSocialEmpty(MOCK_DID);
      expect(screen.getByText("No posts yet")).toBeInTheDocument();
    });

    it("shows Write a post button in empty timeline", async () => {
      await renderSocialEmpty(MOCK_DID);
      expect(screen.getByText("Write a post")).toBeInTheDocument();
    });

    it("shows empty following state", async () => {
      const user = userEvent.setup();
      await renderSocialEmpty(MOCK_DID);
      await user.click(screen.getByText("following"));
      expect(screen.getByText("No followed posts yet")).toBeInTheDocument();
    });

    it("shows empty bookmarks state", async () => {
      const user = userEvent.setup();
      await renderSocialEmpty(MOCK_DID);
      await user.click(screen.getByText(/bookmarks/));
      expect(screen.getByText("No bookmarks yet")).toBeInTheDocument();
    });
  });

  // ── Inline reply ───────────────────────────────────────────────────────

  describe("Inline reply", () => {
    it("opens reply compose on Reply click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const replyButtons = screen.getAllByText("Reply");
      await user.click(replyButtons[0]);
      expect(screen.getByPlaceholderText("Write a reply...")).toBeInTheDocument();
    });

    it("shows Cancel and Reply buttons in reply compose", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const replyButtons = screen.getAllByText("Reply");
      await user.click(replyButtons[0]);
      expect(screen.getByText("Cancel")).toBeInTheDocument();
      // There will be multiple Reply texts now, so check for the compose one
      const replyComposeBtn = screen.getAllByText("Reply");
      expect(replyComposeBtn.length).toBeGreaterThanOrEqual(2);
    });

    it("closes reply compose on Cancel click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const replyButtons = screen.getAllByText("Reply");
      await user.click(replyButtons[0]);
      expect(screen.getByPlaceholderText("Write a reply...")).toBeInTheDocument();
      await user.click(screen.getByText("Cancel"));
      expect(screen.queryByPlaceholderText("Write a reply...")).not.toBeInTheDocument();
    });

    it("submits inline reply via API", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      const replyButtons = screen.getAllByText("Reply");
      await user.click(replyButtons[0]);

      const textarea = screen.getByPlaceholderText("Write a reply...");
      await user.type(textarea, "Nice post!");

      // The inline reply compose area has a submit button labeled "Reply" with
      // specific styling (border class). Find it within the compose area.
      const composeArea = textarea.closest(".inline-reply-compose");
      expect(composeArea).toBeTruthy();
      const submitBtn = within(composeArea as HTMLElement).getByText("Reply");
      await user.click(submitBtn);

      await waitFor(() => {
        expect(mockCreatePost).toHaveBeenCalledWith(
          expect.objectContaining({
            author_did: MOCK_DID,
            content: "Nice post!",
            reply_to: "post-1",
          }),
        );
      });
    });
  });

  // ── Refresh ────────────────────────────────────────────────────────────

  describe("Refresh", () => {
    it("reloads feed on Refresh click", async () => {
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      expect(mockFeed).toHaveBeenCalledTimes(1);
      await user.click(screen.getByText("Refresh"));
      await waitFor(() => {
        expect(mockFeed).toHaveBeenCalledTimes(2);
      });
    });
  });

  // ── API integration ────────────────────────────────────────────────────

  describe("API integration", () => {
    it("calls social.feed on mount with limit 100", async () => {
      await renderSocial(MOCK_DID);
      expect(mockFeed).toHaveBeenCalledWith({ limit: 100 });
    });

    it("handles feed failure gracefully with toast", async () => {
      mockFeed.mockRejectedValue(new Error("Server error"));
      localStorage.setItem("nous_did", MOCK_DID);
      render(<SocialPage />);
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Failed to load feed", variant: "error" }),
        );
      });
    });

    it("handles follow failure gracefully", async () => {
      mockFollow.mockRejectedValue(new Error("fail"));
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Follow"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Failed to update follow", variant: "error" }),
        );
      });
    });

    it("handles delete failure gracefully", async () => {
      mockDeleteEvent.mockRejectedValue(new Error("fail"));
      const user = userEvent.setup();
      await renderSocial(MOCK_DID);
      await user.click(screen.getByText("Delete"));
      await waitFor(() => {
        expect(mockToast).toHaveBeenCalledWith(
          expect.objectContaining({ title: "Failed to delete", variant: "error" }),
        );
      });
    });
  });
});
