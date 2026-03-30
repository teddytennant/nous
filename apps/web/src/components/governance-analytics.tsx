"use client";

import { useMemo } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type {
  DaoResponse,
  ProposalResponse,
  VoteResultResponse,
} from "@/lib/api";

interface AnalyticsProps {
  daos: DaoResponse[];
  proposals: ProposalResponse[];
  tallies: Record<string, VoteResultResponse>;
}

interface StatCardProps {
  label: string;
  value: string | number;
  sub?: string;
}

function StatCard({ label, value, sub }: StatCardProps) {
  return (
    <div className="p-5">
      <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-3">
        {label}
      </p>
      <p className="text-2xl font-extralight tracking-tight">{value}</p>
      {sub && (
        <p className="text-[10px] font-mono text-neutral-700 mt-1">{sub}</p>
      )}
    </div>
  );
}

function StatusBar({
  counts,
}: {
  counts: { label: string; value: number; color: string }[];
}) {
  const total = counts.reduce((acc, c) => acc + c.value, 0);
  if (total === 0) return null;

  return (
    <div>
      <div className="flex h-1.5 rounded-full overflow-hidden bg-white/[0.04]">
        {counts
          .filter((c) => c.value > 0)
          .map((c) => (
            <div
              key={c.label}
              className={cn("transition-all duration-300", c.color)}
              style={{ width: `${(c.value / total) * 100}%` }}
            />
          ))}
      </div>
      <div className="flex gap-4 mt-3">
        {counts.map((c) => (
          <div key={c.label} className="flex items-center gap-1.5">
            <span className={cn("w-2 h-2 rounded-full", c.color)} />
            <span className="text-[10px] font-mono text-neutral-600">
              {c.label} {c.value}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export function GovernanceAnalytics({
  daos,
  proposals,
  tallies,
}: AnalyticsProps) {
  const stats = useMemo(() => {
    const totalMembers = daos.reduce((acc, d) => acc + d.member_count, 0);

    const byStatus: Record<string, number> = {};
    for (const p of proposals) {
      const s = p.status.toLowerCase();
      byStatus[s] = (byStatus[s] || 0) + 1;
    }

    const active = byStatus["active"] || 0;
    const passed = byStatus["passed"] || 0;
    const rejected = byStatus["rejected"] || 0;
    const executed = byStatus["executed"] || 0;

    // Participation: average total_voters / total_members across proposals with tallies
    let avgParticipation = 0;
    const talliedProposals = Object.values(tallies).filter(
      (t) => t.total_voters > 0
    );
    if (talliedProposals.length > 0 && totalMembers > 0) {
      const totalParticipation = talliedProposals.reduce(
        (acc, t) => acc + t.total_voters,
        0
      );
      avgParticipation = Math.round(
        (totalParticipation / (talliedProposals.length * totalMembers)) * 100
      );
    }

    // Approval rate among concluded proposals
    const concluded = passed + rejected + executed;
    const approvalRate =
      concluded > 0 ? Math.round(((passed + executed) / concluded) * 100) : 0;

    // Consensus strength: average vote margin in concluded proposals
    let avgMargin = 0;
    const concludedTallies = proposals
      .filter((p) => {
        const s = p.status.toLowerCase();
        return s === "passed" || s === "rejected" || s === "executed";
      })
      .map((p) => tallies[p.id])
      .filter(Boolean);

    if (concludedTallies.length > 0) {
      const totalMargin = concludedTallies.reduce((acc, t) => {
        const total = t.votes_for + t.votes_against;
        if (total === 0) return acc;
        return acc + Math.abs(t.votes_for - t.votes_against) / total;
      }, 0);
      avgMargin = Math.round((totalMargin / concludedTallies.length) * 100);
    }

    return {
      totalDaos: daos.length,
      totalMembers,
      totalProposals: proposals.length,
      active,
      passed,
      rejected,
      executed,
      avgParticipation,
      approvalRate,
      avgMargin,
    };
  }, [daos, proposals, tallies]);

  // Recent proposals sorted by date
  const recent = useMemo(
    () =>
      [...proposals]
        .sort(
          (a, b) =>
            new Date(b.created_at).getTime() -
            new Date(a.created_at).getTime()
        )
        .slice(0, 5),
    [proposals]
  );

  // Top DAOs by member count
  const topDaos = useMemo(
    () =>
      [...daos]
        .sort((a, b) => b.member_count - a.member_count)
        .slice(0, 5),
    [daos]
  );

  const maxMembers = topDaos.length > 0 ? topDaos[0].member_count : 1;

  return (
    <section>
      {/* Stat Cards */}
      <div className="grid grid-cols-4 gap-px bg-white/[0.04] rounded-lg overflow-hidden mb-10">
        <div className="bg-[#0a0a0a]">
          <StatCard
            label="DAOs"
            value={stats.totalDaos}
            sub={`${stats.totalMembers} total members`}
          />
        </div>
        <div className="bg-[#0a0a0a]">
          <StatCard
            label="Proposals"
            value={stats.totalProposals}
            sub={`${stats.active} active`}
          />
        </div>
        <div className="bg-[#0a0a0a]">
          <StatCard
            label="Participation"
            value={stats.avgParticipation > 0 ? `${stats.avgParticipation}%` : "--"}
            sub="avg voter turnout"
          />
        </div>
        <div className="bg-[#0a0a0a]">
          <StatCard
            label="Approval"
            value={stats.approvalRate > 0 ? `${stats.approvalRate}%` : "--"}
            sub={`${stats.avgMargin}% avg margin`}
          />
        </div>
      </div>

      {/* Proposal Status Distribution */}
      {stats.totalProposals > 0 && (
        <div className="mb-10">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Proposal Status
          </h3>
          <StatusBar
            counts={[
              {
                label: "Active",
                value: stats.active,
                color: "bg-[#d4af37]",
              },
              {
                label: "Passed",
                value: stats.passed + stats.executed,
                color: "bg-emerald-600",
              },
              {
                label: "Rejected",
                value: stats.rejected,
                color: "bg-red-800",
              },
            ]}
          />
        </div>
      )}

      <div className="grid grid-cols-2 gap-8">
        {/* Top DAOs */}
        <div>
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Largest DAOs
          </h3>
          {topDaos.length === 0 ? (
            <p className="text-xs text-neutral-700 font-light">No DAOs yet.</p>
          ) : (
            <div className="space-y-3">
              {topDaos.map((d) => (
                <div key={d.id}>
                  <div className="flex justify-between mb-1">
                    <span className="text-xs font-light truncate mr-3">
                      {d.name}
                    </span>
                    <span className="text-[10px] font-mono text-neutral-600 shrink-0">
                      {d.member_count}
                    </span>
                  </div>
                  <div className="h-px bg-white/[0.04] relative">
                    <div
                      className="absolute inset-y-0 left-0 bg-[#d4af37]/30 transition-all duration-300"
                      style={{
                        width: `${(d.member_count / maxMembers) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Recent Proposals */}
        <div>
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Recent Proposals
          </h3>
          {recent.length === 0 ? (
            <p className="text-xs text-neutral-700 font-light">
              No proposals yet.
            </p>
          ) : (
            <div className="space-y-3">
              {recent.map((p) => {
                const tally = tallies[p.id];
                const votes = tally
                  ? tally.votes_for + tally.votes_against
                  : 0;
                return (
                  <div
                    key={p.id}
                    className="flex items-start justify-between"
                  >
                    <div className="min-w-0">
                      <p className="text-xs font-light truncate">{p.title}</p>
                      <p className="text-[10px] font-mono text-neutral-700">
                        {votes} vote{votes !== 1 ? "s" : ""}
                      </p>
                    </div>
                    <span
                      className={cn(
                        "text-[10px] font-mono uppercase tracking-wider shrink-0 ml-3",
                        p.status.toLowerCase() === "active"
                          ? "text-[#d4af37]"
                          : p.status.toLowerCase() === "passed" ||
                              p.status.toLowerCase() === "executed"
                            ? "text-emerald-600"
                            : "text-neutral-600"
                      )}
                    >
                      {p.status}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>

      {/* Vote Distribution Detail */}
      {Object.keys(tallies).length > 0 && (
        <div className="mt-10">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Vote Distribution
          </h3>
          <Card className="bg-transparent border-white/[0.04]">
            <CardContent className="p-4">
              <div className="grid grid-cols-5 gap-2 text-[10px] font-mono uppercase tracking-[0.1em] text-neutral-600 mb-3 border-b border-white/[0.04] pb-2">
                <span className="col-span-2">Proposal</span>
                <span className="text-right">For</span>
                <span className="text-right">Against</span>
                <span className="text-right">Margin</span>
              </div>
              {proposals
                .filter((p) => tallies[p.id])
                .map((p) => {
                  const t = tallies[p.id];
                  const total = t.votes_for + t.votes_against;
                  const margin =
                    total > 0
                      ? Math.round(
                          ((t.votes_for - t.votes_against) / total) * 100
                        )
                      : 0;
                  return (
                    <div
                      key={p.id}
                      className="grid grid-cols-5 gap-2 text-xs py-1.5"
                    >
                      <span className="col-span-2 font-light truncate">
                        {p.title}
                      </span>
                      <span className="text-right font-mono text-emerald-700">
                        {t.votes_for}
                      </span>
                      <span className="text-right font-mono text-red-800">
                        {t.votes_against}
                      </span>
                      <span
                        className={cn(
                          "text-right font-mono",
                          margin > 0
                            ? "text-[#d4af37]"
                            : margin < 0
                              ? "text-red-700"
                              : "text-neutral-600"
                        )}
                      >
                        {margin > 0 ? "+" : ""}
                        {margin}%
                      </span>
                    </div>
                  );
                })}
            </CardContent>
          </Card>
        </div>
      )}
    </section>
  );
}
