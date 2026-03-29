"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { social, type FeedEvent } from "@/lib/api";

export default function SocialPage() {
  const [posts, setPosts] = useState<FeedEvent[]>([]);
  const [draft, setDraft] = useState("");
  const [loading, setLoading] = useState(true);
  const [posting, setPosting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadFeed = useCallback(async () => {
    try {
      const data = await social.feed({ limit: 50 });
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

  async function handlePost() {
    if (!draft.trim() || posting) return;
    setPosting(true);

    try {
      const hashtags = draft.match(/#(\w+)/g)?.map((t) => t.slice(1)) || [];
      await social.createPost({
        author_did: "did:key:z6Mk...local",
        content: draft,
        hashtags,
      });
      setDraft("");
      await loadFeed();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to post");
    } finally {
      setPosting(false);
    }
  }

  function formatTime(iso: string): string {
    const date = new Date(iso);
    const now = new Date();
    const diff = Math.floor((now.getTime() - date.getTime()) / 1000);
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return date.toLocaleDateString();
  }

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

      <section className="mb-12">
        <div className="border border-white/[0.06] p-5">
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="What's on your mind?"
            className="w-full bg-transparent text-sm font-light resize-none outline-none placeholder:text-neutral-700 min-h-[80px]"
            rows={3}
          />
          <div className="flex justify-end mt-4">
            <Button
              onClick={handlePost}
              disabled={posting || !draft.trim()}
              variant="outline"
              size="sm"
              className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37] disabled:opacity-30"
            >
              {posting ? "Posting..." : "Post"}
            </Button>
          </div>
        </div>
      </section>

      {error && (
        <div className="text-xs text-red-500/70 font-mono mb-6 px-1">
          {error}
        </div>
      )}

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Feed
        </h2>
        {loading ? (
          <p className="text-xs text-neutral-700 font-mono">Loading...</p>
        ) : posts.length === 0 ? (
          <p className="text-sm text-neutral-600 font-light">
            No posts yet. Be the first.
          </p>
        ) : (
          <div className="space-y-px">
            {posts.map((post) => (
              <Card
                key={post.id}
                className="bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-6 mb-6"
              >
                <CardContent className="p-0">
                  <div className="flex items-baseline gap-3 mb-3">
                    <span className="text-xs font-mono text-neutral-600 truncate max-w-[200px]">
                      {post.pubkey}
                    </span>
                    <span className="text-[10px] text-neutral-700">
                      {formatTime(post.created_at)}
                    </span>
                  </div>
                  <p className="text-sm font-light leading-relaxed">
                    {post.content}
                  </p>
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
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
