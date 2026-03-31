"use client";

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}

const GITHUB_REPO = "teddytennant/nous";

// Deterministic PRNG for consistent heatmap across renders
function hash(n: number): number {
  const x = Math.sin(n * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
}

// Generate 52 weeks x 7 days of activity intensities
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

const activity = generateActivity();

const months = [
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
  "Jan",
  "Feb",
  "Mar",
];

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

export function OpenSourceSection() {
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
        <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-700 mb-4">
          Development Velocity
        </p>
        <div className="overflow-x-auto pb-2 -mx-1 px-1">
          <div
            className="inline-grid gap-[3px]"
            style={{
              gridTemplateRows: "repeat(7, 10px)",
              gridAutoFlow: "column",
              gridAutoColumns: "10px",
            }}
          >
            {activity.map((v, i) => (
              <div
                key={i}
                className="rounded-[2px] activity-cell"
                style={{ backgroundColor: cellColor(v) }}
              />
            ))}
          </div>
          {/* Month labels */}
          <div className="flex mt-2" style={{ width: `${52 * 13}px` }}>
            {months.map((m) => (
              <span
                key={m}
                className="text-[9px] font-mono text-neutral-800"
                style={{ width: `${(52 * 13) / 12}px` }}
              >
                {m}
              </span>
            ))}
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
    </section>
  );
}
