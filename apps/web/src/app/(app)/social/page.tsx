"use client";

import { useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

interface Post {
  id: string;
  author: string;
  content: string;
  timestamp: string;
  hashtags: string[];
}

const mockPosts: Post[] = [
  {
    id: "1",
    author: "did:key:z6Mk...x3rW",
    content:
      "First post on Nous. The sovereign web begins here.",
    timestamp: "2 min ago",
    hashtags: ["nous", "decentralized"],
  },
];

export default function SocialPage() {
  const [posts, setPosts] = useState<Post[]>(mockPosts);
  const [draft, setDraft] = useState("");

  function handlePost() {
    if (!draft.trim()) return;
    const newPost: Post = {
      id: crypto.randomUUID(),
      author: "did:key:z6Mk...x3rW",
      content: draft,
      timestamp: "now",
      hashtags: [],
    };
    setPosts([newPost, ...posts]);
    setDraft("");
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
              variant="outline"
              size="sm"
              className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
            >
              Post
            </Button>
          </div>
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Feed
        </h2>
        <div className="space-y-px">
          {posts.map((post) => (
            <Card
              key={post.id}
              className="bg-transparent border-0 rounded-none border-b border-white/[0.04] pb-6 mb-6"
            >
              <CardContent className="p-0">
                <div className="flex items-baseline gap-3 mb-3">
                  <span className="text-xs font-mono text-neutral-600 truncate max-w-[200px]">
                    {post.author}
                  </span>
                  <span className="text-[10px] text-neutral-700">
                    {post.timestamp}
                  </span>
                </div>
                <p className="text-sm font-light leading-relaxed">
                  {post.content}
                </p>
                {post.hashtags.length > 0 && (
                  <div className="flex gap-2 mt-3">
                    {post.hashtags.map((tag) => (
                      <span
                        key={tag}
                        className="text-[10px] font-mono text-neutral-600"
                      >
                        #{tag}
                      </span>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      </section>
    </div>
  );
}
