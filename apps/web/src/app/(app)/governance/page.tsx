"use client";

import { useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface Proposal {
  id: string;
  title: string;
  description: string;
  proposer: string;
  status: "active" | "passed" | "rejected";
  votesFor: number;
  votesAgainst: number;
  deadline: string;
}

const proposals: Proposal[] = [
  {
    id: "NP-001",
    title: "Enable relay node incentives",
    description:
      "Allocate 5% of transaction fees to relay operators who maintain >99% uptime for gossipsub message forwarding.",
    proposer: "did:key:z6Mk...x3rW",
    status: "active",
    votesFor: 42,
    votesAgainst: 7,
    deadline: "3 days",
  },
  {
    id: "NP-002",
    title: "Increase max message size to 64KB",
    description:
      "Current 16KB limit restricts file sharing use cases. Proposal to increase to 64KB with optional compression.",
    proposer: "did:key:z6Mk...q7nV",
    status: "active",
    votesFor: 18,
    votesAgainst: 23,
    deadline: "5 days",
  },
  {
    id: "NP-003",
    title: "Add Arweave storage backend",
    description:
      "Integrate Arweave as a permanent storage option alongside IPFS for critical governance records.",
    proposer: "did:key:z6Mk...8fHj",
    status: "passed",
    votesFor: 156,
    votesAgainst: 12,
    deadline: "Ended",
  },
];

export default function GovernancePage() {
  const [selectedId, setSelectedId] = useState<string | null>(null);

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

        <div className="space-y-px">
          {proposals.map((p) => {
            const total = p.votesFor + p.votesAgainst;
            const forPct = total > 0 ? (p.votesFor / total) * 100 : 0;

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
                          {p.id}
                        </span>
                        <span
                          className={cn(
                            "text-[10px] font-mono uppercase tracking-wider",
                            p.status === "active" && "text-[#d4af37]",
                            p.status === "passed" && "text-emerald-600",
                            p.status === "rejected" && "text-red-700"
                          )}
                        >
                          {p.status}
                        </span>
                      </div>
                      <h3 className="text-sm font-light">{p.title}</h3>
                    </div>
                    <span className="text-[10px] text-neutral-700 shrink-0 ml-4">
                      {p.deadline}
                    </span>
                  </div>

                  {selectedId === p.id && (
                    <div className="mt-4 pt-4 border-t border-white/[0.04]">
                      <p className="text-xs text-neutral-500 font-light leading-relaxed mb-4">
                        {p.description}
                      </p>
                      <p className="text-[10px] font-mono text-neutral-700 mb-4">
                        Proposed by {p.proposer}
                      </p>
                      {p.status === "active" && (
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
                      <span>{p.votesFor} for</span>
                      <span>{p.votesAgainst} against</span>
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
      </section>
    </div>
  );
}
