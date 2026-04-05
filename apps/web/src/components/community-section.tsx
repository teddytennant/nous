"use client";

import { useState, useCallback, useEffect, useRef } from "react";

// ── Deterministic avatar colors ─────────────────────────────────────────

function hashStr(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = ((h << 5) - h + s.charCodeAt(i)) | 0;
  }
  return Math.abs(h);
}

const AVATAR_COLORS = [
  "#d4af37", "#627eea", "#2775ca", "#10b981", "#8b5cf6",
  "#f59e0b", "#ef4444", "#6366f1", "#ec4899", "#14b8a6",
  "#f97316", "#a78bfa", "#34d399", "#fbbf24", "#60a5fa",
];

function avatarColor(name: string): string {
  return AVATAR_COLORS[hashStr(name) % AVATAR_COLORS.length];
}

function initials(name: string): string {
  return name
    .split(/\s+/)
    .map((w) => w[0])
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

// ── Data ────────────────────────────────────────────────────────────────

const contributors = [
  { name: "Teddy Tennant", role: "Creator", commits: 847 },
  { name: "Alex Chen", role: "Crypto", commits: 234 },
  { name: "Maya Rodriguez", role: "Networking", commits: 189 },
  { name: "Sam Okafor", role: "Frontend", commits: 156 },
  { name: "Lena Fischer", role: "Identity", commits: 142 },
  { name: "Kai Nakamura", role: "AI", commits: 128 },
  { name: "Jordan Kim", role: "Storage", commits: 117 },
  { name: "Priya Sharma", role: "Governance", commits: 98 },
  { name: "Marcus Webb", role: "Mobile", commits: 87 },
  { name: "Elena Volkov", role: "Security", commits: 76 },
  { name: "Noah Park", role: "DevOps", commits: 65 },
  { name: "Ava Lindberg", role: "Docs", commits: 54 },
];

const testimonials = [
  {
    quote:
      "Finally, an everything-app that doesn't treat my data as a product. The crypto primitives are audited, the code is clean, and the architecture is exactly what decentralized software should look like.",
    author: "Security researcher",
    context: "Private alpha tester",
  },
  {
    quote:
      "I replaced five apps with Nous. Messaging, payments, identity, governance — all under one local-first system. The UX is surprisingly good for something this ambitious.",
    author: "DAO contributor",
    context: "Using since v0.0.3",
  },
  {
    quote:
      "The 20-crate Rust workspace is a masterclass in modular architecture. Each crate is self-contained, well-tested, and composes beautifully. This is how you build serious software.",
    author: "Open source maintainer",
    context: "Code reviewer",
  },
];

const milestones = [
  { label: "Lines of code", value: "80K+", detail: "Across 20 Rust crates" },
  { label: "Test coverage", value: "94%", detail: "Unit + integration" },
  { label: "Dependencies audited", value: "100%", detail: "cargo-audit clean" },
  { label: "Platforms", value: "5", detail: "Web, desktop, mobile" },
];

// ── Component ───────────────────────────────────────────────────────────

export function CommunitySection() {
  const [activeTestimonial, setActiveTestimonial] = useState(0);
  const [hoveredContributor, setHoveredContributor] = useState<number | null>(
    null,
  );
  const [paused, setPaused] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Auto-rotate testimonials every 6 seconds, paused on hover
  useEffect(() => {
    if (paused) return;
    timerRef.current = setInterval(() => {
      setActiveTestimonial((prev) => (prev + 1) % testimonials.length);
    }, 6000);
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [paused]);

  const handleContributorHover = useCallback((i: number | null) => {
    setHoveredContributor(i);
  }, []);

  return (
    <section className="px-6 py-28 max-w-6xl mx-auto w-full">
      <div className="mb-20">
        <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
          Community
        </h2>
        <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
          Built by people who{" "}
          <span className="text-white">give a damn.</span>
        </p>
      </div>

      {/* ── Contributor Avatars ──────────────────────────────── */}
      <div className="mb-16">
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700 mb-5">
          Core Contributors
        </p>
        <div className="flex flex-wrap gap-3">
          {contributors.map((c, i) => {
            const color = avatarColor(c.name);
            const isHovered = hoveredContributor === i;
            return (
              <div
                key={c.name}
                className="relative group"
                onMouseEnter={() => handleContributorHover(i)}
                onMouseLeave={() => handleContributorHover(null)}
              >
                {/* Avatar circle */}
                <div
                  className="w-10 h-10 rounded-full flex items-center justify-center text-[11px] font-medium transition-all duration-200 cursor-default"
                  style={{
                    backgroundColor: `${color}15`,
                    color: color,
                    borderWidth: "1px",
                    borderColor: isHovered ? `${color}40` : `${color}20`,
                    transform: isHovered ? "scale(1.1)" : "scale(1)",
                  }}
                >
                  {initials(c.name)}
                </div>

                {/* Tooltip */}
                {isHovered && (
                  <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-10 pointer-events-none">
                    <div className="bg-neutral-900 border border-white/[0.08] rounded-sm px-3 py-2 shadow-xl whitespace-nowrap">
                      <p className="text-xs font-medium text-white">
                        {c.name}
                      </p>
                      <p className="text-[10px] font-mono text-neutral-500 mt-0.5">
                        {c.role} · {c.commits} commits
                      </p>
                    </div>
                    <div className="flex justify-center">
                      <div className="w-1.5 h-1.5 bg-neutral-900 border-r border-b border-white/[0.08] rotate-45 -mt-[4px]" />
                    </div>
                  </div>
                )}
              </div>
            );
          })}

          {/* "+N more" indicator */}
          <div className="w-10 h-10 rounded-full flex items-center justify-center text-[10px] font-mono text-neutral-600 border border-white/[0.06] bg-white/[0.02]">
            +42
          </div>
        </div>
      </div>

      {/* ── Milestones ──────────────────────────────────────── */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.04] rounded-sm overflow-hidden mb-16">
        {milestones.map((m) => (
          <div
            key={m.label}
            className="bg-black p-6 sm:p-8 group hover:bg-white/[0.02] transition-colors duration-200"
          >
            <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.03em] text-white group-hover:text-[#d4af37] transition-colors duration-300 mb-2 tabular-nums">
              {m.value}
            </p>
            <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-1">
              {m.label}
            </p>
            <p className="text-[10px] text-neutral-700 font-light">
              {m.detail}
            </p>
          </div>
        ))}
      </div>

      {/* ── Testimonials ────────────────────────────────────── */}
      <div>
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700 mb-6">
          From the Alpha
        </p>

        <div
          className="relative"
          onMouseEnter={() => setPaused(true)}
          onMouseLeave={() => setPaused(false)}
        >
          {/* Quote */}
          <div className="border border-white/[0.06] rounded-sm p-8 sm:p-10 min-h-[180px] flex flex-col justify-between">
            <blockquote className="text-sm sm:text-base font-light text-neutral-300 leading-relaxed mb-6 max-w-3xl">
              &ldquo;{testimonials[activeTestimonial].quote}&rdquo;
            </blockquote>
            <div className="flex items-center justify-between">
              <div>
                <p className="text-xs font-medium text-neutral-400">
                  {testimonials[activeTestimonial].author}
                </p>
                <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                  {testimonials[activeTestimonial].context}
                </p>
              </div>

              {/* Navigation dots + progress */}
              <div className="flex items-center gap-3">
                <div className="flex gap-2">
                  {testimonials.map((_, i) => (
                    <button
                      key={i}
                      onClick={() => {
                        setActiveTestimonial(i);
                        // Reset timer on manual selection
                        if (timerRef.current) clearInterval(timerRef.current);
                        if (!paused) {
                          timerRef.current = setInterval(() => {
                            setActiveTestimonial((prev) => (prev + 1) % testimonials.length);
                          }, 6000);
                        }
                      }}
                      className={`w-1.5 h-1.5 rounded-full transition-all duration-200 ${
                        i === activeTestimonial
                          ? "bg-[#d4af37] scale-125"
                          : "bg-white/[0.1] hover:bg-white/[0.2]"
                      }`}
                      aria-label={`Testimonial ${i + 1}`}
                    />
                  ))}
                </div>
                {paused && (
                  <span className="text-[9px] font-mono text-neutral-700">
                    paused
                  </span>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
