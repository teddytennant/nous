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

// ── SVG Charts ────────────────────────────────────────────────────────────

/**
 * Donut chart — shows aggregate vote distribution (for/against)
 * with total votes in the center.
 */
function DonutChart({
  votesFor,
  votesAgainst,
}: {
  votesFor: number;
  votesAgainst: number;
}) {
  const total = votesFor + votesAgainst;
  if (total === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-xs text-neutral-700 font-light">No votes yet</p>
      </div>
    );
  }

  const forPct = votesFor / total;
  const againstPct = votesAgainst / total;

  // SVG arc parameters
  const cx = 60;
  const cy = 60;
  const r = 46;
  const strokeWidth = 10;
  const circumference = 2 * Math.PI * r;

  // For arc: starts at top (12 o'clock), goes clockwise
  const forLen = forPct * circumference;
  const againstLen = againstPct * circumference;

  // Gap between segments (2px visual gap)
  const gap = total > 0 && votesFor > 0 && votesAgainst > 0 ? 4 : 0;
  const forDash = Math.max(0, forLen - gap / 2);
  const againstDash = Math.max(0, againstLen - gap / 2);

  return (
    <div className="flex items-center gap-6">
      <div className="relative shrink-0" style={{ width: 120, height: 120 }}>
        <svg
          width="120"
          height="120"
          viewBox="0 0 120 120"
          className="chart-donut-enter"
        >
          {/* Background track */}
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="rgba(255,255,255,0.04)"
            strokeWidth={strokeWidth}
          />
          {/* For votes (gold) */}
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="#d4af37"
            strokeWidth={strokeWidth}
            strokeDasharray={`${forDash} ${circumference - forDash}`}
            strokeDashoffset={circumference * 0.25}
            strokeLinecap="round"
            className="chart-arc-enter"
          />
          {/* Against votes (red) */}
          {votesAgainst > 0 && (
            <circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke="#991b1b"
              strokeWidth={strokeWidth}
              strokeDasharray={`${againstDash} ${circumference - againstDash}`}
              strokeDashoffset={circumference * 0.25 - forLen - gap / 2}
              strokeLinecap="round"
              className="chart-arc-enter"
              style={{ animationDelay: "100ms" }}
            />
          )}
        </svg>
        {/* Center label */}
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-lg font-extralight tracking-tight">{total}</span>
          <span className="text-[9px] font-mono text-neutral-600 uppercase tracking-wider">
            votes
          </span>
        </div>
      </div>

      {/* Legend */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-[#d4af37]" />
          <span className="text-xs font-light text-neutral-400">
            For{" "}
            <span className="font-mono text-[#d4af37]">
              {votesFor} ({Math.round(forPct * 100)}%)
            </span>
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-red-800" />
          <span className="text-xs font-light text-neutral-400">
            Against{" "}
            <span className="font-mono text-red-800">
              {votesAgainst} ({Math.round(againstPct * 100)}%)
            </span>
          </span>
        </div>
      </div>
    </div>
  );
}

/**
 * Ring gauge — circular progress indicator for a percentage value.
 * Used for participation rate and approval rate.
 */
function RingGauge({
  value,
  label,
  color = "#d4af37",
}: {
  value: number;
  label: string;
  color?: string;
}) {
  const cx = 40;
  const cy = 40;
  const r = 32;
  const strokeWidth = 5;
  const circumference = 2 * Math.PI * r;
  const filled = (Math.min(value, 100) / 100) * circumference;

  return (
    <div className="flex flex-col items-center gap-2">
      <div className="relative" style={{ width: 80, height: 80 }}>
        <svg
          width="80"
          height="80"
          viewBox="0 0 80 80"
          className="chart-donut-enter"
        >
          {/* Track */}
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="rgba(255,255,255,0.04)"
            strokeWidth={strokeWidth}
          />
          {/* Fill */}
          {value > 0 && (
            <circle
              cx={cx}
              cy={cy}
              r={r}
              fill="none"
              stroke={color}
              strokeWidth={strokeWidth}
              strokeDasharray={`${filled} ${circumference - filled}`}
              strokeDashoffset={circumference * 0.25}
              strokeLinecap="round"
              className="chart-arc-enter"
            />
          )}
        </svg>
        {/* Center value */}
        <div className="absolute inset-0 flex items-center justify-center">
          <span className="text-sm font-extralight tracking-tight">
            {value > 0 ? `${value}%` : "--"}
          </span>
        </div>
      </div>
      <span className="text-[9px] font-mono uppercase tracking-[0.15em] text-neutral-600">
        {label}
      </span>
    </div>
  );
}

/**
 * Horizontal stacked bar for per-proposal vote breakdown.
 * Shows for/against as proportional segments with vote counts.
 */
function VoteBar({
  title,
  votesFor,
  votesAgainst,
  status,
}: {
  title: string;
  votesFor: number;
  votesAgainst: number;
  status: string;
}) {
  const total = votesFor + votesAgainst;
  const forPct = total > 0 ? (votesFor / total) * 100 : 0;
  const againstPct = total > 0 ? (votesAgainst / total) * 100 : 0;
  const margin = total > 0 ? Math.round(((votesFor - votesAgainst) / total) * 100) : 0;

  const s = status.toLowerCase();

  return (
    <div className="py-3 first:pt-0 last:pb-0">
      <div className="flex items-baseline justify-between mb-2">
        <span className="text-xs font-light truncate mr-3 max-w-[60%]">{title}</span>
        <div className="flex items-center gap-3 shrink-0">
          <span
            className={cn(
              "text-[10px] font-mono",
              s === "active"
                ? "text-[#d4af37]"
                : s === "passed" || s === "executed"
                  ? "text-emerald-600"
                  : "text-neutral-600"
            )}
          >
            {status}
          </span>
          {total > 0 && (
            <span
              className={cn(
                "text-[10px] font-mono",
                margin > 0 ? "text-[#d4af37]" : margin < 0 ? "text-red-700" : "text-neutral-600"
              )}
            >
              {margin > 0 ? "+" : ""}{margin}%
            </span>
          )}
        </div>
      </div>

      {/* Stacked bar */}
      <div className="flex h-2 rounded-full overflow-hidden bg-white/[0.04]">
        {forPct > 0 && (
          <div
            className="bg-[#d4af37] transition-all duration-500 ease-out chart-bar-enter"
            style={{ width: `${forPct}%` }}
          />
        )}
        {againstPct > 0 && (
          <div
            className="bg-red-800 transition-all duration-500 ease-out chart-bar-enter"
            style={{ width: `${againstPct}%`, animationDelay: "60ms" }}
          />
        )}
      </div>

      {/* Vote counts under bar */}
      <div className="flex justify-between mt-1.5">
        <span className="text-[10px] font-mono text-neutral-600">
          {votesFor} for
        </span>
        <span className="text-[10px] font-mono text-neutral-600">
          {votesAgainst} against
        </span>
      </div>
    </div>
  );
}

// ── Main Component ───────────────────────────────────────────────────────

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

    // Aggregate vote totals
    let totalFor = 0;
    let totalAgainst = 0;
    for (const t of Object.values(tallies)) {
      totalFor += t.votes_for;
      totalAgainst += t.votes_against;
    }

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
      totalFor,
      totalAgainst,
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

  // Proposals with tallies for the vote chart
  const proposalsWithTallies = useMemo(
    () => proposals.filter((p) => tallies[p.id]),
    [proposals, tallies]
  );

  return (
    <section>
      {/* Stat Cards */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-white/[0.04] rounded-lg overflow-hidden mb-10">
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

      {/* Chart Row: Donut + Gauges */}
      {(stats.totalFor > 0 || stats.totalAgainst > 0 || stats.avgParticipation > 0) && (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-8 mb-10">
          {/* Donut — Aggregate vote distribution */}
          <div>
            <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-5">
              Vote Distribution
            </h3>
            <DonutChart
              votesFor={stats.totalFor}
              votesAgainst={stats.totalAgainst}
            />
          </div>

          {/* Ring gauges — Participation + Approval */}
          <div>
            <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-5">
              Health Metrics
            </h3>
            <div className="flex items-center justify-around py-2">
              <RingGauge
                value={stats.avgParticipation}
                label="Participation"
                color="#d4af37"
              />
              <RingGauge
                value={stats.approvalRate}
                label="Approval"
                color="#059669"
              />
              <RingGauge
                value={stats.avgMargin}
                label="Consensus"
                color="#737373"
              />
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-8">
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

      {/* Per-Proposal Vote Bars */}
      {proposalsWithTallies.length > 0 && (
        <div className="mt-10">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Votes by Proposal
          </h3>
          <Card className="bg-transparent border-white/[0.04]">
            <CardContent className="p-5 divide-y divide-white/[0.04]">
              {proposalsWithTallies.map((p) => {
                const t = tallies[p.id];
                return (
                  <VoteBar
                    key={p.id}
                    title={p.title}
                    votesFor={t.votes_for}
                    votesAgainst={t.votes_against}
                    status={p.status}
                  />
                );
              })}
            </CardContent>
          </Card>
        </div>
      )}
    </section>
  );
}
