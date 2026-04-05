"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { social, type FeedEvent } from "@/lib/api";
import { useRealtime } from "@/lib/use-realtime";
import { useToast } from "@/components/toast";
import { setNavBadge } from "@/components/sidebar";
import { EmptyState, SocialIllustration, FollowingIllustration, BookmarkIllustration } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";
import { usePageShortcuts, useListNavigation } from "@/components/keyboard-shortcuts";
import { cn } from "@/lib/utils";
import { Avatar } from "@/components/avatar";
import { Tooltip } from "@/components/ui/tooltip";
import { Link, Bookmark, Share2, Check, MessageCircle, ChevronDown, Heart, RefreshCw } from "lucide-react";

/* ── Character Progress Ring ──────────────────────────────────────────── */

function CharacterProgress({ current, max }: { current: number; max: number }) {
  const radius = 8;
  const stroke = 1.5;
  const circumference = 2 * Math.PI * radius;
  const ratio = current / max;
  const offset = circumference * (1 - ratio);
  const remaining = max - current;

  const color =
    remaining <= 0
      ? "#dc2626"
      : remaining <= 20
        ? "#f59e0b"
        : remaining <= 50
          ? "#d4af37"
          : "#525252";

  if (current === 0) return null;

  return (
    <div className="flex items-center gap-2">
      <svg width="20" height="20" viewBox="0 0 20 20" className="shrink-0">
        <circle
          cx="10"
          cy="10"
          r={radius}
          fill="none"
          stroke="rgba(255,255,255,0.06)"
          strokeWidth={stroke}
        />
        <circle
          cx="10"
          cy="10"
          r={radius}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          strokeLinecap="round"
          transform="rotate(-90 10 10)"
          className="transition-all duration-150"
        />
      </svg>
      {remaining <= 50 && (
        <span
          className="text-[10px] font-mono tabular-nums transition-colors duration-150"
          style={{ color }}
        >
          {remaining}
        </span>
      )}
    </div>
  );
}

/* ── Auto-expanding Textarea ─────────────────────────────────────────── */

function AutoTextarea({
  value,
  onChange,
  placeholder,
  className,
  minRows = 3,
  maxRows = 10,
  onKeyDown,
  autoFocus,
  textareaRef,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
  minRows?: number;
  maxRows?: number;
  onKeyDown?: (e: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  autoFocus?: boolean;
  textareaRef?: React.RefObject<HTMLTextAreaElement | null>;
}) {
  const internalRef = useRef<HTMLTextAreaElement>(null);
  const ref = textareaRef || internalRef;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.style.height = "auto";
    const lineHeight = 20;
    const min = lineHeight * minRows;
    const max = lineHeight * maxRows;
    el.style.height = `${Math.min(Math.max(el.scrollHeight, min), max)}px`;
  }, [value, minRows, maxRows, ref]);

  return (
    <textarea
      ref={ref}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={className}
      onKeyDown={onKeyDown}
      autoFocus={autoFocus}
      style={{ overflow: "hidden" }}
    />
  );
}

const MAX_POST_LENGTH = 500;
const BOOKMARKS_KEY = "nous_bookmarks";
const LIKES_KEY = "nous_likes";

function loadBookmarks(): Set<string> {
  if (typeof window === "undefined") return new Set();
  try {
    const raw = localStorage.getItem(BOOKMARKS_KEY);
    return raw ? new Set(JSON.parse(raw) as string[]) : new Set();
  } catch {
    return new Set();
  }
}

function saveBookmarks(ids: Set<string>) {
  localStorage.setItem(BOOKMARKS_KEY, JSON.stringify([...ids]));
}

function loadLikes(): Set<string> {
  if (typeof window === "undefined") return new Set();
  try {
    const raw = localStorage.getItem(LIKES_KEY);
    return raw ? new Set(JSON.parse(raw) as string[]) : new Set();
  } catch {
    return new Set();
  }
}

function saveLikes(ids: Set<string>) {
  localStorage.setItem(LIKES_KEY, JSON.stringify([...ids]));
}

// ── Threading ──────────────────────────────────────────────────────────────

/** Extract the parent event ID from Nostr-style tags. Returns null for root posts. */
function getReplyParent(event: FeedEvent): string | null {
  for (const tag of event.tags) {
    if (tag[0] === "e" && tag[1]) return tag[1];
  }
  return null;
}

/** Build a map of parentId → replies, sorted chronologically. */
function buildReplyMap(events: FeedEvent[]): Map<string, FeedEvent[]> {
  const map = new Map<string, FeedEvent[]>();
  for (const event of events) {
    const parentId = getReplyParent(event);
    if (parentId) {
      const existing = map.get(parentId) || [];
      existing.push(event);
      map.set(parentId, existing);
    }
  }
  // Sort replies oldest-first within each thread
  for (const replies of map.values()) {
    replies.sort((a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime());
  }
  return map;
}

/** Get root posts (posts that aren't replies to another post in the feed). */
function getRootPosts(events: FeedEvent[], eventIds: Set<string>): FeedEvent[] {
  return events.filter((e) => {
    const parent = getReplyParent(e);
    // Show as root if: no parent tag, or parent doesn't exist in this feed
    return !parent || !eventIds.has(parent);
  });
}

type Tab = "timeline" | "following" | "bookmarks";

export default function SocialPage() {
  const [posts, setPosts] = useState<FeedEvent[]>([]);
  const [draft, setDraft] = useState("");
  const [loading, setLoading] = useState(true);
  const [posting, setPosting] = useState(false);
  const [activeTab, setActiveTab] = useState<Tab>("timeline");
  const [following, setFollowing] = useState<Set<string>>(new Set());
  const [bookmarks, setBookmarks] = useState<Set<string>>(() => loadBookmarks());
  const [likes, setLikes] = useState<Set<string>>(() => loadLikes());
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [expandedThreads, setExpandedThreads] = useState<Set<string>>(new Set());
  const [inlineReplyTo, setInlineReplyTo] = useState<string | null>(null);
  const [inlineReplyDraft, setInlineReplyDraft] = useState("");
  const [inlinePosting, setInlinePosting] = useState(false);
  const [likedAnimating, setLikedAnimating] = useState<string | null>(null);
  const [heartBurstId, setHeartBurstId] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [newPostIds, setNewPostIds] = useState<Set<string>>(new Set());
  const lastTapRef = useRef<{ id: string; time: number } | null>(null);
  const inlineReplyRef = useRef<HTMLTextAreaElement>(null);

  const { toast } = useToast();
  const userDid = typeof window !== "undefined" ? localStorage.getItem("nous_did") || "" : "";
  const userDisplayName = typeof window !== "undefined" ? localStorage.getItem("nous_display_name") || "" : "";

  const loadFeed = useCallback(async () => {
    try {
      const data = await social.feed({ limit: 100 });
      setPosts(data.events);
      // Reset new-post badge — user has seen the latest
      newPostCountRef.current = 0;
      setNavBadge("/social", 0);
    } catch (e) {
      toast({ title: "Failed to load feed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await loadFeed();
    setRefreshing(false);
  }, [loadFeed]);

  usePageShortcuts({
    n: () => document.querySelector<HTMLTextAreaElement>("textarea")?.focus(),
    r: () => { handleRefresh(); },
    b: () => setActiveTab("bookmarks"),
  });

  useEffect(() => {
    loadFeed();
  }, [loadFeed]);

  // Track new posts from other users for sidebar badge
  const newPostCountRef = useRef(0);

  // Live post updates via SSE
  useRealtime("new_post", (data) => {
    const postId = data.id;
    setPosts((prev) => [
      {
        id: postId,
        pubkey: data.author,
        created_at: new Date().toISOString(),
        kind: 1,
        content: data.content,
        tags: [],
      },
      ...prev,
    ]);
    // Animate the new post in
    setNewPostIds((prev) => new Set(prev).add(postId));
    setTimeout(() => {
      setNewPostIds((prev) => {
        const next = new Set(prev);
        next.delete(postId);
        return next;
      });
    }, 500);
    // Count posts from other users for the sidebar badge
    if (data.author && data.author !== userDid) {
      newPostCountRef.current += 1;
      setNavBadge("/social", newPostCountRef.current);
    }
  });

  // Clear sidebar badge on unmount
  useEffect(() => {
    return () => { setNavBadge("/social", 0); };
  }, []);

  async function handlePost() {
    if (!draft.trim() || posting || !userDid) return;
    setPosting(true);
    try {
      const hashtags = draft.match(/#(\w+)/g)?.map((t) => t.slice(1)) || [];
      await social.createPost({
        author_did: userDid,
        content: draft,
        hashtags,
      });
      setDraft("");
      await loadFeed();
      toast({ title: "Post published", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to post", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setPosting(false);
    }
  }

  async function handleInlineReply(parentId: string) {
    if (!inlineReplyDraft.trim() || inlinePosting || !userDid) return;
    setInlinePosting(true);
    try {
      const hashtags = inlineReplyDraft.match(/#(\w+)/g)?.map((t) => t.slice(1)) || [];
      await social.createPost({
        author_did: userDid,
        content: inlineReplyDraft,
        reply_to: parentId,
        hashtags,
      });
      setInlineReplyDraft("");
      setInlineReplyTo(null);
      await loadFeed();
      // Auto-expand thread to show the new reply
      setExpandedThreads((prev) => new Set(prev).add(parentId));
      toast({ title: "Reply posted", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to reply", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setInlinePosting(false);
    }
  }

  function openInlineReply(postId: string) {
    setInlineReplyTo(postId);
    setInlineReplyDraft("");
    // Auto-expand thread if it has replies
    if (replyMap.get(postId)?.length) {
      setExpandedThreads((prev) => new Set(prev).add(postId));
    }
    // Focus after render
    requestAnimationFrame(() => {
      inlineReplyRef.current?.focus();
    });
  }

  function toggleLike(eventId: string) {
    setLikes((prev) => {
      const next = new Set(prev);
      if (next.has(eventId)) {
        next.delete(eventId);
      } else {
        next.add(eventId);
        setLikedAnimating(eventId);
        setTimeout(() => setLikedAnimating(null), 300);
      }
      saveLikes(next);
      return next;
    });
  }

  function handleDoubleTap(eventId: string) {
    const now = Date.now();
    const last = lastTapRef.current;
    if (last && last.id === eventId && now - last.time < 300) {
      // Double tap — like it (only add, never remove on double-tap)
      if (!likes.has(eventId)) {
        toggleLike(eventId);
      }
      setHeartBurstId(eventId);
      setTimeout(() => setHeartBurstId(null), 800);
      lastTapRef.current = null;
    } else {
      lastTapRef.current = { id: eventId, time: now };
    }
  }

  async function handleDelete(eventId: string) {
    try {
      await social.deleteEvent(eventId);
      await loadFeed();
      toast({ title: "Post deleted" });
    } catch (e) {
      toast({ title: "Failed to delete", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  async function toggleFollow(targetDid: string) {
    if (!userDid) return;
    try {
      if (following.has(targetDid)) {
        await social.unfollow(userDid, targetDid);
        setFollowing((prev) => {
          const next = new Set(prev);
          next.delete(targetDid);
          return next;
        });
        toast({ title: "Unfollowed", variant: "success" });
      } else {
        await social.follow(userDid, targetDid);
        setFollowing((prev) => new Set(prev).add(targetDid));
        toast({ title: "Followed", variant: "success" });
      }
    } catch (e) {
      toast({ title: "Failed to update follow", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  function toggleBookmark(eventId: string) {
    setBookmarks((prev) => {
      const next = new Set(prev);
      if (next.has(eventId)) {
        next.delete(eventId);
        toast({ title: "Bookmark removed" });
      } else {
        next.add(eventId);
        toast({ title: "Bookmarked", variant: "success" });
      }
      saveBookmarks(next);
      return next;
    });
  }

  async function copyPostLink(eventId: string) {
    const url = `${window.location.origin}/social?post=${eventId}`;
    try {
      await navigator.clipboard.writeText(url);
      setCopiedId(eventId);
      toast({ title: "Link copied", variant: "success" });
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      toast({ title: "Failed to copy link", variant: "error" });
    }
  }

  async function sharePost(post: FeedEvent) {
    const url = `${window.location.origin}/social?post=${post.id}`;
    const text = post.content.length > 140
      ? post.content.slice(0, 137) + "..."
      : post.content;

    if (typeof navigator.share === "function") {
      try {
        await navigator.share({ title: "Nous Post", text, url });
        return;
      } catch {
        // User cancelled or share failed — fall through to copy
      }
    }
    await copyPostLink(post.id);
  }

  function toggleThread(postId: string) {
    setExpandedThreads((prev) => {
      const next = new Set(prev);
      if (next.has(postId)) {
        next.delete(postId);
      } else {
        next.add(postId);
      }
      return next;
    });
  }

  // Build threading data
  const allEventIds = new Set(posts.map((p) => p.id));
  const replyMap = buildReplyMap(posts);

  function formatTime(iso: string): string {
    const date = new Date(iso);
    const now = new Date();
    const diff = Math.floor((now.getTime() - date.getTime()) / 1000);
    if (diff < 10) return "just now";
    if (diff < 60) return `${diff}s`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
    if (diff < 604800) return `${Math.floor(diff / 86400)}d`;
    return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  function truncateDid(did: string): string {
    if (did.length > 30) return `${did.slice(0, 16)}...${did.slice(-6)}`;
    return did;
  }

  const filteredPosts =
    activeTab === "following"
      ? posts.filter((p) => following.has(p.pubkey))
      : activeTab === "bookmarks"
        ? posts.filter((p) => bookmarks.has(p.id))
        : posts;

  // For timeline/following: show only root posts (replies appear nested)
  // For bookmarks: show all bookmarked posts flat (user bookmarked them specifically)
  const displayPosts =
    activeTab === "bookmarks"
      ? filteredPosts
      : getRootPosts(filteredPosts, allEventIds);

  const { selectedIndex, setSelectedIndex, containerRef } = useListNavigation({
    itemCount: displayPosts.length,
    onActivate: (index) => {
      const post = displayPosts[index];
      if (post) {
        openInlineReply(post.id);
      }
    },
  });

  return (
    <div className="p-4 sm:p-8 max-w-3xl">
      <PageHeader title="Social" subtitle="Decentralized feed. Your posts, your protocol." />

      {/* Compose */}
      <section className="mb-12">
        <div className="border border-white/[0.06] p-5">
          {userDid && (
            <div className="flex items-center gap-2.5 mb-3 pb-3 border-b border-white/[0.04]">
              <Avatar did={userDid} size="xs" />
              <span className="text-xs font-light text-neutral-500">
                {userDisplayName || "Anonymous"}
              </span>
              <span className="text-[10px] font-mono text-neutral-700">
                posting as
              </span>
            </div>
          )}
          <AutoTextarea
            value={draft}
            onChange={(v) => setDraft(v.slice(0, MAX_POST_LENGTH))}
            placeholder="What's on your mind?"
            className="w-full bg-transparent text-sm font-light resize-none outline-none placeholder:text-neutral-700 min-h-[80px]"
            minRows={3}
            maxRows={8}
            onKeyDown={(e) => {
              if (e.key === "Enter" && e.metaKey) handlePost();
            }}
          />
          <div className="flex items-center justify-between mt-4">
            <CharacterProgress current={draft.length} max={MAX_POST_LENGTH} />
            <div className="flex items-center gap-3">
              <kbd className="hidden sm:inline text-[9px] font-mono text-neutral-700 bg-white/[0.03] px-1.5 py-0.5 rounded border border-white/[0.04]">
                ⌘↵
              </kbd>
              <Button
                onClick={handlePost}
                disabled={posting || !draft.trim() || !userDid}
                variant="outline"
                size="sm"
                className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] disabled:opacity-30"
              >
                {posting ? "Posting..." : "Post"}
              </Button>
            </div>
          </div>
          {!userDid && (
            <p className="text-[10px] text-red-500/60 font-mono mt-2">
              Set your identity in Settings to post
            </p>
          )}
        </div>
      </section>

      {/* Tabs + Refresh */}
      <div className="flex items-center justify-between mb-8">
        <div className="flex gap-6">
          {(["timeline", "following", "bookmarks"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => { setActiveTab(tab); setSelectedIndex(-1); }}
              className={`text-xs font-mono uppercase tracking-[0.2em] pb-2 transition-colors duration-150 ${
                activeTab === tab
                  ? "text-[#d4af37] border-b border-[#d4af37]"
                  : "text-neutral-600 hover:text-neutral-400"
              }`}
            >
              {tab}{tab === "bookmarks" && bookmarks.size > 0 ? ` (${bookmarks.size})` : ""}
            </button>
          ))}
        </div>
        <button
          onClick={handleRefresh}
          disabled={refreshing}
          className="flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors disabled:opacity-50"
        >
          <RefreshCw size={10} className={cn(refreshing && "animate-spin")} />
          {refreshing ? "Loading" : "Refresh"}
        </button>
      </div>

      {/* Feed */}
      <section>
        {loading ? (
          <div className="space-y-px">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="border-b border-white/[0.04] pb-6 mb-6">
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <Skeleton className="h-7 w-7 rounded-full shrink-0" />
                    <Skeleton className="h-3 w-28" />
                    <Skeleton className="h-2.5 w-8" />
                  </div>
                  <Skeleton className="h-2.5 w-14" />
                </div>
                <div className="space-y-2">
                  <Skeleton className="h-3.5 w-full" />
                  <Skeleton className="h-3.5 w-4/5" />
                  <Skeleton className="h-3.5 w-2/3" />
                </div>
                <div className="flex items-center gap-6 mt-4">
                  <Skeleton className="h-2.5 w-10" />
                </div>
              </div>
            ))}
          </div>
        ) : displayPosts.length === 0 ? (
          activeTab === "following" ? (
            <EmptyState
              icon={<FollowingIllustration />}
              title="No followed posts yet"
              description="Follow other users to see their posts appear in this feed. Discover people in the timeline tab."
            />
          ) : activeTab === "bookmarks" ? (
            <EmptyState
              icon={<BookmarkIllustration />}
              title="No bookmarks yet"
              description="Bookmark posts to save them for later. They're stored locally on your device."
              action={
                <button
                  onClick={() => { setActiveTab("timeline"); setSelectedIndex(-1); }}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  Browse timeline
                </button>
              }
            />
          ) : (
            <EmptyState
              icon={<SocialIllustration />}
              title="No posts yet"
              description="Be the first to post something. Your words live on the protocol — decentralized and permanent."
              action={
                <button
                  onClick={() => document.querySelector("textarea")?.focus()}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  Write a post
                </button>
              }
            />
          )
        ) : (
          <div ref={containerRef} className="space-y-px stagger-in">
            {displayPosts.map((post, i) => {
              const isOwn = post.pubkey === userDid;
              const isFollowing = following.has(post.pubkey);
              const isSelected = i === selectedIndex;
              return (
                <Card
                  key={post.id}
                  data-list-item
                  className={cn(
                    "relative bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-6 mb-6 transition-colors duration-150",
                    isSelected && "bg-[#d4af37]/[0.015]",
                    newPostIds.has(post.id) && "new-post-enter"
                  )}
                >
                  {isSelected && (
                    <div className="absolute left-0 top-0 bottom-6 w-0.5 bg-[#d4af37] rounded-full" />
                  )}
                  <CardContent className="p-0">
                    {/* Author row */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-3">
                        <Avatar did={post.pubkey} size="sm" />
                        <div className="flex items-center gap-2 min-w-0">
                          {isOwn && userDisplayName ? (
                            <>
                              <span className="text-xs font-light text-neutral-300 truncate">
                                {userDisplayName}
                              </span>
                              <Tooltip content={post.pubkey}>
                                <span className="text-[10px] font-mono text-neutral-700 truncate max-w-[100px] hidden sm:inline cursor-default hover:text-neutral-500 transition-colors duration-150">
                                  {truncateDid(post.pubkey)}
                                </span>
                              </Tooltip>
                            </>
                          ) : (
                            <Tooltip content={post.pubkey}>
                              <span className="text-xs font-mono text-neutral-500 truncate max-w-[200px] cursor-default hover:text-neutral-300 transition-colors duration-150">
                                {truncateDid(post.pubkey)}
                              </span>
                            </Tooltip>
                          )}
                        </div>
                        <Tooltip content={new Date(post.created_at).toLocaleString()}>
                          <span className="text-[10px] text-neutral-700 shrink-0 cursor-default hover:text-neutral-500 transition-colors duration-150">
                            {formatTime(post.created_at)}
                          </span>
                        </Tooltip>
                      </div>
                      {!isOwn && userDid && (
                        <button
                          onClick={() => toggleFollow(post.pubkey)}
                          className={`text-[10px] font-mono uppercase tracking-wider transition-colors duration-150 ${
                            isFollowing
                              ? "text-[#d4af37]"
                              : "text-neutral-700 hover:text-white"
                          }`}
                        >
                          {isFollowing ? "Following" : "Follow"}
                        </button>
                      )}
                      {isOwn && (
                        <span className="text-[10px] font-mono text-neutral-700">
                          you
                        </span>
                      )}
                    </div>

                    {/* Content — double-tap to like on mobile */}
                    <div
                      className="relative select-none"
                      onTouchEnd={() => handleDoubleTap(post.id)}
                    >
                      <p className="text-sm font-light leading-relaxed whitespace-pre-wrap">
                        {post.content}
                      </p>
                      {heartBurstId === post.id && (
                        <div className="absolute inset-0 flex items-center justify-center">
                          <Heart
                            size={48}
                            fill="#d4af37"
                            className="text-[#d4af37] heart-burst drop-shadow-[0_0_12px_rgba(212,175,55,0.5)]"
                          />
                        </div>
                      )}
                    </div>

                    {/* Tags */}
                    {post.tags.length > 0 && (
                      <div className="flex gap-2 mt-3">
                        {post.tags
                          .filter((t) => t[0] === "t")
                          .map((t) => (
                            <span
                              key={t[1]}
                              className="text-[10px] font-mono text-neutral-600"
                            >
                              #{t[1]}
                            </span>
                          ))}
                      </div>
                    )}

                    {/* Actions */}
                    <div className="flex items-center gap-6 mt-4">
                      <button
                        onClick={() => toggleLike(post.id)}
                        className={cn(
                          "flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider transition-colors",
                          likes.has(post.id)
                            ? "text-[#d4af37]"
                            : "text-neutral-700 hover:text-white"
                        )}
                      >
                        <Heart
                          size={11}
                          fill={likes.has(post.id) ? "currentColor" : "none"}
                          className={likedAnimating === post.id ? "like-pulse" : ""}
                        />
                        {likes.has(post.id) ? "Liked" : "Like"}
                      </button>
                      <button
                        onClick={() => openInlineReply(post.id)}
                        className={cn(
                          "text-[10px] font-mono uppercase tracking-wider transition-colors",
                          inlineReplyTo === post.id
                            ? "text-[#d4af37]"
                            : "text-neutral-700 hover:text-white"
                        )}
                      >
                        Reply
                      </button>
                      {(replyMap.get(post.id)?.length ?? 0) > 0 && (
                        <button
                          onClick={() => toggleThread(post.id)}
                          className={cn(
                            "flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider transition-colors",
                            expandedThreads.has(post.id)
                              ? "text-[#d4af37]"
                              : "text-neutral-700 hover:text-white"
                          )}
                        >
                          <MessageCircle size={11} />
                          {replyMap.get(post.id)!.length}
                          <ChevronDown
                            size={10}
                            className={cn(
                              "transition-transform duration-200",
                              expandedThreads.has(post.id) && "rotate-180"
                            )}
                          />
                        </button>
                      )}
                      <button
                        onClick={() => toggleBookmark(post.id)}
                        className={cn(
                          "flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider transition-colors",
                          bookmarks.has(post.id)
                            ? "text-[#d4af37]"
                            : "text-neutral-700 hover:text-white"
                        )}
                      >
                        <Bookmark
                          size={11}
                          fill={bookmarks.has(post.id) ? "currentColor" : "none"}
                        />
                        {bookmarks.has(post.id) ? "Saved" : "Save"}
                      </button>
                      <button
                        onClick={() => copyPostLink(post.id)}
                        className="flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-white transition-colors"
                      >
                        {copiedId === post.id ? (
                          <Check size={11} className="text-[#d4af37]" />
                        ) : (
                          <Link size={11} />
                        )}
                        {copiedId === post.id ? "Copied" : "Link"}
                      </button>
                      <button
                        onClick={() => sharePost(post)}
                        className="flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-white transition-colors"
                      >
                        <Share2 size={11} />
                        Share
                      </button>
                      {isOwn && (
                        <button
                          onClick={() => handleDelete(post.id)}
                          className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-red-400 transition-colors"
                        >
                          Delete
                        </button>
                      )}
                    </div>

                    {/* Inline Thread Replies */}
                    {(replyMap.get(post.id)?.length ?? 0) > 0 && (
                      <div
                        className="thread-replies mt-4"
                        data-expanded={expandedThreads.has(post.id)}
                      >
                        <div className="thread-replies-inner">
                          <div className="ml-4 pl-4 border-l border-white/[0.06]">
                            {replyMap.get(post.id)!.map((reply) => {
                              const isReplyOwn = reply.pubkey === userDid;
                              return (
                                <div
                                  key={reply.id}
                                  className="thread-reply-item py-3 first:pt-1"
                                >
                                  {/* Reply author row */}
                                  <div className="flex items-center gap-2.5 mb-1.5">
                                    <Avatar did={reply.pubkey} size="xs" />
                                    <Tooltip content={reply.pubkey}>
                                      <span className="text-[11px] font-mono text-neutral-600 truncate max-w-[160px] cursor-default hover:text-neutral-400 transition-colors duration-150">
                                        {truncateDid(reply.pubkey)}
                                      </span>
                                    </Tooltip>
                                    <Tooltip content={new Date(reply.created_at).toLocaleString()}>
                                      <span className="text-[10px] text-neutral-700 cursor-default hover:text-neutral-500 transition-colors duration-150">
                                        {formatTime(reply.created_at)}
                                      </span>
                                    </Tooltip>
                                    {isReplyOwn && (
                                      <span className="text-[10px] font-mono text-neutral-700">you</span>
                                    )}
                                  </div>

                                  {/* Reply content — double-tap to like */}
                                  <div
                                    className="relative select-none"
                                    onTouchEnd={() => handleDoubleTap(reply.id)}
                                  >
                                    <p className="text-[13px] font-light leading-relaxed text-neutral-300 whitespace-pre-wrap">
                                      {reply.content}
                                    </p>
                                    {heartBurstId === reply.id && (
                                      <div className="absolute inset-0 flex items-center justify-center">
                                        <Heart
                                          size={32}
                                          fill="#d4af37"
                                          className="text-[#d4af37] heart-burst drop-shadow-[0_0_8px_rgba(212,175,55,0.5)]"
                                        />
                                      </div>
                                    )}
                                  </div>

                                  {/* Reply actions */}
                                  <div className="flex items-center gap-5 mt-2">
                                    <button
                                      onClick={() => toggleLike(reply.id)}
                                      className={cn(
                                        "flex items-center gap-1 text-[10px] font-mono uppercase tracking-wider transition-colors",
                                        likes.has(reply.id)
                                          ? "text-[#d4af37]"
                                          : "text-neutral-700 hover:text-white"
                                      )}
                                    >
                                      <Heart
                                        size={10}
                                        fill={likes.has(reply.id) ? "currentColor" : "none"}
                                        className={likedAnimating === reply.id ? "like-pulse" : ""}
                                      />
                                    </button>
                                    <button
                                      onClick={() => openInlineReply(post.id)}
                                      className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-white transition-colors"
                                    >
                                      Reply
                                    </button>
                                    <button
                                      onClick={() => toggleBookmark(reply.id)}
                                      className={cn(
                                        "flex items-center gap-1 text-[10px] font-mono uppercase tracking-wider transition-colors",
                                        bookmarks.has(reply.id)
                                          ? "text-[#d4af37]"
                                          : "text-neutral-700 hover:text-white"
                                      )}
                                    >
                                      <Bookmark
                                        size={10}
                                        fill={bookmarks.has(reply.id) ? "currentColor" : "none"}
                                      />
                                      {bookmarks.has(reply.id) ? "Saved" : "Save"}
                                    </button>
                                    {isReplyOwn && (
                                      <button
                                        onClick={() => handleDelete(reply.id)}
                                        className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-red-400 transition-colors"
                                      >
                                        Delete
                                      </button>
                                    )}
                                  </div>

                                  {/* Nested replies (one level deep) */}
                                  {(replyMap.get(reply.id)?.length ?? 0) > 0 && (
                                    <div className="ml-4 pl-3 mt-2 border-l border-white/[0.04]">
                                      {replyMap.get(reply.id)!.map((nested) => (
                                        <div key={nested.id} className="thread-reply-item py-2">
                                          <div className="flex items-center gap-2 mb-1">
                                            <Avatar did={nested.pubkey} size="xs" />
                                            <Tooltip content={nested.pubkey}>
                                              <span className="text-[10px] font-mono text-neutral-600 truncate max-w-[140px] cursor-default hover:text-neutral-400 transition-colors duration-150">
                                                {truncateDid(nested.pubkey)}
                                              </span>
                                            </Tooltip>
                                            <Tooltip content={new Date(nested.created_at).toLocaleString()}>
                                              <span className="text-[10px] text-neutral-700 cursor-default hover:text-neutral-500 transition-colors duration-150">
                                                {formatTime(nested.created_at)}
                                              </span>
                                            </Tooltip>
                                          </div>
                                          <p className="text-xs font-light leading-relaxed text-neutral-400 whitespace-pre-wrap">
                                            {nested.content}
                                          </p>
                                        </div>
                                      ))}
                                    </div>
                                  )}
                                </div>
                              );
                            })}
                          </div>
                        </div>
                      </div>
                    )}

                    {/* Inline Reply Compose */}
                    {inlineReplyTo === post.id && userDid && (
                      <div className="inline-reply-compose mt-4 ml-4 pl-4 border-l border-[#d4af37]/20">
                        {/* Reply context preview */}
                        <div className="flex items-center gap-2 mb-2 pb-2 border-b border-white/[0.04]">
                          <span className="text-[10px] font-mono text-neutral-700">Replying to</span>
                          <span className="text-[10px] font-light text-neutral-500 truncate">
                            {post.content.length > 80 ? post.content.slice(0, 77) + "..." : post.content}
                          </span>
                        </div>
                        <div className="flex gap-3">
                          <Avatar did={userDid} size="xs" />
                          <div className="flex-1 min-w-0">
                            <AutoTextarea
                              textareaRef={inlineReplyRef}
                              value={inlineReplyDraft}
                              onChange={(v) => setInlineReplyDraft(v.slice(0, MAX_POST_LENGTH))}
                              placeholder="Write a reply..."
                              className="w-full bg-transparent text-[13px] font-light resize-none outline-none placeholder:text-neutral-700 min-h-[40px]"
                              minRows={2}
                              maxRows={6}
                              autoFocus
                              onKeyDown={(e) => {
                                if (e.key === "Enter" && e.metaKey) {
                                  handleInlineReply(post.id);
                                }
                                if (e.key === "Escape") {
                                  setInlineReplyTo(null);
                                  setInlineReplyDraft("");
                                }
                              }}
                            />
                            <div className="flex items-center justify-between mt-2">
                              <div className="flex items-center gap-3">
                                <CharacterProgress current={inlineReplyDraft.length} max={MAX_POST_LENGTH} />
                                <kbd className="hidden sm:inline text-[9px] font-mono text-neutral-700 bg-white/[0.03] px-1.5 py-0.5 rounded border border-white/[0.04]">
                                  ⌘↵ send
                                </kbd>
                                <kbd className="hidden sm:inline text-[9px] font-mono text-neutral-700 bg-white/[0.03] px-1.5 py-0.5 rounded border border-white/[0.04]">
                                  esc cancel
                                </kbd>
                              </div>
                              <div className="flex items-center gap-2">
                                <button
                                  onClick={() => {
                                    setInlineReplyTo(null);
                                    setInlineReplyDraft("");
                                  }}
                                  className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-white transition-colors px-2 py-1"
                                >
                                  Cancel
                                </button>
                                <button
                                  onClick={() => handleInlineReply(post.id)}
                                  disabled={inlinePosting || !inlineReplyDraft.trim()}
                                  className="text-[10px] font-mono uppercase tracking-wider px-3 py-1 border border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
                                >
                                  {inlinePosting ? "Sending..." : "Reply"}
                                </button>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    )}
                  </CardContent>
                </Card>
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
