"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  governance,
  type ProposalResponse,
  type VoteResultResponse,
} from "@/lib/api";

export default function GovernancePage() {
  const [proposals, setProposals] = useState<ProposalResponse[]>([]);
  const [tallies, setTallies] = useState<Record<string, VoteResultResponse>>(
    {}
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadProposals = useCallback(async () => {
    try {
      const data = await governance.listProposals();
      setProposals(data.proposals);

      const tallyResults: Record<string, VoteResultResponse> = {};
      for (const p of data.proposals) {
        try {
          tallyResults[p.id] = await governance.getTally(p.id);
        } catch {
          // Tally not available yet
        }
      }
      setTallies(tallyResults);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load proposals");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadProposals();
  }, [loadProposals]);

  function formatDeadline(votingEnds: string): string {
    const end = new Date(votingEnds);
    const now = new Date();
    const diff = end.getTime() - now.getTime();
    if (diff <= 0) return "Ended";
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    if (days > 0) return `${days} day${days > 1 ? "s" : ""}`;
    const hours = Math.floor(diff / (1000 * 60 * 60));
    return `${hours}h`;
  }

  function statusColor(status: string): string {
    const s = status.toLowerCase();
    if (s === "active") return "text-[#d4af37]";
    if (s === "passed" || s === "executed") return "text-emerald-600";
    if (s === "rejected" || s === "cancelled") return "text-red-700";
    return "text-neutral-600";
  }

  return (
    <div className="p-8 max-w-4xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Governance
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          Quadratic voting. Every voice weighted fairly.
        </p>
      </header>

      {error && (
        <div className="text-xs text-red-500/70 font-mono mb-6 px-1">
          {error}
        </div>
      )}

      <section className="mb-12">
        <div className="flex items-center justify-between mb-8">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
            Proposals
          </h2>
          <Button
            variant="outline"
            size="sm"
            className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
          >
            New Proposal
          </Button>
        </div>

        {loading ? (
          <p className="text-xs text-neutral-700 font-mono">Loading...</p>
        ) : proposals.length === 0 ? (
          <p className="text-sm text-neutral-600 font-light">
            No proposals yet. Create the first one.
          </p>
        ) : (
          <div className="space-y-px">
            {proposals.map((p) => {
              const tally = tallies[p.id];
              const votesFor = tally?.votes_for || 0;
              const votesAgainst = tally?.votes_against || 0;
              const total = votesFor + votesAgainst;
              const forPct = total > 0 ? (votesFor / total) * 100 : 0;

              return (
                <Card
                  key={p.id}
                  className={cn(
                    "bg-transparent border-0 rounded-none cursor-pointer transition-colors duration-150",
                    selectedId === p.id
                      ? "bg-white/[0.02]"
                      : "hover:bg-white/[0.01]"
                  )}
                  onClick={() =>
                    setSelectedId(selectedId === p.id ? null : p.id)
                  }
                >
                  <CardContent className="p-5">
                    <div className="flex items-start justify-between mb-3">
                      <div>
                        <div className="flex items-center gap-3 mb-1">
                          <span className="text-[10px] font-mono text-neutral-700">
                            {p.id.slice(0, 12)}...
                          </span>
                          <span
                            className={cn(
                              "text-[10px] font-mono uppercase tracking-wider",
                              statusColor(p.status)
                            )}
                          >
                            {p.status}
                          </span>
                        </div>
                        <h3 className="text-sm font-light">{p.title}</h3>
                      </div>
                      <span className="text-[10px] text-neutral-700 shrink-0 ml-4">
                        {formatDeadline(p.voting_ends)}
                      </span>
                    </div>

                    {selectedId === p.id && (
                      <div className="mt-4 pt-4 border-t border-white/[0.04]">
                        <p className="text-xs text-neutral-500 font-light leading-relaxed mb-4">
                          {p.description}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-700 mb-2">
                          Proposed by {p.proposer_did}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-700 mb-4">
                          Quorum: {(p.quorum * 100).toFixed(0)}% | Threshold:{" "}
                          {(p.threshold * 100).toFixed(0)}%
                        </p>
                        {p.status === "Active" && (
                          <div className="flex gap-3">
                            <Button
                              variant="outline"
                              size="sm"
                              className="text-xs font-mono uppercase tracking-wider border-emerald-900 text-emerald-600 hover:bg-emerald-950"
                            >
                              Vote For
                            </Button>
                            <Button
                              variant="outline"
                              size="sm"
                              className="text-xs font-mono uppercase tracking-wider border-red-900 text-red-700 hover:bg-red-950"
                            >
                              Vote Against
                            </Button>
                          </div>
                        )}
                      </div>
                    )}

                    <div className="mt-3">
                      <div className="flex justify-between text-[10px] font-mono text-neutral-700 mb-1.5">
                        <span>{votesFor} for</span>
                        <span>{votesAgainst} against</span>
                      </div>
                      <div className="h-px bg-white/[0.06] relative">
                        <div
                          className="absolute inset-y-0 left-0 bg-[#d4af37]/40"
                          style={{ width: `${forPct}%` }}
                        />
                      </div>
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
