"use client";

/**
 * AppPreview — Animated CSS mockup of the Nous dashboard for the landing page.
 * Pure CSS + Tailwind, zero external deps. Shows sidebar, stat cards, activity
 * feed, and sparkline to give visitors a feel for the real product.
 */

// ── Sidebar nav items ───���────────────────────────────────────────────────

const sidebarItems = [
  { label: "Dashboard", active: true },
  { label: "Social", active: false },
  { label: "Messages", active: false },
  { label: "Wallet", active: false },
  { label: "Governance", active: false },
  { label: "AI", active: false },
  { label: "Files", active: false },
  { label: "Network", active: false },
];

// ── Stat cards ──────────��────────────────────────────────────────────────

const stats = [
  { label: "Reputation", value: "847", delta: "+12", positive: true },
  { label: "Messages", value: "23", delta: "3 new", positive: true },
  { label: "Balance", value: "2.41 ETH", delta: "+0.08", positive: true },
  { label: "Peers", value: "14", delta: "stable", positive: true },
];

// ── Activity rows ───────────────────────────────────────────���────────────

const activities = [
  { type: "governance", text: "Voted on proposal #42", time: "2m ago" },
  { type: "social", text: "New follower: anon_7f3d", time: "8m ago" },
  { type: "payment", text: "Received 0.05 ETH", time: "14m ago" },
  { type: "message", text: "Encrypted message from peer", time: "21m ago" },
  { type: "identity", text: "Credential verified", time: "1h ago" },
];

// ── Sparkline SVG (hardcoded points for deterministic rendering) ─────────

function MiniSparkline() {
  const points = [4, 6, 5, 8, 7, 10, 9, 12, 11, 14, 13, 16, 15, 18, 20];
  const max = 22;
  const w = 120;
  const h = 32;
  const step = w / (points.length - 1);

  const pathD = points
    .map((v, i) => {
      const x = i * step;
      const y = h - (v / max) * h;
      return i === 0 ? `M${x},${y}` : `L${x},${y}`;
    })
    .join(" ");

  const fillD = `${pathD} L${w},${h} L0,${h} Z`;

  return (
    <svg
      viewBox={`0 0 ${w} ${h}`}
      className="w-full h-8"
      preserveAspectRatio="none"
    >
      <defs>
        <linearGradient id="spark-fill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#d4af37" stopOpacity="0.15" />
          <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={fillD} fill="url(#spark-fill)" />
      <path
        d={pathD}
        fill="none"
        stroke="#d4af37"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="app-preview-sparkline"
      />
    </svg>
  );
}

// ─�� Dot indicator for activity type ──────────────────────────────────────

function TypeDot({ type }: { type: string }) {
  const color =
    type === "governance"
      ? "bg-[#d4af37]"
      : type === "social"
        ? "bg-blue-500"
        : type === "payment"
          ? "bg-emerald-500"
          : type === "message"
            ? "bg-purple-400"
            : "bg-neutral-500";
  return <span className={`w-1.5 h-1.5 rounded-full ${color} shrink-0`} />;
}

// ── Main component ──────────────────────────────────────────────────────

export function AppPreview() {
  return (
    <div className="relative mx-auto max-w-5xl">
      {/* Glow behind the window */}
      <div className="absolute -inset-8 bg-[radial-gradient(ellipse_at_center,#d4af37_0%,transparent_70%)] opacity-[0.03] pointer-events-none rounded-2xl" />

      {/* Window frame */}
      <div className="relative border border-white/[0.08] rounded-lg overflow-hidden bg-[#050505] shadow-2xl shadow-black/60">
        {/* Title bar */}
        <div className="flex items-center gap-2 px-4 py-2.5 bg-[#0a0a0a] border-b border-white/[0.06]">
          <span className="w-2.5 h-2.5 rounded-full bg-[#ff5f57]" />
          <span className="w-2.5 h-2.5 rounded-full bg-[#febc2e]" />
          <span className="w-2.5 h-2.5 rounded-full bg-[#28c840]" />
          <span className="flex-1 text-center text-[10px] font-mono text-neutral-700 tracking-wider select-none">
            Nous — Dashboard
          </span>
        </div>

        {/* App body */}
        <div className="flex min-h-[340px] sm:min-h-[400px]">
          {/* Sidebar */}
          <div className="hidden sm:flex flex-col w-44 border-r border-white/[0.06] bg-[#080808] py-4 px-3 shrink-0">
            {/* Logo */}
            <div className="flex items-center gap-2 px-2 mb-6">
              <div className="w-5 h-5 rounded-sm bg-[#d4af37]/20 border border-[#d4af37]/30 flex items-center justify-center">
                <span className="text-[8px] font-bold text-[#d4af37]">N</span>
              </div>
              <span className="text-xs font-extralight tracking-[-0.02em] text-neutral-300">
                Nous
              </span>
            </div>

            {/* Nav items */}
            <nav className="space-y-0.5 flex-1">
              {sidebarItems.map((item) => (
                <div
                  key={item.label}
                  className={`flex items-center gap-2.5 px-2 py-1.5 rounded-sm text-[11px] font-light transition-colors ${
                    item.active
                      ? "bg-white/[0.04] text-white"
                      : "text-neutral-600 hover:text-neutral-400"
                  }`}
                >
                  {/* Icon placeholder */}
                  <div
                    className={`w-3.5 h-3.5 rounded-sm ${
                      item.active
                        ? "bg-[#d4af37]/20 border border-[#d4af37]/30"
                        : "bg-white/[0.04] border border-white/[0.06]"
                    }`}
                  />
                  {item.label}
                </div>
              ))}
            </nav>

            {/* Bottom user */}
            <div className="flex items-center gap-2 px-2 pt-4 border-t border-white/[0.06]">
              <div className="w-5 h-5 rounded-full bg-[#d4af37]/10 border border-[#d4af37]/20" />
              <div className="min-w-0">
                <p className="text-[10px] font-light text-neutral-400 truncate">
                  anon
                </p>
                <p className="text-[8px] font-mono text-neutral-700 truncate">
                  did:key:z6Mk...
                </p>
              </div>
            </div>
          </div>

          {/* Main content */}
          <div className="flex-1 p-4 sm:p-6 overflow-hidden">
            {/* Greeting */}
            <div className="mb-5">
              <p className="text-sm sm:text-base font-extralight text-neutral-300 mb-0.5">
                Good evening
              </p>
              <p className="text-[10px] font-mono text-neutral-700">
                Node online &middot; 14 peers connected
              </p>
            </div>

            {/* Stat cards */}
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-2 sm:gap-3 mb-5">
              {stats.map((stat, i) => (
                <div
                  key={stat.label}
                  className="bg-white/[0.02] border border-white/[0.04] rounded-sm p-3 app-preview-card"
                  style={{ animationDelay: `${i * 100 + 400}ms` }}
                >
                  <p className="text-[9px] font-mono uppercase tracking-wider text-neutral-700 mb-1.5">
                    {stat.label}
                  </p>
                  <p className="text-base sm:text-lg font-extralight tracking-tight text-white leading-none mb-1">
                    {stat.value}
                  </p>
                  <p
                    className={`text-[9px] font-mono ${
                      stat.positive ? "text-emerald-600" : "text-red-500"
                    }`}
                  >
                    {stat.delta}
                  </p>
                </div>
              ))}
            </div>

            {/* Two columns: activity + chart */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
              {/* Activity feed */}
              <div
                className="bg-white/[0.01] border border-white/[0.04] rounded-sm p-3 app-preview-card"
                style={{ animationDelay: "800ms" }}
              >
                <p className="text-[9px] font-mono uppercase tracking-wider text-neutral-600 mb-3">
                  Recent Activity
                </p>
                <div className="space-y-2">
                  {activities.map((a, i) => (
                    <div
                      key={i}
                      className="flex items-center gap-2 app-preview-row"
                      style={{ animationDelay: `${900 + i * 80}ms` }}
                    >
                      <TypeDot type={a.type} />
                      <span className="text-[10px] font-light text-neutral-400 flex-1 truncate">
                        {a.text}
                      </span>
                      <span className="text-[8px] font-mono text-neutral-700 shrink-0">
                        {a.time}
                      </span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Reputation chart */}
              <div
                className="bg-white/[0.01] border border-white/[0.04] rounded-sm p-3 app-preview-card"
                style={{ animationDelay: "900ms" }}
              >
                <div className="flex items-center justify-between mb-3">
                  <p className="text-[9px] font-mono uppercase tracking-wider text-neutral-600">
                    Reputation
                  </p>
                  <p className="text-[9px] font-mono text-emerald-700">
                    +12 this week
                  </p>
                </div>
                <div className="mt-2">
                  <MiniSparkline />
                </div>
                <div className="flex items-baseline gap-2 mt-2">
                  <span className="text-lg font-extralight text-white">
                    847
                  </span>
                  <span className="text-[9px] font-mono text-neutral-600">
                    total score
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Reflection / shadow at bottom */}
      <div className="h-16 bg-gradient-to-b from-white/[0.02] to-transparent rounded-b-lg -mt-px mx-4 opacity-40" />
    </div>
  );
}
