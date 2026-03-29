"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { social, type FeedEvent } from "@/lib/api";
import { useRealtime } from "@/lib/use-realtime";

const MAX_POST_LENGTH = 500;

export default function SocialPage() {
  const [posts, setPosts] = useState<FeedEvent[]>([]);
  const [draft, setDraft] = useState("");
  const [replyTo, setReplyTo] = useState<{ id: string; author: string } | null>(null);
  const [loading, setLoading] = useState(true);
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"timeline" | "following">("timeline");
  const [following, setFollowing] = useState<Set<string>>(new Set());

  const userDid = typeof window !== "undefined" ? localStorage.getItem("nous_did") || "" : "";

  const loadFeed = useCallback(async () => {
    try {
      const data = await social.feed({ limit: 100 });
      setPosts(data.events);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load feed");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadFeed();
  }, [loadFeed]);

  // Live post updates via SSE
  useRealtime("new_post", useCallback((data) => {
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
  }, []));

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
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to post");
    } finally {
      setPosting(false);
    }
  }

  async function handleDelete(eventId: string) {
    try {
      await social.deleteEvent(eventId);
      await loadFeed();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to delete");
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
      } else {
        await social.follow(userDid, targetDid);
        setFollowing((prev) => new Set(prev).add(targetDid));
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to update follow");
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

  return (
    <div className="p-8 max-w-3xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Social
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          Decentralized feed. Your posts, your protocol.
        </p>
      </header>

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

      {error && (
        <div className="text-xs text-red-500/70 font-mono mb-6 px-1 flex items-center justify-between">
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            className="text-neutral-600 hover:text-white ml-4"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Tabs + Refresh */}
      <div className="flex items-center justify-between mb-8">
        <div className="flex gap-6">
          {(["timeline", "following"] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
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
          <p className="text-xs text-neutral-700 font-mono">Loading...</p>
        ) : displayPosts.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-neutral-600 font-light">
              {activeTab === "following"
                ? "Follow someone to see their posts here"
                : "No posts yet. Be the first."}
            </p>
          </div>
        ) : (
          <div className="space-y-px">
            {displayPosts.map((post) => {
              const isOwn = post.pubkey === userDid;
              const isFollowing = following.has(post.pubkey);
              return (
                <Card
                  key={post.id}
                  className="bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-6 mb-6"
                >
                  <CardContent className="p-0">
                    {/* Author row */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-baseline gap-3">
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
