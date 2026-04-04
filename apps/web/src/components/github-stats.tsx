"use client";

import { useEffect, useState } from "react";

interface RepoStats {
  stars: number;
  forks: number;
  watchers: number;
  openIssues: number;
  language: string;
  license: string;
  size: number; // KB
  updatedAt: string;
}

const GITHUB_REPO = "teddytennant/nous";
const CACHE_KEY = "nous_gh_stats";
const CACHE_TTL = 5 * 60 * 1000; // 5 minutes

function getCached(): { data: RepoStats; ts: number } | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (Date.now() - parsed.ts < CACHE_TTL) return parsed;
  } catch {
    // ignore
  }
  return null;
}

function setCache(data: RepoStats) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ data, ts: Date.now() }));
  } catch {
    // ignore
  }
}

function formatNumber(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return n.toString();
}

function formatSize(kb: number): string {
  if (kb >= 1024) return `${(kb / 1024).toFixed(1)} MB`;
  return `${kb} KB`;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const hours = Math.floor(diff / 3_600_000);
  if (hours < 1) return "just now";
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return `${Math.floor(days / 30)}mo ago`;
}

function StarIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" className={className}>
      <path d="M8 .25a.75.75 0 0 1 .673.418l1.882 3.815 4.21.612a.75.75 0 0 1 .416 1.279l-3.046 2.97.719 4.192a.75.75 0 0 1-1.088.791L8 12.347l-3.766 1.98a.75.75 0 0 1-1.088-.79l.72-4.194L.818 6.374a.75.75 0 0 1 .416-1.28l4.21-.611L7.327.668A.75.75 0 0 1 8 .25Z" />
    </svg>
  );
}

function ForkIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" className={className}>
      <path d="M5 5.372v.878c0 .414.336.75.75.75h4.5a.75.75 0 0 0 .75-.75v-.878a2.25 2.25 0 1 1 1.5 0v.878a2.25 2.25 0 0 1-2.25 2.25h-1.5v2.128a2.251 2.251 0 1 1-1.5 0V8.5h-1.5A2.25 2.25 0 0 1 3.5 6.25v-.878a2.25 2.25 0 1 1 1.5 0ZM5 3.25a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Zm6.75.75a.75.75 0 1 0 0-1.5.75.75 0 0 0 0 1.5Zm-3 8.75a.75.75 0 1 0-1.5 0 .75.75 0 0 0 1.5 0Z" />
    </svg>
  );
}

function EyeIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" className={className}>
      <path d="M8 2c1.981 0 3.671.992 4.933 2.078 1.27 1.091 2.187 2.345 2.637 3.023a1.62 1.62 0 0 1 0 1.798c-.45.678-1.367 1.932-2.637 3.023C11.67 13.008 9.981 14 8 14c-1.981 0-3.671-.992-4.933-2.078C1.797 10.831.88 9.577.43 8.899a1.62 1.62 0 0 1 0-1.798c.45-.678 1.367-1.932 2.637-3.023C4.33 2.992 6.019 2 8 2ZM1.679 7.932a.12.12 0 0 0 0 .136c.411.622 1.241 1.75 2.366 2.717C5.176 11.758 6.527 12.5 8 12.5c1.473 0 2.825-.742 3.955-1.715 1.124-.967 1.954-2.096 2.366-2.717a.12.12 0 0 0 0-.136c-.412-.621-1.242-1.75-2.366-2.717C10.824 4.242 9.473 3.5 8 3.5c-1.473 0-2.824.742-3.955 1.715-1.124.967-1.954 2.096-2.366 2.717ZM8 10a2 2 0 1 1-.001-3.999A2 2 0 0 1 8 10Z" />
    </svg>
  );
}

function IssueIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 16 16" fill="currentColor" className={className}>
      <path d="M8 9.5a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z" />
      <path d="M8 0a8 8 0 1 1 0 16A8 8 0 0 1 8 0ZM1.5 8a6.5 6.5 0 1 0 13 0 6.5 6.5 0 0 0-13 0Z" />
    </svg>
  );
}

// Static fallback for SSR/initial render
const FALLBACK_STATS: RepoStats = {
  stars: 0,
  forks: 0,
  watchers: 0,
  openIssues: 0,
  language: "Rust",
  license: "MIT",
  size: 0,
  updatedAt: "",
};

export function GitHubStats() {
  const [stats, setStats] = useState<RepoStats | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    const cached = getCached();
    if (cached) {
      setStats(cached.data);
      return;
    }

    fetch(`https://api.github.com/repos/${GITHUB_REPO}`, {
      headers: { Accept: "application/vnd.github.v3+json" },
    })
      .then((res) => {
        if (!res.ok) throw new Error("fetch failed");
        return res.json();
      })
      .then((data) => {
        const parsed: RepoStats = {
          stars: data.stargazers_count ?? 0,
          forks: data.forks_count ?? 0,
          watchers: data.subscribers_count ?? 0,
          openIssues: data.open_issues_count ?? 0,
          language: data.language ?? "Rust",
          license: data.license?.spdx_id ?? "MIT",
          size: data.size ?? 0,
          updatedAt: data.pushed_at ?? "",
        };
        setStats(parsed);
        setCache(parsed);
      })
      .catch(() => {
        setError(true);
      });
  }, []);

  // Don't render if we have no data and hit an error (private repo, rate limited)
  if (error && !stats) return null;

  const data = stats || FALLBACK_STATS;
  const hasData = stats !== null;

  const metrics = [
    { icon: StarIcon, label: "Stars", value: formatNumber(data.stars) },
    { icon: ForkIcon, label: "Forks", value: formatNumber(data.forks) },
    { icon: EyeIcon, label: "Watchers", value: formatNumber(data.watchers) },
    { icon: IssueIcon, label: "Issues", value: formatNumber(data.openIssues) },
  ];

  return (
    <div className="mt-16 pt-12 border-t border-white/[0.04]">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 mb-6">
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700">
          Repository
        </p>
        {hasData && data.updatedAt && (
          <p className="text-[10px] font-mono text-neutral-700">
            Last pushed {timeAgo(data.updatedAt)}
          </p>
        )}
      </div>

      {/* Stats grid */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-white/[0.04] rounded-sm overflow-hidden mb-8">
        {metrics.map((metric) => {
          const Icon = metric.icon;
          return (
            <div
              key={metric.label}
              className="bg-black p-5 sm:p-6 group hover:bg-white/[0.02] transition-colors duration-200"
            >
              <div className="flex items-center gap-2 mb-2">
                <Icon className="w-3.5 h-3.5 text-neutral-600 group-hover:text-[#d4af37] transition-colors duration-200" />
                <span className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700">
                  {metric.label}
                </span>
              </div>
              {hasData ? (
                <p className="text-xl sm:text-2xl font-extralight tracking-[-0.02em] text-white tabular-nums">
                  {metric.value}
                </p>
              ) : (
                <div className="h-7 w-12 bg-white/[0.04] rounded-sm animate-pulse" />
              )}
            </div>
          );
        })}
      </div>

      {/* Meta row */}
      {hasData && (
        <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
          {data.language && (
            <div className="flex items-center gap-2">
              <span className="w-2.5 h-2.5 rounded-full bg-[#dea584]" />
              <span className="text-xs font-light text-neutral-500">
                {data.language}
              </span>
            </div>
          )}
          {data.license && (
            <span className="text-xs font-light text-neutral-600">
              {data.license} license
            </span>
          )}
          {data.size > 0 && (
            <span className="text-xs font-light text-neutral-700">
              {formatSize(data.size)}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
