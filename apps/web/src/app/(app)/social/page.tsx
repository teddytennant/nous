"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { social, type FeedEvent } from "@/lib/api";
import { useRealtime } from "@/lib/use-realtime";
import { useToast } from "@/components/toast";
import { EmptyState, SocialIllustration, FollowingIllustration } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";
import { usePageShortcuts, useListNavigation } from "@/components/keyboard-shortcuts";
import { cn } from "@/lib/utils";
import { Avatar } from "@/components/avatar";

const MAX_POST_LENGTH = 500;

export default function SocialPage() {
  const [posts, setPosts] = useState<FeedEvent[]>([]);
  const [draft, setDraft] = useState("");
  const [replyTo, setReplyTo] = useState<{ id: string; author: string } | null>(null);
  const [loading, setLoading] = useState(true);
  const [posting, setPosting] = useState(false);
  const [activeTab, setActiveTab] = useState<"timeline" | "following">("timeline");
  const [following, setFollowing] = useState<Set<string>>(new Set());

  const { toast } = useToast();
  const userDid = typeof window !== "undefined" ? localStorage.getItem("nous_did") || "" : "";

  const loadFeed = useCallback(async () => {
    try {
      const data = await social.feed({ limit: 100 });
      setPosts(data.events);
    } catch (e) {
      toast({ title: "Failed to load feed", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  usePageShortcuts({
    n: () => document.querySelector<HTMLTextAreaElement>("textarea")?.focus(),
    r: () => { loadFeed(); },
  });

  useEffect(() => {
    loadFeed();
  }, [loadFeed]);

  // Live post updates via SSE
  useRealtime("new_post", (data) => {
    setPosts((prev) => [
      {
        id: data.id,
        pubkey: data.author,
        created_at: new Date().toISOString(),
        kind: 1,
        content: data.content,
        tags: [],
      },
      ...prev,
    ]);
  });

  async function handlePost() {
    if (!draft.trim() || posting || !userDid) return;
    setPosting(true);
    try {
      const hashtags = draft.match(/#(\w+)/g)?.map((t) => t.slice(1)) || [];
      await social.createPost({
        author_did: userDid,
        content: draft,
        reply_to: replyTo?.id,
        hashtags,
      });
      setDraft("");
      setReplyTo(null);
      await loadFeed();
      toast({ title: "Post published", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to post", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setPosting(false);
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

  function formatTime(iso: string): string {
    const date = new Date(iso);
    const now = new Date();
    const diff = Math.floor((now.getTime() - date.getTime()) / 1000);
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

  const displayPosts =
    activeTab === "following"
      ? posts.filter((p) => following.has(p.pubkey))
      : posts;

  const { selectedIndex, setSelectedIndex, containerRef } = useListNavigation({
    itemCount: displayPosts.length,
    onActivate: (index) => {
      const post = displayPosts[index];
      if (post) {
        setReplyTo({ id: post.id, author: post.pubkey });
        document.querySelector<HTMLTextAreaElement>("textarea")?.focus();
      }
    },
  });

  return (
    <div className="p-4 sm:p-8 max-w-3xl">
      <PageHeader title="Social" subtitle="Decentralized feed. Your posts, your protocol." />

      {/* Compose */}
      <section className="mb-12">
        <div className="border border-white/[0.06] p-5">
          {replyTo && (
            <div className="flex items-center justify-between mb-3 pb-3 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600">
                Replying to {truncateDid(replyTo.author)}
              </span>
              <button
                onClick={() => setReplyTo(null)}
                className="text-[10px] font-mono text-neutral-700 hover:text-white transition-colors"
              >
                Cancel
              </button>
            </div>
          )}
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value.slice(0, MAX_POST_LENGTH))}
            placeholder={
              replyTo ? "Write your reply..." : "What's on your mind?"
            }
            className="w-full bg-transparent text-sm font-light resize-none outline-none placeholder:text-neutral-700 min-h-[80px]"
            rows={3}
            onKeyDown={(e) => {
              if (e.key === "Enter" && e.metaKey) handlePost();
            }}
          />
          <div className="flex items-center justify-between mt-4">
            <span className="text-[10px] font-mono text-neutral-700">
              {draft.length}/{MAX_POST_LENGTH}
            </span>
            <Button
              onClick={handlePost}
              disabled={posting || !draft.trim() || !userDid}
              variant="outline"
              size="sm"
              className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] disabled:opacity-30"
            >
              {posting ? "Posting..." : replyTo ? "Reply" : "Post"}
            </Button>
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
          {(["timeline", "following"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => { setActiveTab(tab); setSelectedIndex(-1); }}
              className={`text-xs font-mono uppercase tracking-[0.2em] pb-2 transition-colors duration-150 ${
                activeTab === tab
                  ? "text-[#d4af37] border-b border-[#d4af37]"
                  : "text-neutral-600 hover:text-neutral-400"
              }`}
            >
              {tab}
            </button>
          ))}
        </div>
        <button
          onClick={loadFeed}
          className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
        >
          Refresh
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
                    isSelected && "bg-[#d4af37]/[0.015]"
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
                        <span className="text-xs font-mono text-neutral-500 truncate max-w-[200px]">
                          {truncateDid(post.pubkey)}
                        </span>
                        <span className="text-[10px] text-neutral-700">
                          {formatTime(post.created_at)}
                        </span>
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

                    {/* Content */}
                    <p className="text-sm font-light leading-relaxed whitespace-pre-wrap">
                      {post.content}
                    </p>

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
                        onClick={() =>
                          setReplyTo({ id: post.id, author: post.pubkey })
                        }
                        className="text-[10px] font-mono uppercase tracking-wider text-neutral-700 hover:text-white transition-colors"
                      >
                        Reply
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
