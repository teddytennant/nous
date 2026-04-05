"use client";

import { useState, useCallback } from "react";
import { Check, Clock, ArrowRight } from "lucide-react";

/* ── Types ─────────────────────────────────────────────────────────────── */

type ItemStatus = "done" | "in-progress" | "planned";

interface RoadmapItem {
  label: string;
  status: ItemStatus;
}

interface RoadmapPhase {
  name: string;
  quarter: string;
  description: string;
  items: RoadmapItem[];
}

/* ── Data ──────────────────────────────────────────────────────────────── */

const phases: RoadmapPhase[] = [
  {
    name: "Foundation",
    quarter: "Q1 2026",
    description: "Core architecture, crypto primitives, and local-first storage",
    items: [
      { label: "20-crate Rust workspace", status: "done" },
      { label: "Ed25519 + X25519 + AES-256-GCM crypto", status: "done" },
      { label: "DID:key identity system", status: "done" },
      { label: "SQLite + CRDT local-first storage", status: "done" },
      { label: "REST + GraphQL + gRPC APIs", status: "done" },
      { label: "End-to-end encrypted messaging", status: "done" },
      { label: "CLI and TUI interfaces", status: "done" },
    ],
  },
  {
    name: "Platform",
    quarter: "Q2 2026",
    description: "Social, governance, payments, and cross-platform apps",
    items: [
      { label: "Decentralized social feeds", status: "done" },
      { label: "Quadratic voting governance", status: "done" },
      { label: "Multi-chain wallet + escrow", status: "done" },
      { label: "P2P marketplace with disputes", status: "done" },
      { label: "Next.js web app", status: "done" },
      { label: "Tauri desktop apps (macOS, Linux, Windows)", status: "in-progress" },
      { label: "Android native app", status: "in-progress" },
      { label: "iOS TestFlight beta", status: "planned" },
    ],
  },
  {
    name: "Intelligence",
    quarter: "Q3 2026",
    description: "Local AI, semantic search, and autonomous agents",
    items: [
      { label: "Local LLM inference", status: "in-progress" },
      { label: "Semantic search across all data", status: "planned" },
      { label: "Agent framework for task automation", status: "planned" },
      { label: "On-device RAG pipeline", status: "planned" },
      { label: "Voice and image understanding", status: "planned" },
    ],
  },
  {
    name: "Network",
    quarter: "Q4 2026",
    description: "Federation, public nodes, and ecosystem growth",
    items: [
      { label: "Federated node discovery", status: "planned" },
      { label: "Public relay infrastructure", status: "planned" },
      { label: "Plugin / extension system", status: "planned" },
      { label: "Developer SDK and docs site", status: "planned" },
      { label: "Mobile push notifications", status: "planned" },
      { label: "Nostr + ActivityPub bridges", status: "planned" },
    ],
  },
];

/* ── Status indicators ────────────────────────────────────────────────── */

function StatusIcon({ status }: { status: ItemStatus }) {
  switch (status) {
    case "done":
      return (
        <div className="w-4 h-4 rounded-full bg-[#d4af37]/15 flex items-center justify-center shrink-0">
          <Check className="w-2.5 h-2.5 text-[#d4af37]" />
        </div>
      );
    case "in-progress":
      return (
        <div className="w-4 h-4 rounded-full border border-[#d4af37]/30 flex items-center justify-center shrink-0">
          <div className="w-1.5 h-1.5 rounded-full bg-[#d4af37] animate-pulse" />
        </div>
      );
    case "planned":
      return (
        <div className="w-4 h-4 rounded-full border border-white/[0.08] shrink-0" />
      );
  }
}

function statusLabel(status: ItemStatus): string {
  switch (status) {
    case "done":
      return "Shipped";
    case "in-progress":
      return "In progress";
    case "planned":
      return "Planned";
  }
}

/* ── Phase Card ───────────────────────────────────────────────────────── */

function PhaseCard({
  phase,
  index,
  isActive,
  onSelect,
}: {
  phase: RoadmapPhase;
  index: number;
  isActive: boolean;
  onSelect: () => void;
}) {
  const doneCount = phase.items.filter((i) => i.status === "done").length;
  const total = phase.items.length;
  const progress = total > 0 ? (doneCount / total) * 100 : 0;

  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={isActive}
      className={`text-left w-full p-5 rounded-sm border transition-all duration-200 ${
        isActive
          ? "border-[#d4af37]/20 bg-[#d4af37]/[0.03]"
          : "border-white/[0.06] hover:border-white/[0.1] hover:bg-white/[0.02]"
      }`}
    >
      <div className="flex items-center justify-between mb-2">
        <span className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
          Phase {index + 1}
        </span>
        <span
          className={`text-[10px] font-mono tracking-wider ${
            isActive ? "text-[#d4af37]" : "text-neutral-700"
          }`}
        >
          {phase.quarter}
        </span>
      </div>
      <h3
        className={`text-sm font-medium mb-1 transition-colors duration-200 ${
          isActive ? "text-white" : "text-neutral-400"
        }`}
      >
        {phase.name}
      </h3>
      <p className="text-xs text-neutral-600 font-light mb-3">
        {phase.description}
      </p>

      {/* Progress bar */}
      <div className="h-px bg-white/[0.06] rounded-full overflow-hidden">
        <div
          className="h-full bg-[#d4af37]/40 rounded-full transition-all duration-500"
          style={{ width: `${progress}%` }}
        />
      </div>
      <p className="text-[10px] font-mono text-neutral-700 mt-1.5">
        {doneCount}/{total} shipped
      </p>
    </button>
  );
}

/* ── Roadmap Section ──────────────────────────────────────────────────── */

export function RoadmapSection() {
  const [activePhase, setActivePhase] = useState(1); // Default to current phase

  const handleSelect = useCallback((i: number) => {
    setActivePhase(i);
  }, []);

  const current = phases[activePhase];

  return (
    <section id="roadmap" className="px-6 py-28 max-w-6xl mx-auto w-full scroll-mt-16">
      <div className="mb-20">
        <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
          Roadmap
        </h2>
        <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
          Where we&apos;re going.{" "}
          <span className="text-white">Transparently.</span>
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-8">
        {/* Phase selector */}
        <div className="flex lg:flex-col gap-3 overflow-x-auto lg:overflow-x-visible pb-2 lg:pb-0">
          {phases.map((phase, i) => (
            <div key={phase.name} className="min-w-[220px] lg:min-w-0">
              <PhaseCard
                phase={phase}
                index={i}
                isActive={activePhase === i}
                onSelect={() => handleSelect(i)}
              />
            </div>
          ))}
        </div>

        {/* Active phase items */}
        <div>
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-base font-medium">{current.name}</h3>
            <div className="flex items-center gap-4">
              {(["done", "in-progress", "planned"] as ItemStatus[]).map(
                (status) => {
                  const count = current.items.filter(
                    (i) => i.status === status,
                  ).length;
                  if (count === 0) return null;
                  return (
                    <div
                      key={status}
                      className="flex items-center gap-1.5"
                    >
                      <StatusIcon status={status} />
                      <span className="text-[10px] font-mono text-neutral-600">
                        {count} {statusLabel(status).toLowerCase()}
                      </span>
                    </div>
                  );
                },
              )}
            </div>
          </div>

          <div className="space-y-1">
            {current.items.map((item) => (
              <div
                key={item.label}
                className="flex items-center gap-3 py-3 px-3 rounded-sm hover:bg-white/[0.02] transition-colors duration-150 group"
              >
                <StatusIcon status={item.status} />
                <span
                  className={`text-sm font-light transition-colors duration-200 ${
                    item.status === "done"
                      ? "text-neutral-500"
                      : item.status === "in-progress"
                        ? "text-neutral-300 group-hover:text-white"
                        : "text-neutral-600 group-hover:text-neutral-400"
                  }`}
                >
                  {item.label}
                </span>
                {item.status === "in-progress" && (
                  <span className="text-[9px] font-mono uppercase tracking-wider text-[#d4af37]/60 ml-auto">
                    Active
                  </span>
                )}
              </div>
            ))}
          </div>

          {/* Timeline connector */}
          <div className="mt-8 pt-6 border-t border-white/[0.04] flex items-center gap-2">
            <Clock className="w-3.5 h-3.5 text-neutral-700" />
            <p className="text-xs text-neutral-600 font-light">
              Roadmap updated April 2026. Priorities shift based on community feedback.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
