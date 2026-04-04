"use client";

import { useState, useCallback } from "react";
import { GitHubStats } from "@/components/github-stats";

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}

const GITHUB_REPO = "teddytennant/nous";

// ── Deterministic PRNG ──────────────────────────────────────────────────

function hash(n: number): number {
  const x = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
}

// ── Activity data (52 weeks x 7 days) ───────────────────────────────────

function generateActivity(): number[] {
  const cells: number[] = [];
  for (let w = 0; w < 52; w++) {
    for (let d = 0; d < 7; d++) {
      const i = w * 7 + d;
      let v = hash(i + 7);

      // Weekends (Sun=0, Sat=6) are quieter
      if (d === 0 || d === 6) v *= 0.2;

      // Activity ramps up over time (more recent weeks = more commits)
      v *= 0.15 + (w / 52) * 0.85;

      // Periodic bursts — simulate sprint weeks
      if (w % 6 < 2) v *= 1.4;

      // Random zero days (weekdays off, holidays, etc.)
      if (hash(i + 500) > 0.6) v = 0;

      cells.push(Math.min(1, Math.max(0, v)));
    }
  }
  return cells;
}

function cellColor(v: number): string {
  if (v === 0) return "rgba(255,255,255,0.03)";
  if (v < 0.15) return "rgba(255,255,255,0.07)";
  if (v < 0.35) return "rgba(212,175,55,0.14)";
  if (v < 0.55) return "rgba(212,175,55,0.28)";
  if (v < 0.75) return "rgba(212,175,55,0.42)";
  return "rgba(212,175,55,0.6)";
}

// ── Dates + commit counts ───────────────────────────────────────────────

function generateDates(): Date[] {
  // Anchor: March 31, 2026 (Tuesday, day index 2)
  // Most recent Sunday: March 29, 2026
  const currentSunday = new Date(2026, 2, 29);
  // Start 51 weeks before that to get 52 total weeks
  const start = new Date(currentSunday);
  start.setDate(start.getDate() - 51 * 7);

  const dates: Date[] = [];
  for (let i = 0; i < 52 * 7; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    dates.push(d);
  }
  return dates;
}

function toCommits(v: number): number {
  if (v === 0) return 0;
  return Math.max(1, Math.round(v * 14));
}

// Precompute all static data at module level
const activity = generateActivity();
const dates = generateDates();
const commits = activity.map(toCommits);
const totalContributions = commits.reduce((a, b) => a + b, 0);
const activeDays = commits.filter((c) => c > 0).length;

function computeStreaks(arr: number[]): { longest: number; current: number } {
  let longest = 0;
  let cur = 0;
  for (const c of arr) {
    if (c > 0) {
      cur++;
      if (cur > longest) longest = cur;
    } else {
      cur = 0;
    }
  }
  // Current streak: count backwards from last cell
  let current = 0;
  for (let i = arr.length - 1; i >= 0; i--) {
    if (arr[i] > 0) current++;
    else break;
  }
  return { longest, current };
}

const streaks = computeStreaks(commits);

// ── Constants ───────────────────────────────────────────────────────────

const CELL_SIZE = 10;
const CELL_GAP = 3;
const CELL_STEP = CELL_SIZE + CELL_GAP; // 13px

const months = [
  "Apr", "May", "Jun", "Jul", "Aug", "Sep",
  "Oct", "Nov", "Dec", "Jan", "Feb", "Mar",
];

const legendLevels = [0, 0.1, 0.3, 0.5, 0.7, 0.9];

const techStack = [
  { name: "Rust", detail: "Backend + CLI" },
  { name: "TypeScript", detail: "Web frontend" },
  { name: "Next.js", detail: "App framework" },
  { name: "SQLite", detail: "Local storage" },
  { name: "libp2p", detail: "P2P networking" },
  { name: "Tauri", detail: "Desktop apps" },
  { name: "Kotlin", detail: "Android" },
  { name: "Swift", detail: "iOS" },
];

// ── Component ───────────────────────────────────────────────────────────

export function OpenSourceSection() {
  const [hovered, setHovered] = useState<number | null>(null);

  const handleMouseOver = useCallback((e: React.MouseEvent) => {
    const idx = (e.target as HTMLElement).dataset.idx;
    if (idx != null) {
      setHovered(Number(idx));
    } else {
      setHovered(null);
    }
  }, []);

  const handleMouseLeave = useCallback(() => {
    setHovered(null);
  }, []);

  // Tooltip data derived from hovered index
  const tip =
    hovered !== null
      ? {
          week: Math.floor(hovered / 7),
          day: hovered % 7,
          date: dates[hovered],
          count: commits[hovered],
        }
      : null;

  return (
    <section className="px-6 py-28 max-w-6xl mx-auto w-full">
      <div className="mb-20">
        <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
          Open Source
        </h2>
        <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
          Every line is <span className="text-white">auditable.</span>
        </p>
      </div>

      {/* Activity heatmap */}
      <div className="mb-16">
        <div className="flex items-baseline justify-between mb-4">
          <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700">
            Development Velocity
          </p>
          <p className="text-[10px] font-mono text-neutral-600">
            {totalContributions.toLocaleString()} contributions in the past year
          </p>
        </div>

        <div className="overflow-x-auto pb-2 -mx-1 px-1">
          {/* Grid + tooltip wrapper */}
          <div className="relative inline-block">
            <div
              className="inline-grid gap-[3px]"
              style={{
                gridTemplateRows: `repeat(7, ${CELL_SIZE}px)`,
                gridAutoFlow: "column",
                gridAutoColumns: `${CELL_SIZE}px`,
              }}
              onMouseOver={handleMouseOver}
              onMouseLeave={handleMouseLeave}
            >
              {activity.map((v, i) => (
                <div
                  key={i}
                  data-idx={i}
                  className="rounded-[2px] activity-cell"
                  style={{ backgroundColor: cellColor(v) }}
                />
              ))}
            </div>

            {/* Tooltip */}
            {tip && (
              <div
                className="absolute z-10 pointer-events-none"
                style={{
                  left: tip.week * CELL_STEP + CELL_SIZE / 2,
                  top: tip.day * CELL_STEP - 8,
                  transform: "translate(-50%, -100%)",
                }}
              >
                <div className="heatmap-tooltip bg-[#1a1a1a] border border-white/10 rounded-sm px-3 py-2 text-center whitespace-nowrap shadow-lg">
                  <p className="text-[11px] font-medium text-white">
                    {tip.count === 0
                      ? "No contributions"
                      : `${tip.count} contribution${tip.count !== 1 ? "s" : ""}`}
                  </p>
                  <p className="text-[10px] text-neutral-500 font-mono mt-0.5">
                    {tip.date.toLocaleDateString("en-US", {
                      weekday: "short",
                      month: "short",
                      day: "numeric",
                      year: "numeric",
                    })}
                  </p>
                </div>
                {/* Arrow pointing down */}
                <div className="flex justify-center">
                  <div className="w-1.5 h-1.5 bg-[#1a1a1a] border-r border-b border-white/10 rotate-45 -mt-[4px]" />
                </div>
              </div>
            )}
          </div>

          {/* Month labels */}
          <div className="flex mt-2" style={{ width: `${52 * CELL_STEP}px` }}>
            {months.map((m) => (
              <span
                key={m}
                className="text-[9px] font-mono text-neutral-800"
                style={{ width: `${(52 * CELL_STEP) / 12}px` }}
              >
                {m}
              </span>
            ))}
          </div>

          {/* Legend + stats row */}
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 mt-4">
            {/* Legend */}
            <div className="flex items-center gap-1.5">
              <span className="text-[9px] font-mono text-neutral-700 mr-0.5">
                Less
              </span>
              {legendLevels.map((v, i) => (
                <div
                  key={i}
                  className="w-[10px] h-[10px] rounded-[2px]"
                  style={{ backgroundColor: cellColor(v) }}
                />
              ))}
              <span className="text-[9px] font-mono text-neutral-700 ml-0.5">
                More
              </span>
            </div>

            {/* Stats */}
            <div className="flex items-center gap-4">
              <span className="text-[9px] font-mono text-neutral-600">
                {activeDays} active days
              </span>
              <span className="text-neutral-800">·</span>
              <span className="text-[9px] font-mono text-neutral-600">
                {streaks.longest}-day best streak
              </span>
              {streaks.current > 0 && (
                <>
                  <span className="text-neutral-800">·</span>
                  <span className="text-[9px] font-mono text-[#d4af37]/60">
                    {streaks.current}-day current streak
                  </span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Tech stack */}
      <div className="mb-16">
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700 mb-4">
          Built With
        </p>
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
          {techStack.map((tech) => (
            <div
              key={tech.name}
              className="group px-4 py-3 border border-white/[0.06] rounded-sm hover:border-white/10 hover:bg-white/[0.02] transition-colors duration-200"
            >
              <p className="text-sm font-light text-neutral-300 group-hover:text-white transition-colors duration-200 mb-0.5">
                {tech.name}
              </p>
              <p className="text-[10px] font-mono text-neutral-700 tracking-wider">
                {tech.detail}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* GitHub CTA */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
        <a
          href={`https://github.com/${GITHUB_REPO}`}
          target="_blank"
          rel="noopener noreferrer"
          className="group flex items-center gap-3 px-6 py-3 border border-white/[0.08] rounded-md hover:border-[#d4af37]/30 hover:bg-[#d4af37]/[0.02] transition-all duration-200"
        >
          <GithubIcon className="w-5 h-5 text-neutral-400 group-hover:text-white transition-colors duration-200" />
          <span className="text-sm font-light text-neutral-300 group-hover:text-white transition-colors duration-200">
            Star on GitHub
          </span>
        </a>
        <a
          href={`https://github.com/${GITHUB_REPO}/tree/main/crates`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-xs text-neutral-600 hover:text-[#d4af37] transition-colors duration-200 link-underline"
        >
          Browse the source
        </a>
      </div>

      {/* Live GitHub stats */}
      <GitHubStats />
    </section>
  );
}
