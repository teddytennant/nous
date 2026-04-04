"use client";

import { useCallback, useEffect, useState } from "react";
import { Sheet, SheetHeader, SheetBody } from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import {
  governance,
  type ProposalResponse,
  type VoteResultResponse,
  type DaoResponse,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import { useToast } from "@/components/toast";
import { Copy, Check, ExternalLink } from "lucide-react";

// ── Helpers ──────────────────────────────────────────────────────────────

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function formatShortDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
  });
}

function timeRemaining(votingEnds: string): string {
  const end = new Date(votingEnds).getTime();
  const now = Date.now();
  const diff = end - now;
  if (diff <= 0) return "Voting ended";
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const mins = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  if (days > 0) return `${days}d ${hours}h remaining`;
  if (hours > 0) return `${hours}h ${mins}m remaining`;
  return `${mins}m remaining`;
}

function statusColor(status: string): string {
  const s = status.toLowerCase();
  if (s === "active") return "text-[#d4af37]";
  if (s === "passed" || s === "executed") return "text-emerald-500";
  if (s === "rejected" || s === "cancelled") return "text-red-500";
  return "text-neutral-500";
}

function statusBg(status: string): string {
  const s = status.toLowerCase();
  if (s === "active") return "bg-[#d4af37]/10 border-[#d4af37]/20";
  if (s === "passed" || s === "executed")
    return "bg-emerald-500/10 border-emerald-500/20";
  if (s === "rejected" || s === "cancelled")
    return "bg-red-500/10 border-red-500/20";
  return "bg-neutral-500/10 border-neutral-500/20";
}

// ── Timeline ─────────────────────────────────────────────────────────────

type TimelineStage = {
  label: string;
  date: string | null;
  reached: boolean;
  current: boolean;
  color: string;
};

function getTimelineStages(proposal: ProposalResponse): TimelineStage[] {
  const now = Date.now();
  const voteStart = new Date(proposal.voting_starts).getTime();
  const voteEnd = new Date(proposal.voting_ends).getTime();
  const s = proposal.status.toLowerCase();
  const isFinished =
    s === "passed" || s === "executed" || s === "rejected" || s === "cancelled";
  const isPassed = s === "passed" || s === "executed";

  return [
    {
      label: "Created",
      date: proposal.created_at,
      reached: true,
      current: now < voteStart && !isFinished,
      color: "bg-white",
    },
    {
      label: "Voting Open",
      date: proposal.voting_starts,
      reached: now >= voteStart || isFinished,
      current: now >= voteStart && now < voteEnd && !isFinished,
      color: "bg-[#d4af37]",
    },
    {
      label: "Voting Closed",
      date: proposal.voting_ends,
      reached: now >= voteEnd || isFinished,
      current: now >= voteEnd && !isFinished,
      color: "bg-white",
    },
    {
      label: isPassed
        ? "Passed"
        : isFinished
          ? s === "cancelled"
            ? "Cancelled"
            : "Rejected"
          : "Outcome",
      date: isFinished ? proposal.voting_ends : null,
      reached: isFinished,
      current: false,
      color: isPassed
        ? "bg-emerald-500"
        : isFinished
          ? "bg-red-500"
          : "bg-white",
    },
  ];
}

function DetailTimeline({ proposal }: { proposal: ProposalResponse }) {
  const stages = getTimelineStages(proposal);
  const reachedCount = stages.filter((s) => s.reached).length;
  const progress = ((reachedCount - 1) / (stages.length - 1)) * 100;

  return (
    <div className="timeline-enter">
      <div className="relative flex items-start justify-between">
        {/* Background line */}
        <div className="absolute top-[5px] left-[5px] right-[5px] h-px bg-white/[0.08]" />
        {/* Progress line */}
        <div
          className="absolute top-[5px] left-[5px] h-px timeline-line-grow"
          style={{
            width: `${progress}%`,
            background:
              "linear-gradient(90deg, rgba(255,255,255,0.25), rgba(212,175,55,0.4))",
          }}
        />

        {stages.map((stage, i) => (
          <div
            key={stage.label}
            className="relative flex flex-col items-center"
            style={{
              flex:
                i === 0 || i === stages.length - 1 ? "0 0 auto" : "1",
            }}
          >
            <div
              className={cn(
                "relative w-[11px] h-[11px] rounded-full border transition-all duration-300",
                stage.reached
                  ? `${stage.color} border-transparent`
                  : "bg-transparent border-white/20",
                stage.current &&
                  "ring-2 ring-[#d4af37]/30 ring-offset-1 ring-offset-black"
              )}
            >
              {stage.current && (
                <span className="absolute inset-0 rounded-full bg-[#d4af37]/40 animate-ping" />
              )}
            </div>
            <span
              className={cn(
                "text-[9px] font-mono uppercase tracking-wider mt-2 whitespace-nowrap",
                stage.current
                  ? "text-[#d4af37]"
                  : stage.reached
                    ? "text-neutral-400"
                    : "text-neutral-700"
              )}
            >
              {stage.label}
            </span>
            {stage.date && (
              <span className="text-[8px] font-mono text-neutral-700 mt-0.5">
                {formatShortDate(stage.date)}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Vote donut chart ─────────────────────────────────────────────────────

function VoteDonut({
  votesFor,
  votesAgainst,
  votesAbstain,
  size = 120,
}: {
  votesFor: number;
  votesAgainst: number;
  votesAbstain: number;
  size?: number;
}) {
  const total = votesFor + votesAgainst + votesAbstain;
  if (total === 0) {
    return (
      <div
        className="flex items-center justify-center"
        style={{ width: size, height: size }}
      >
        <div
          className="rounded-full border-2 border-white/[0.06] flex items-center justify-center"
          style={{ width: size - 8, height: size - 8 }}
        >
          <span className="text-[10px] font-mono text-neutral-700">
            No votes
          </span>
        </div>
      </div>
    );
  }

  const r = (size - 8) / 2;
  const cx = size / 2;
  const cy = size / 2;
  const circumference = 2 * Math.PI * r;

  const forPct = votesFor / total;
  const againstPct = votesAgainst / total;
  const abstainPct = votesAbstain / total;

  const forLen = forPct * circumference;
  const againstLen = againstPct * circumference;
  const abstainLen = abstainPct * circumference;

  // Offset so "for" starts at top (12 o'clock)
  const forOffset = 0;
  const againstOffset = forLen;
  const abstainOffset = forLen + againstLen;

  return (
    <div
      className="relative flex items-center justify-center"
      style={{ width: size, height: size }}
    >
      <svg
        width={size}
        height={size}
        className="chart-donut-enter"
        style={{ transform: "rotate(-90deg)" }}
      >
        {/* Background circle */}
        <circle
          cx={cx}
          cy={cy}
          r={r}
          fill="none"
          stroke="rgba(255,255,255,0.04)"
          strokeWidth={6}
        />
        {/* For arc */}
        {forPct > 0 && (
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="#10b981"
            strokeWidth={6}
            strokeDasharray={`${forLen} ${circumference - forLen}`}
            strokeDashoffset={-forOffset}
            strokeLinecap="round"
            className="chart-arc-enter"
          />
        )}
        {/* Against arc */}
        {againstPct > 0 && (
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="#ef4444"
            strokeWidth={6}
            strokeDasharray={`${againstLen} ${circumference - againstLen}`}
            strokeDashoffset={-againstOffset}
            strokeLinecap="round"
            className="chart-arc-enter"
            style={{ animationDelay: "60ms" }}
          />
        )}
        {/* Abstain arc */}
        {abstainPct > 0 && (
          <circle
            cx={cx}
            cy={cy}
            r={r}
            fill="none"
            stroke="#525252"
            strokeWidth={6}
            strokeDasharray={`${abstainLen} ${circumference - abstainLen}`}
            strokeDashoffset={-abstainOffset}
            strokeLinecap="round"
            className="chart-arc-enter"
            style={{ animationDelay: "120ms" }}
          />
        )}
      </svg>
      {/* Center text */}
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-lg font-extralight tracking-tight tabular-nums">
          {total}
        </span>
        <span className="text-[9px] font-mono text-neutral-600 uppercase tracking-wider">
          {total === 1 ? "vote" : "votes"}
        </span>
      </div>
    </div>
  );
}

// ── Quorum meter ─────────────────────────────────────────────────────────

function QuorumMeter({
  totalVoters,
  quorum,
  threshold,
  forPct,
}: {
  totalVoters: number;
  quorum: number;
  threshold: number;
  forPct: number;
}) {
  // For this visual, quorum is a percentage that needs to be met
  // We'll show two horizontal meters: participation and approval
  return (
    <div className="space-y-4">
      {/* Approval meter */}
      <div>
        <div className="flex items-center justify-between mb-1.5">
          <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
            Approval
          </span>
          <span
            className={cn(
              "text-[10px] font-mono tabular-nums",
              forPct >= threshold * 100
                ? "text-emerald-500"
                : "text-neutral-500"
            )}
          >
            {forPct.toFixed(1)}% / {(threshold * 100).toFixed(0)}%
          </span>
        </div>
        <div className="relative h-1.5 bg-white/[0.04] rounded-full overflow-hidden">
          <div
            className={cn(
              "absolute inset-y-0 left-0 rounded-full vote-bar-enter",
              forPct >= threshold * 100
                ? "bg-emerald-500/60"
                : "bg-neutral-500/40"
            )}
            style={{ width: `${Math.min(forPct, 100)}%` }}
          />
          {/* Threshold marker */}
          <div
            className="absolute top-0 bottom-0 w-px bg-[#d4af37]/60"
            style={{ left: `${threshold * 100}%` }}
          />
        </div>
      </div>

      {/* Participation label */}
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
          Participation
        </span>
        <span className="text-[10px] font-mono text-neutral-500 tabular-nums">
          {totalVoters} voter{totalVoters !== 1 ? "s" : ""} &middot; quorum{" "}
          {(quorum * 100).toFixed(0)}%
        </span>
      </div>
    </div>
  );
}

// ── Main Component ───────────────────────────────────────────────────────

interface ProposalDetailSheetProps {
  proposal: ProposalResponse | null;
  tally: VoteResultResponse | null;
  dao: DaoResponse | null;
  open: boolean;
  onClose: () => void;
  onVote: (proposalId: string, choice: "for" | "against") => Promise<void>;
}

export function ProposalDetailSheet({
  proposal,
  tally,
  dao,
  open,
  onClose,
  onVote,
}: ProposalDetailSheetProps) {
  const { toast } = useToast();
  const [voting, setVoting] = useState<"for" | "against" | null>(null);
  const [copied, setCopied] = useState(false);

  // Reset voting state when proposal changes
  useEffect(() => {
    setVoting(null);
    setCopied(false);
  }, [proposal?.id]);

  const handleVote = useCallback(
    async (choice: "for" | "against") => {
      if (!proposal) return;
      setVoting(choice);
      try {
        await onVote(proposal.id, choice);
      } finally {
        setVoting(null);
      }
    },
    [proposal, onVote]
  );

  const handleCopyId = useCallback(() => {
    if (!proposal) return;
    navigator.clipboard.writeText(proposal.id);
    setCopied(true);
    toast({ title: "Proposal ID copied", variant: "success" });
    setTimeout(() => setCopied(false), 2000);
  }, [proposal, toast]);

  if (!proposal) return null;

  const votesFor = tally?.votes_for ?? 0;
  const votesAgainst = tally?.votes_against ?? 0;
  const votesAbstain = tally?.votes_abstain ?? 0;
  const totalVotes = votesFor + votesAgainst;
  const forPct = totalVotes > 0 ? (votesFor / totalVotes) * 100 : 0;
  const isActive = proposal.status.toLowerCase() === "active";

  return (
    <Sheet open={open} onClose={onClose}>
      <SheetHeader onClose={onClose}>
        <div className="flex items-center gap-3">
          <span
            className={cn(
              "inline-flex items-center px-2 py-0.5 text-[10px] font-mono uppercase tracking-wider rounded-sm border",
              statusBg(proposal.status),
              statusColor(proposal.status)
            )}
          >
            {proposal.status}
          </span>
          {isActive && (
            <span className="text-[10px] font-mono text-neutral-600">
              {timeRemaining(proposal.voting_ends)}
            </span>
          )}
        </div>
      </SheetHeader>

      <SheetBody className="sheet-content-stagger">
        {/* Title */}
        <div className="mb-8">
          <h2 className="text-xl font-light tracking-tight mb-3">
            {proposal.title}
          </h2>
          <div className="flex items-center gap-3">
            <button
              onClick={handleCopyId}
              className="flex items-center gap-1.5 text-[10px] font-mono text-neutral-700 hover:text-neutral-400 transition-colors duration-150"
            >
              {copied ? (
                <Check size={10} className="text-[#d4af37]" />
              ) : (
                <Copy size={10} />
              )}
              {proposal.id.slice(0, 16)}...
            </button>
            {dao && (
              <span className="text-[10px] font-mono text-neutral-700">
                in {dao.name}
              </span>
            )}
          </div>
        </div>

        {/* Description */}
        <div className="mb-8">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-3">
            Description
          </h3>
          <p className="text-sm text-neutral-400 font-light leading-relaxed whitespace-pre-wrap">
            {proposal.description || "No description provided."}
          </p>
        </div>

        {/* Timeline */}
        <div className="mb-8">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Lifecycle
          </h3>
          <DetailTimeline proposal={proposal} />
        </div>

        {/* Vote breakdown — donut + legend */}
        <div className="mb-8">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Vote Breakdown
          </h3>
          <div className="flex items-center gap-8">
            <VoteDonut
              votesFor={votesFor}
              votesAgainst={votesAgainst}
              votesAbstain={votesAbstain}
            />
            <div className="space-y-3 flex-1">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="w-2 h-2 rounded-full bg-emerald-500" />
                  <span className="text-xs font-light text-neutral-300">
                    For
                  </span>
                </div>
                <span className="text-xs font-mono tabular-nums text-neutral-400">
                  {votesFor}
                  {totalVotes > 0 && (
                    <span className="text-neutral-600 ml-1.5">
                      {((votesFor / totalVotes) * 100).toFixed(0)}%
                    </span>
                  )}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="w-2 h-2 rounded-full bg-red-500" />
                  <span className="text-xs font-light text-neutral-300">
                    Against
                  </span>
                </div>
                <span className="text-xs font-mono tabular-nums text-neutral-400">
                  {votesAgainst}
                  {totalVotes > 0 && (
                    <span className="text-neutral-600 ml-1.5">
                      {((votesAgainst / totalVotes) * 100).toFixed(0)}%
                    </span>
                  )}
                </span>
              </div>
              {votesAbstain > 0 && (
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-neutral-600" />
                    <span className="text-xs font-light text-neutral-300">
                      Abstain
                    </span>
                  </div>
                  <span className="text-xs font-mono tabular-nums text-neutral-400">
                    {votesAbstain}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Quorum + Threshold meters */}
        <div className="mb-8">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Thresholds
          </h3>
          <QuorumMeter
            totalVoters={tally?.total_voters ?? 0}
            quorum={proposal.quorum}
            threshold={proposal.threshold}
            forPct={forPct}
          />
        </div>

        {/* Metadata */}
        <div className="mb-8">
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-4">
            Details
          </h3>
          <div className="space-y-3">
            <div className="flex items-center justify-between py-2 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Proposer
              </span>
              <span className="text-xs font-mono text-neutral-400 truncate max-w-[240px]">
                {proposal.proposer_did.length > 32
                  ? `${proposal.proposer_did.slice(0, 20)}...${proposal.proposer_did.slice(-8)}`
                  : proposal.proposer_did}
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Created
              </span>
              <span className="text-xs font-light text-neutral-400">
                {formatDate(proposal.created_at)}
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Voting Opens
              </span>
              <span className="text-xs font-light text-neutral-400">
                {formatDate(proposal.voting_starts)}
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Voting Closes
              </span>
              <span className="text-xs font-light text-neutral-400">
                {formatDate(proposal.voting_ends)}
              </span>
            </div>
            <div className="flex items-center justify-between py-2 border-b border-white/[0.04]">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Quorum
              </span>
              <span className="text-xs font-mono text-neutral-400 tabular-nums">
                {(proposal.quorum * 100).toFixed(0)}%
              </span>
            </div>
            <div className="flex items-center justify-between py-2">
              <span className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                Threshold
              </span>
              <span className="text-xs font-mono text-neutral-400 tabular-nums">
                {(proposal.threshold * 100).toFixed(0)}%
              </span>
            </div>
          </div>
        </div>

        {/* Vote Actions */}
        {isActive && (
          <div className="sticky bottom-0 bg-black/90 backdrop-blur-xl -mx-6 px-6 py-4 border-t border-white/[0.06]">
            <div className="flex gap-3">
              <Button
                variant="outline"
                className={cn(
                  "flex-1 text-xs font-mono uppercase tracking-wider border-emerald-900 text-emerald-500 hover:bg-emerald-950 hover:border-emerald-800",
                  voting === "for" && "opacity-60 pointer-events-none"
                )}
                onClick={() => handleVote("for")}
                disabled={voting !== null}
              >
                {voting === "for" ? "Voting..." : "Vote For"}
              </Button>
              <Button
                variant="outline"
                className={cn(
                  "flex-1 text-xs font-mono uppercase tracking-wider border-red-900 text-red-500 hover:bg-red-950 hover:border-red-800",
                  voting === "against" && "opacity-60 pointer-events-none"
                )}
                onClick={() => handleVote("against")}
                disabled={voting !== null}
              >
                {voting === "against" ? "Voting..." : "Vote Against"}
              </Button>
            </div>
          </div>
        )}

        {/* Outcome banner for finished proposals */}
        {!isActive && tally && (
          <div
            className={cn(
              "rounded-md border p-4 text-center",
              tally.passed
                ? "bg-emerald-500/5 border-emerald-500/20"
                : "bg-red-500/5 border-red-500/20"
            )}
          >
            <span
              className={cn(
                "text-sm font-medium",
                tally.passed ? "text-emerald-500" : "text-red-500"
              )}
            >
              {tally.passed ? "Proposal Passed" : "Proposal Did Not Pass"}
            </span>
            <p className="text-[10px] font-mono text-neutral-600 mt-1">
              {votesFor} for &middot; {votesAgainst} against &middot;{" "}
              {tally.total_voters} total voters
            </p>
          </div>
        )}
      </SheetBody>
    </Sheet>
  );
}
