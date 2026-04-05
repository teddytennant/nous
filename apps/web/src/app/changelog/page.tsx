"use client";

import { useState, useMemo } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  ExternalLink,
  Sparkles,
  Bug,
  Wrench,
  Zap,
  Shield,
  Package,
  ChevronDown,
  Tag,
  Calendar,
  Download,
} from "lucide-react";
import { cn } from "@/lib/utils";

// ── Types ─────────────────────────────────────���─────────────────────────

type ChangeCategory = "feature" | "fix" | "improvement" | "security" | "breaking" | "platform";

interface Change {
  description: string;
  category: ChangeCategory;
  /** Optional scope like "web", "api", "cli", "android", "desktop" */
  scope?: string;
}

interface Release {
  version: string;
  date: string;
  title: string;
  summary: string;
  changes: Change[];
  /** True for the most recent release */
  latest?: boolean;
}

// ── Constants ────��──────────────────────────────────────────────────────

const GITHUB_REPO = "teddytennant/nous";

const CATEGORY_CONFIG: Record<ChangeCategory, { label: string; icon: typeof Sparkles; color: string }> = {
  feature: { label: "Feature", icon: Sparkles, color: "text-[#d4af37]" },
  fix: { label: "Fix", icon: Bug, color: "text-red-400" },
  improvement: { label: "Improvement", icon: Wrench, color: "text-blue-400" },
  security: { label: "Security", icon: Shield, color: "text-emerald-400" },
  breaking: { label: "Breaking", icon: Zap, color: "text-orange-400" },
  platform: { label: "Platform", icon: Package, color: "text-purple-400" },
};

const FILTER_OPTIONS: { key: ChangeCategory | "all"; label: string }[] = [
  { key: "all", label: "All" },
  { key: "feature", label: "Features" },
  { key: "fix", label: "Fixes" },
  { key: "improvement", label: "Improvements" },
  { key: "security", label: "Security" },
  { key: "platform", label: "Platform" },
];

// ── Release data ─────────────────────────────��──────────────────────────

const releases: Release[] = [
  {
    version: "0.4.0",
    date: "2026-04-05",
    title: "UI/UX Overhaul",
    summary: "Massive frontend polish pass. New landing page sections, enhanced toast system, keyboard navigation, and comprehensive test coverage across all pages.",
    latest: true,
    changes: [
      { description: "FAQ accordion with category tabs and keyboard navigation", category: "feature", scope: "web" },
      { description: "Interactive roadmap with phase progress tracking", category: "feature", scope: "web" },
      { description: "Auto-rotating testimonials with pause-on-hover", category: "feature", scope: "web" },
      { description: "Enhanced toast system with progress bars, dismiss buttons, and action buttons", category: "improvement", scope: "web" },
      { description: "Toast info variant with blue accent", category: "feature", scope: "web" },
      { description: "Programmatic toast dismissal via returned IDs", category: "improvement", scope: "web" },
      { description: "Toast cap at 3 visible notifications", category: "improvement", scope: "web" },
      { description: "Landing page footer redesigned with 4-column layout", category: "improvement", scope: "web" },
      { description: "Comprehensive dashboard page tests (36 tests)", category: "improvement", scope: "web" },
      { description: "Comprehensive AI page tests (35 tests)", category: "improvement", scope: "web" },
      { description: "Download page tests (40 tests)", category: "improvement", scope: "web" },
      { description: "Respects prefers-reduced-motion for all toast animations", category: "improvement", scope: "web" },
    ],
  },
  {
    version: "0.3.0",
    date: "2026-03-31",
    title: "Full-Stack Polish",
    summary: "Command palette, keyboard shortcuts, notification system, governance analytics, and DataTable component. The app now feels like a real product.",
    changes: [
      { description: "Command palette (Cmd+K) for quick navigation across all pages", category: "feature", scope: "web" },
      { description: "Global keyboard shortcuts (G+D for dashboard, G+S for social, etc.)", category: "feature", scope: "web" },
      { description: "Notification panel with bell icon, unread badges, and mark-all-read", category: "feature", scope: "web" },
      { description: "Governance analytics dashboard with vote distribution charts", category: "feature", scope: "web" },
      { description: "Proposal detail sheet with timeline and vote breakdown", category: "feature", scope: "web" },
      { description: "DataTable component with sorting, filtering, and pagination", category: "feature", scope: "web" },
      { description: "Wallet chart with sparkline transaction history", category: "feature", scope: "web" },
      { description: "Nav badges for unread counts on sidebar items", category: "feature", scope: "web" },
      { description: "Page header component with breadcrumbs", category: "improvement", scope: "web" },
      { description: "Sidebar collapsible sections with keyboard shortcut hints", category: "improvement", scope: "web" },
      { description: "Mobile bottom tab bar navigation", category: "feature", scope: "web" },
      { description: "Product tour for first-time users", category: "feature", scope: "web" },
      { description: "Delegation system for governance voting power", category: "feature", scope: "api" },
      { description: "Reputation and credential display on identity page", category: "feature", scope: "web" },
    ],
  },
  {
    version: "0.2.0",
    date: "2026-03-29",
    title: "Communication & Commerce",
    summary: "End-to-end encrypted messaging, social feed with threading, marketplace with escrow, and wallet with Lightning invoices.",
    changes: [
      { description: "Encrypted messaging with channels, DMs, and group chats", category: "feature", scope: "api" },
      { description: "Social feed with threaded replies, bookmarks, and likes", category: "feature", scope: "web" },
      { description: "Real-time WebSocket updates for messages and social feed", category: "feature", scope: "api" },
      { description: "Marketplace with listings, offers, and escrow contracts", category: "feature", scope: "api" },
      { description: "Wallet with multi-token balances, send, and receive", category: "feature", scope: "web" },
      { description: "Lightning invoice creation and payment", category: "feature", scope: "api" },
      { description: "Escrow contract creation with conditions and deadlines", category: "feature", scope: "api" },
      { description: "File upload with drag-and-drop and encryption", category: "feature", scope: "web" },
      { description: "Peer graph visualization on network page", category: "feature", scope: "web" },
      { description: "Onboarding flow with identity generation", category: "feature", scope: "web" },
      { description: "Empty states with custom SVG illustrations for all pages", category: "improvement", scope: "web" },
      { description: "Skeleton loading states on all pages", category: "improvement", scope: "web" },
      { description: "Desktop release workflow for macOS, Linux, and Windows", category: "platform", scope: "desktop" },
      { description: "Android release workflow with APK signing", category: "platform", scope: "android" },
    ],
  },
  {
    version: "0.1.0",
    date: "2026-03-22",
    title: "Genesis",
    summary: "Initial release. 20-crate Rust workspace with identity, crypto, messaging, social, governance, payments, marketplace, AI, files, and storage. REST + GraphQL + gRPC API server, CLI tool, and Next.js web frontend.",
    changes: [
      { description: "20-crate Rust workspace architecture", category: "feature", scope: "api" },
      { description: "Self-sovereign identity with DID creation and management", category: "feature", scope: "api" },
      { description: "Ed25519 + X25519 cryptographic key management", category: "security", scope: "api" },
      { description: "REST, GraphQL, and gRPC API endpoints", category: "feature", scope: "api" },
      { description: "CLI tool for node management and operations", category: "feature", scope: "cli" },
      { description: "Next.js web frontend with dark theme", category: "feature", scope: "web" },
      { description: "P2P networking with libp2p", category: "feature", scope: "api" },
      { description: "SQLite storage with CRDT-based sync", category: "feature", scope: "api" },
      { description: "AI module with local inference support", category: "feature", scope: "api" },
      { description: "CI/CD pipeline with cross-platform release builds", category: "platform" },
      { description: "Download page with OS detection and platform-specific instructions", category: "feature", scope: "web" },
      { description: "PWA support for mobile install", category: "platform", scope: "web" },
    ],
  },
];

// ── Helpers ──────────────────────────────────────────────────────────────

function formatDate(iso: string): string {
  return new Date(iso + "T00:00:00").toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

function formatDateShort(iso: string): string {
  return new Date(iso + "T00:00:00").toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

function daysAgo(iso: string): string {
  const diff = Date.now() - new Date(iso + "T00:00:00").getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  if (days < 30) return `${Math.floor(days / 7)} weeks ago`;
  return `${Math.floor(days / 30)} months ago`;
}

// ── Components ────────────────────────────���─────────────────────────────

function CategoryBadge({ category }: { category: ChangeCategory }) {
  const config = CATEGORY_CONFIG[category];
  const Icon = config.icon;

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 text-[10px] font-mono uppercase tracking-wider",
        config.color,
      )}
    >
      <Icon className="w-3 h-3" />
      {config.label}
    </span>
  );
}

function ScopeBadge({ scope }: { scope: string }) {
  return (
    <span className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-mono uppercase tracking-wider text-neutral-500 bg-white/[0.03] border border-white/[0.06] rounded-sm">
      {scope}
    </span>
  );
}

function VersionTimeline({ releases: filteredReleases }: { releases: Release[] }) {
  return (
    <div className="relative">
      {/* Timeline line */}
      <div className="absolute left-[7px] top-3 bottom-3 w-px bg-white/[0.06] hidden sm:block" />

      <div className="space-y-16">
        {filteredReleases.map((release, i) => (
          <article
            key={release.version}
            id={`v${release.version}`}
            className="scroll-mt-24"
          >
            {/* Version header */}
            <div className="flex items-start gap-4 sm:gap-6 mb-8">
              {/* Timeline dot */}
              <div className="hidden sm:flex shrink-0 mt-2">
                <div
                  className={cn(
                    "w-[15px] h-[15px] rounded-full border-2",
                    i === 0
                      ? "border-[#d4af37] bg-[#d4af37]/20"
                      : "border-white/20 bg-black",
                  )}
                />
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex flex-wrap items-center gap-3 mb-2">
                  <h2 className="text-2xl sm:text-3xl font-extralight tracking-[-0.03em]">
                    {release.version}
                  </h2>
                  {release.latest && (
                    <span className="px-2 py-0.5 text-[10px] font-mono uppercase tracking-wider text-[#d4af37] border border-[#d4af37]/30 rounded-sm bg-[#d4af37]/5">
                      Latest
                    </span>
                  )}
                  <a
                    href={`https://github.com/${GITHUB_REPO}/releases/tag/v${release.version}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-[10px] font-mono text-neutral-600 hover:text-neutral-400 transition-colors duration-200"
                  >
                    <Tag className="w-3 h-3" />
                    v{release.version}
                    <ExternalLink className="w-2.5 h-2.5" />
                  </a>
                </div>

                <h3 className="text-lg font-light text-neutral-300 mb-2">
                  {release.title}
                </h3>

                <div className="flex flex-wrap items-center gap-4 text-xs text-neutral-600 font-light mb-4">
                  <span className="inline-flex items-center gap-1.5">
                    <Calendar className="w-3 h-3" />
                    {formatDate(release.date)}
                  </span>
                  <span>{daysAgo(release.date)}</span>
                  <span>{release.changes.length} changes</span>
                </div>

                <p className="text-sm text-neutral-500 font-light leading-relaxed max-w-2xl mb-8">
                  {release.summary}
                </p>

                {/* Changes list */}
                <div className="space-y-3">
                  {release.changes.map((change, j) => (
                    <div
                      key={j}
                      className="group flex items-start gap-3 py-2 px-3 -mx-3 rounded-sm hover:bg-white/[0.02] transition-colors duration-200"
                    >
                      <div className="shrink-0 mt-0.5">
                        <CategoryBadge category={change.category} />
                      </div>
                      <p className="text-sm text-neutral-400 font-light leading-relaxed flex-1">
                        {change.description}
                      </p>
                      {change.scope && (
                        <div className="shrink-0">
                          <ScopeBadge scope={change.scope} />
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                {/* Release links */}
                <div className="flex flex-wrap gap-3 mt-6 pt-6 border-t border-white/[0.04]">
                  <a
                    href={`https://github.com/${GITHUB_REPO}/releases/tag/v${release.version}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-white transition-colors duration-200 font-light"
                  >
                    <ExternalLink className="w-3 h-3" />
                    Release notes
                  </a>
                  <a
                    href={`https://github.com/${GITHUB_REPO}/compare/v${
                      i < releases.length - 1 ? releases[i + 1].version : release.version
                    }...v${release.version}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1.5 text-xs text-neutral-500 hover:text-white transition-colors duration-200 font-light"
                  >
                    <ExternalLink className="w-3 h-3" />
                    Full diff
                  </a>
                  <Link
                    href="/download"
                    className="inline-flex items-center gap-1.5 text-xs text-[#d4af37]/70 hover:text-[#d4af37] transition-colors duration-200 font-light"
                  >
                    <Download className="w-3 h-3" />
                    Download
                  </Link>
                </div>
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}

// ── Main page ───────────────────────────────────────────────────────────

export default function ChangelogPage() {
  const [filter, setFilter] = useState<ChangeCategory | "all">("all");
  const [expandedVersions, setExpandedVersions] = useState<Set<string>>(
    () => new Set(releases.map((r) => r.version)),
  );

  const filteredReleases = useMemo(() => {
    if (filter === "all") return releases;
    return releases
      .map((release) => ({
        ...release,
        changes: release.changes.filter((c) => c.category === filter),
      }))
      .filter((release) => release.changes.length > 0);
  }, [filter]);

  const totalChanges = releases.reduce((sum, r) => sum + r.changes.length, 0);
  const latestVersion = releases[0]?.version ?? "0.0.0";

  return (
    <div className="min-h-screen bg-black text-white">
      {/* Navigation */}
      <nav className="sticky top-0 z-50 backdrop-blur-xl bg-black/80 border-b border-white/[0.04]">
        <div className="max-w-4xl mx-auto px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href="/"
              className="inline-flex items-center gap-2 text-xs text-neutral-500 hover:text-white transition-colors duration-200 font-light"
            >
              <ArrowLeft className="w-3.5 h-3.5" />
              Home
            </Link>
            <span className="w-px h-4 bg-white/[0.06]" />
            <span className="text-xs font-mono text-neutral-600 tracking-wider uppercase">
              Changelog
            </span>
          </div>
          <div className="flex items-center gap-3">
            <Link
              href="/download"
              className="text-xs text-neutral-500 hover:text-white transition-colors duration-200 font-light"
            >
              Download
            </Link>
            <Link
              href="/dashboard"
              className="text-xs px-3 py-1.5 bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] rounded-sm transition-colors duration-200 font-light"
            >
              Open App
            </Link>
          </div>
        </div>
      </nav>

      <main className="max-w-4xl mx-auto px-6 py-16 sm:py-24">
        {/* Hero */}
        <header className="mb-16">
          <div className="flex flex-wrap items-center gap-3 mb-6">
            <span className="px-2.5 py-1 text-[10px] font-mono uppercase tracking-wider text-[#d4af37] border border-[#d4af37]/20 rounded-sm bg-[#d4af37]/5">
              v{latestVersion}
            </span>
            <span className="text-[10px] font-mono text-neutral-700 tracking-wider">
              {totalChanges} CHANGES SHIPPED
            </span>
          </div>

          <h1 className="text-4xl sm:text-5xl font-extralight tracking-[-0.04em] mb-4">
            Changelog
          </h1>
          <p className="text-sm text-neutral-500 font-light leading-relaxed max-w-lg">
            Every feature, fix, and improvement shipped to Nous. Follow along as
            we build the sovereign everything-app in the open.
          </p>

          {/* Subscribe / RSS hint */}
          <div className="flex items-center gap-4 mt-6">
            <a
              href={`https://github.com/${GITHUB_REPO}/releases.atom`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-xs text-neutral-600 hover:text-neutral-400 transition-colors duration-200 font-mono"
            >
              <span className="w-1.5 h-1.5 rounded-full bg-orange-500/60" />
              RSS Feed
            </a>
            <a
              href={`https://github.com/${GITHUB_REPO}/releases`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-xs text-neutral-600 hover:text-neutral-400 transition-colors duration-200 font-mono"
            >
              GitHub Releases
              <ExternalLink className="w-2.5 h-2.5" />
            </a>
          </div>
        </header>

        {/* Filter bar */}
        <div className="flex flex-wrap items-center gap-2 mb-12 pb-6 border-b border-white/[0.04]" role="tablist" aria-label="Filter changes by category">
          {FILTER_OPTIONS.map((option) => {
            const isActive = filter === option.key;
            const count =
              option.key === "all"
                ? totalChanges
                : releases.reduce(
                    (sum, r) =>
                      sum + r.changes.filter((c) => c.category === option.key).length,
                    0,
                  );
            return (
              <button
                key={option.key}
                role="tab"
                aria-selected={isActive}
                onClick={() => setFilter(option.key)}
                className={cn(
                  "inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-light rounded-sm transition-all duration-200",
                  isActive
                    ? "bg-white/[0.06] text-white border border-white/[0.1]"
                    : "text-neutral-600 hover:text-neutral-400 hover:bg-white/[0.02] border border-transparent",
                )}
              >
                {option.label}
                <span
                  className={cn(
                    "text-[10px] font-mono",
                    isActive ? "text-neutral-400" : "text-neutral-700",
                  )}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        {/* Version quick nav */}
        <div className="flex flex-wrap items-center gap-3 mb-12" aria-label="Jump to version">
          <span className="text-[10px] font-mono uppercase tracking-wider text-neutral-700">
            Jump to:
          </span>
          {releases.map((release) => (
            <a
              key={release.version}
              href={`#v${release.version}`}
              className="text-xs text-neutral-600 hover:text-white transition-colors duration-200 font-mono"
            >
              v{release.version}
            </a>
          ))}
        </div>

        {/* Timeline */}
        {filteredReleases.length > 0 ? (
          <VersionTimeline releases={filteredReleases} />
        ) : (
          <div className="text-center py-24">
            <p className="text-sm text-neutral-600 font-light">
              No changes match this filter.
            </p>
            <button
              onClick={() => setFilter("all")}
              className="mt-4 text-xs text-[#d4af37] hover:text-[#d4af37]/80 transition-colors duration-200 font-light"
            >
              Clear filter
            </button>
          </div>
        )}

        {/* Bottom CTA */}
        <div className="mt-24 pt-12 border-t border-white/[0.04] text-center">
          <p className="text-sm text-neutral-600 font-light mb-2">
            Building in the open since March 2026
          </p>
          <p className="text-xs text-neutral-700 font-light mb-6">
            Follow the project on GitHub for real-time updates
          </p>
          <div className="flex justify-center gap-4">
            <a
              href={`https://github.com/${GITHUB_REPO}`}
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-5 py-2.5 text-xs font-light bg-white/[0.04] hover:bg-white/[0.08] border border-white/[0.06] rounded-sm transition-colors duration-200"
            >
              <GithubIcon className="w-3.5 h-3.5" />
              Star on GitHub
            </a>
            <Link
              href="/download"
              className="inline-flex items-center gap-2 px-5 py-2.5 text-xs font-medium bg-[#d4af37] hover:bg-[#c4a030] active:bg-[#b39028] text-black rounded-sm transition-colors duration-200"
            >
              <Download className="w-3.5 h-3.5" />
              Download Nous
            </Link>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="border-t border-white/[0.04] py-8 px-6">
        <div className="max-w-4xl mx-auto flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-[10px] font-mono text-neutral-700 tracking-wider">
            NOUS v{latestVersion}
          </p>
          <div className="flex items-center gap-4">
            <Link href="/" className="text-xs text-neutral-600 hover:text-white transition-colors duration-200 font-light">
              Home
            </Link>
            <Link href="/download" className="text-xs text-neutral-600 hover:text-white transition-colors duration-200 font-light">
              Download
            </Link>
            <a
              href={`https://github.com/${GITHUB_REPO}`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-neutral-600 hover:text-white transition-colors duration-200 font-light"
            >
              GitHub
            </a>
          </div>
          <p className="text-[10px] font-mono text-neutral-800 tracking-wider">
            YOUR INFRASTRUCTURE. YOUR RULES.
          </p>
        </div>
      </footer>
    </div>
  );
}

// ── GitHub icon ─���─────────────────────────���─────────────────────────────

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}
