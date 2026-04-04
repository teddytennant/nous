"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";
import {
  governance,
  delegation as delegationApi,
  identity,
  type DaoResponse,
  type DaoDetailResponse,
  type ProposalResponse,
  type VoteResultResponse,
  type DelegationResponse,
  type PowerEntry,
} from "@/lib/api";
import { GovernanceAnalytics } from "@/components/governance-analytics";
import { EmptyState, GovernanceIllustration, DelegationIllustration } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";
import { Avatar } from "@/components/avatar";
import { useToast } from "@/components/toast";
import { usePageShortcuts, useListNavigation } from "@/components/keyboard-shortcuts";
import { ProposalDetailSheet } from "@/components/proposal-detail-sheet";

type Tab = "analytics" | "proposals" | "daos" | "delegation";
type StatusFilter = "all" | "active" | "passed" | "rejected";
type SortKey = "newest" | "deadline" | "most-votes" | "title";

const STATUS_FILTERS: { key: StatusFilter; label: string }[] = [
  { key: "all", label: "All" },
  { key: "active", label: "Active" },
  { key: "passed", label: "Passed" },
  { key: "rejected", label: "Rejected" },
];

const SORT_OPTIONS: { key: SortKey; label: string }[] = [
  { key: "newest", label: "Newest" },
  { key: "deadline", label: "Deadline" },
  { key: "most-votes", label: "Most Votes" },
  { key: "title", label: "Title A–Z" },
];

export default function GovernancePage() {
  const [tab, setTab] = useState<Tab>("analytics");
  const [daos, setDaos] = useState<DaoResponse[]>([]);
  const [selectedDao, setSelectedDao] = useState<DaoDetailResponse | null>(
    null
  );
  const [proposals, setProposals] = useState<ProposalResponse[]>([]);
  const [tallies, setTallies] = useState<Record<string, VoteResultResponse>>(
    {}
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [sheetProposal, setSheetProposal] = useState<ProposalResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all");
  const [sortKey, setSortKey] = useState<SortKey>("newest");
  const { toast } = useToast();
  const [userDid, setUserDid] = useState<string | null>(null);

  // Modal state
  const [showDaoForm, setShowDaoForm] = useState(false);
  const [showProposalForm, setShowProposalForm] = useState(false);
  const [daoName, setDaoName] = useState("");
  const [daoDesc, setDaoDesc] = useState("");
  const [propTitle, setPropTitle] = useState("");
  const [propDesc, setPropDesc] = useState("");
  const [propDaoId, setPropDaoId] = useState("");
  const [submitting, setSubmitting] = useState(false);

  // Delegation state
  const [delegations, setDelegations] = useState<DelegationResponse[]>([]);
  const [showDelegateForm, setShowDelegateForm] = useState(false);
  const [delegateTo, setDelegateTo] = useState("");
  const [delegateScope, setDelegateScope] = useState("");
  const [powerMap, setPowerMap] = useState<PowerEntry[]>([]);

  usePageShortcuts({
    p: () => setShowProposalForm(true),
    d: () => setShowDaoForm(true),
  });

  // Filter + sort proposals
  const filteredProposals = (() => {
    let list = proposals;

    // Status filter
    if (statusFilter !== "all") {
      list = list.filter((p) => {
        const s = p.status.toLowerCase();
        if (statusFilter === "active") return s === "active";
        if (statusFilter === "passed") return s === "passed" || s === "executed";
        if (statusFilter === "rejected") return s === "rejected" || s === "cancelled";
        return true;
      });
    }

    // Sort
    list = [...list].sort((a, b) => {
      switch (sortKey) {
        case "newest":
          return new Date(b.voting_ends).getTime() - new Date(a.voting_ends).getTime();
        case "deadline":
          return new Date(a.voting_ends).getTime() - new Date(b.voting_ends).getTime();
        case "most-votes": {
          const totalA = (tallies[a.id]?.votes_for || 0) + (tallies[a.id]?.votes_against || 0);
          const totalB = (tallies[b.id]?.votes_for || 0) + (tallies[b.id]?.votes_against || 0);
          return totalB - totalA;
        }
        case "title":
          return a.title.localeCompare(b.title);
        default:
          return 0;
      }
    });

    return list;
  })();

  const { selectedIndex: proposalNavIndex, setSelectedIndex: setProposalNavIndex, containerRef: proposalsContainerRef } = useListNavigation({
    itemCount: filteredProposals.length,
    enabled: tab === "proposals" && !showProposalForm,
    onActivate: (index) => {
      const p = filteredProposals[index];
      if (p) {
        setSelectedId(p.id);
        setSheetProposal(p);
      }
    },
  });

  useEffect(() => {
    const stored = localStorage.getItem("nous_did");
    if (stored) setUserDid(stored);
  }, []);

  const loadDaos = useCallback(async () => {
    try {
      const data = await governance.listDaos();
      setDaos(data.daos);
    } catch {
      // Ignore
    }
  }, []);

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
    } catch (e) {
      toast({ title: "Failed to load proposals", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    loadDaos();
    loadProposals();
  }, [loadDaos, loadProposals]);

  async function ensureIdentity(): Promise<string> {
    if (userDid) return userDid;
    const id = await identity.create("Nous User");
    localStorage.setItem("nous_did", id.did);
    setUserDid(id.did);
    return id.did;
  }

  async function handleCreateDao() {
    if (!daoName.trim()) return;
    setSubmitting(true);
    try {
      const did = await ensureIdentity();
      await governance.createDao(did, daoName.trim(), daoDesc.trim());
      setDaoName("");
      setDaoDesc("");
      setShowDaoForm(false);
      await loadDaos();
      toast({ title: "DAO created", description: daoName.trim(), variant: "success" });
    } catch (e) {
      toast({ title: "Failed to create DAO", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setSubmitting(false);
    }
  }

  async function handleCreateProposal() {
    if (!propTitle.trim() || !propDaoId) return;
    setSubmitting(true);
    try {
      const did = await ensureIdentity();
      await governance.createProposal(propDaoId, {
        proposer_did: did,
        title: propTitle.trim(),
        description: propDesc.trim(),
      });
      setPropTitle("");
      setPropDesc("");
      setPropDaoId("");
      setShowProposalForm(false);
      await loadProposals();
      toast({ title: "Proposal submitted", description: propTitle.trim(), variant: "success" });
    } catch (e) {
      toast({ title: "Failed to create proposal", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setSubmitting(false);
    }
  }

  async function handleVote(proposalId: string, choice: "for" | "against") {
    try {
      const did = await ensureIdentity();
      await governance.vote(proposalId, {
        voter_did: did,
        choice,
        credits: 1,
      });
      // Refresh tally
      const tally = await governance.getTally(proposalId);
      setTallies((prev) => ({ ...prev, [proposalId]: tally }));
      toast({ title: `Voted ${choice}`, variant: "success" });
    } catch (e) {
      toast({ title: "Failed to cast vote", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  async function handleSelectDao(dao: DaoResponse) {
    try {
      const detail = await governance.getDao(dao.id);
      setSelectedDao(detail);
    } catch {
      setSelectedDao(null);
    }
  }

  const loadDelegations = useCallback(async () => {
    if (!userDid) return;
    try {
      const data = await delegationApi.list({ from_did: userDid });
      setDelegations(data.delegations);
    } catch {
      // Ignore
    }
  }, [userDid]);

  const loadPower = useCallback(
    async (daoId: string) => {
      try {
        const data = await delegationApi.power("dao", daoId);
        setPowerMap(data.power);
      } catch {
        // Ignore
      }
    },
    []
  );

  async function handleDelegate() {
    if (!delegateTo.trim() || !delegateScope) return;
    setSubmitting(true);
    try {
      const did = await ensureIdentity();
      await delegationApi.create({
        from_did: did,
        to_did: delegateTo.trim(),
        scope_type: "dao",
        scope_id: delegateScope,
      });
      setDelegateTo("");
      setShowDelegateForm(false);
      await loadDelegations();
      await loadPower(delegateScope);
      toast({ title: "Delegation created", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to delegate", description: e instanceof Error ? e.message : undefined, variant: "error" });
    } finally {
      setSubmitting(false);
    }
  }

  async function handleRevoke(delegationId: string) {
    try {
      const did = await ensureIdentity();
      await delegationApi.revoke(delegationId, did);
      await loadDelegations();
      toast({ title: "Delegation revoked", variant: "success" });
    } catch (e) {
      toast({ title: "Failed to revoke", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  function formatDeadline(votingEnds: string): string {
    const end = new Date(votingEnds);
    const now = new Date();
    const diff = end.getTime() - now.getTime();
    if (diff <= 0) return "Ended";
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    if (days > 0) return `${days}d`;
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
    <div className="p-4 sm:p-8 max-w-4xl">
      <PageHeader title="Governance" subtitle="Quadratic voting. Every voice weighted fairly." />

      {/* Tabs */}
      <div className="flex gap-6 mb-10 border-b border-white/[0.06] pb-3">
        {(["analytics", "proposals", "daos", "delegation"] as Tab[]).map((t) => (
          <button
            key={t}
            onClick={() => {
              setTab(t);
              setSelectedDao(null);
              setProposalNavIndex(-1);
            }}
            className={cn(
              "text-xs font-mono uppercase tracking-[0.2em] pb-1 transition-colors",
              tab === t
                ? "text-[#d4af37] border-b border-[#d4af37]"
                : "text-neutral-600 hover:text-neutral-400"
            )}
          >
            {t}
          </button>
        ))}
      </div>

      {/* ── Analytics Tab ── */}
      {tab === "analytics" && (
        <GovernanceAnalytics
          daos={daos}
          proposals={proposals}
          tallies={tallies}
        />
      )}

      {/* ── Proposals Tab ── */}
      {tab === "proposals" && (
        <section>
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              {filteredProposals.length}
              {statusFilter !== "all" ? ` ${statusFilter}` : ""}{" "}
              Proposal{filteredProposals.length !== 1 ? "s" : ""}
              {statusFilter !== "all" && proposals.length !== filteredProposals.length && (
                <span className="text-neutral-700 ml-1">of {proposals.length}</span>
              )}
            </h2>
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                setShowProposalForm(!showProposalForm);
                if (daos.length > 0 && !propDaoId) setPropDaoId(daos[0].id);
              }}
              className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
            >
              {showProposalForm ? "Cancel" : "New Proposal"}
            </Button>
          </div>

          {/* Filter + Sort controls */}
          {proposals.length > 0 && !showProposalForm && (
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-8">
              {/* Status filters */}
              <div className="flex items-center gap-1">
                {STATUS_FILTERS.map((f) => {
                  const count =
                    f.key === "all"
                      ? proposals.length
                      : proposals.filter((p) => {
                          const s = p.status.toLowerCase();
                          if (f.key === "active") return s === "active";
                          if (f.key === "passed") return s === "passed" || s === "executed";
                          if (f.key === "rejected") return s === "rejected" || s === "cancelled";
                          return false;
                        }).length;
                  return (
                    <button
                      key={f.key}
                      onClick={() => { setStatusFilter(f.key); setProposalNavIndex(-1); }}
                      className={cn(
                        "text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 rounded-sm transition-all duration-150",
                        statusFilter === f.key
                          ? "bg-[#d4af37]/10 text-[#d4af37] border border-[#d4af37]/20"
                          : "text-neutral-600 hover:text-neutral-400 border border-transparent hover:border-white/[0.06]"
                      )}
                    >
                      {f.label}
                      <span className="ml-1.5 text-neutral-700">{count}</span>
                    </button>
                  );
                })}
              </div>

              {/* Sort selector */}
              <div className="flex items-center gap-2">
                <span className="text-[10px] font-mono uppercase tracking-wider text-neutral-700">Sort</span>
                <div className="flex items-center gap-1">
                  {SORT_OPTIONS.map((opt) => (
                    <button
                      key={opt.key}
                      onClick={() => { setSortKey(opt.key); setProposalNavIndex(-1); }}
                      className={cn(
                        "text-[10px] font-mono tracking-wider px-2.5 py-1 rounded-sm transition-all duration-150",
                        sortKey === opt.key
                          ? "bg-white/[0.06] text-white"
                          : "text-neutral-600 hover:text-neutral-400"
                      )}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* New Proposal Form */}
          {showProposalForm && (
            <Card className="bg-white/[0.02] border-white/[0.06] mb-8">
              <CardContent className="p-6 space-y-4">
                <div>
                  <label className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 block mb-2">
                    DAO
                  </label>
                  <select
                    value={propDaoId}
                    onChange={(e) => setPropDaoId(e.target.value)}
                    className="w-full bg-black/40 border border-white/[0.08] rounded px-3 py-2 text-sm font-light focus:outline-none focus:border-[#d4af37]/40"
                  >
                    {daos.length === 0 && (
                      <option value="">No DAOs — create one first</option>
                    )}
                    {daos.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 block mb-2">
                    Title
                  </label>
                  <input
                    type="text"
                    value={propTitle}
                    onChange={(e) => setPropTitle(e.target.value)}
                    placeholder="Proposal title"
                    className="w-full bg-black/40 border border-white/[0.08] rounded px-3 py-2 text-sm font-light placeholder:text-neutral-700 focus:outline-none focus:border-[#d4af37]/40"
                  />
                </div>
                <div>
                  <label className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 block mb-2">
                    Description
                  </label>
                  <textarea
                    value={propDesc}
                    onChange={(e) => setPropDesc(e.target.value)}
                    placeholder="Describe the proposal"
                    rows={3}
                    className="w-full bg-black/40 border border-white/[0.08] rounded px-3 py-2 text-sm font-light placeholder:text-neutral-700 focus:outline-none focus:border-[#d4af37]/40 resize-none"
                  />
                </div>
                <Button
                  onClick={handleCreateProposal}
                  disabled={submitting || !propTitle.trim() || !propDaoId}
                  className="bg-[#d4af37] text-black hover:bg-[#c4a030] text-xs font-mono uppercase tracking-wider disabled:opacity-30"
                >
                  {submitting ? "Submitting..." : "Submit Proposal"}
                </Button>
              </CardContent>
            </Card>
          )}

          {loading ? (
            <div className="space-y-px">
              {Array.from({ length: 3 }).map((_, i) => (
                <div key={i} className="p-5">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <div className="flex items-center gap-3 mb-1">
                        <Skeleton className="h-2.5 w-24" />
                        <Skeleton className="h-2.5 w-14" />
                      </div>
                      <Skeleton className="h-3.5 w-56 mt-1" />
                    </div>
                    <Skeleton className="h-2.5 w-8" />
                  </div>
                  <div className="mt-3">
                    <div className="flex justify-between mb-1.5">
                      <Skeleton className="h-2.5 w-12" />
                      <Skeleton className="h-2.5 w-16" />
                    </div>
                    <Skeleton className="h-px w-full" />
                  </div>
                </div>
              ))}
            </div>
          ) : proposals.length === 0 ? (
            <EmptyState
              icon={<GovernanceIllustration />}
              title="No proposals yet"
              description="Submit the first proposal for your DAO. Quadratic voting ensures every voice is weighted fairly."
              action={
                <button
                  onClick={() => {
                    setShowProposalForm(true);
                    if (daos.length > 0 && !propDaoId) setPropDaoId(daos[0].id);
                  }}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  New Proposal
                </button>
              }
            />
          ) : filteredProposals.length === 0 ? (
            <div className="py-16 text-center">
              <p className="text-sm text-neutral-600 font-light mb-2">
                No {statusFilter} proposals
              </p>
              <button
                onClick={() => setStatusFilter("all")}
                className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
              >
                Show all proposals
              </button>
            </div>
          ) : (
            <div ref={proposalsContainerRef} className="space-y-px stagger-in">
              {filteredProposals.map((p, i) => {
                const tally = tallies[p.id];
                const votesFor = tally?.votes_for || 0;
                const votesAgainst = tally?.votes_against || 0;
                const total = votesFor + votesAgainst;
                const forPct = total > 0 ? (votesFor / total) * 100 : 0;
                const isNavSelected = i === proposalNavIndex;

                return (
                  <Card
                    key={p.id}
                    data-list-item
                    className={cn(
                      "relative bg-transparent border-0 rounded-none cursor-pointer transition-colors duration-150",
                      isNavSelected && "bg-[#d4af37]/[0.015]",
                      selectedId === p.id
                        ? "bg-white/[0.02]"
                        : !isNavSelected && "hover:bg-white/[0.01]"
                    )}
                    onClick={() => {
                      setSelectedId(p.id);
                      setSheetProposal(p);
                    }}
                  >
                    {isNavSelected && (
                      <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-[#d4af37] rounded-full" />
                    )}
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

                      {/* Voting progress bar */}
                      <div className="mt-4">
                        {/* Labels row */}
                        <div className="flex items-center justify-between mb-2">
                          <div className="flex items-center gap-3">
                            <span className="text-[10px] font-mono text-emerald-600">
                              {votesFor} for
                            </span>
                            <span className="text-[10px] font-mono text-red-500/70">
                              {votesAgainst} against
                            </span>
                            {(tally?.votes_abstain ?? 0) > 0 && (
                              <span className="text-[10px] font-mono text-neutral-600">
                                {tally.votes_abstain} abstain
                              </span>
                            )}
                          </div>
                          <span className="text-[10px] font-mono text-neutral-700">
                            {tally?.total_voters ?? 0} voter{(tally?.total_voters ?? 0) !== 1 ? "s" : ""}
                          </span>
                        </div>

                        {/* Bar */}
                        <div className="relative h-1.5 bg-white/[0.04] rounded-full overflow-hidden">
                          {total > 0 && (
                            <>
                              <div
                                className="absolute inset-y-0 left-0 bg-emerald-500/60 vote-bar-enter rounded-l-full"
                                style={{ width: `${forPct}%` }}
                              />
                              <div
                                className="absolute inset-y-0 bg-red-500/40 vote-bar-enter rounded-r-full"
                                style={{
                                  left: `${forPct}%`,
                                  width: `${(votesAgainst / total) * 100}%`,
                                  animationDelay: "80ms",
                                }}
                              />
                            </>
                          )}

                          {/* Quorum marker */}
                          <div
                            className="absolute top-0 bottom-0 w-px bg-white/20"
                            style={{ left: `${p.quorum * 100}%` }}
                            title={`Quorum: ${(p.quorum * 100).toFixed(0)}%`}
                          />
                        </div>

                        {/* Quorum + threshold labels */}
                        <div className="flex items-center justify-between mt-1.5">
                          <span className="text-[9px] font-mono text-neutral-700">
                            {p.threshold * 100}% to pass
                          </span>
                          {total > 0 && (
                            <span className={cn(
                              "text-[9px] font-mono",
                              forPct >= p.threshold * 100 ? "text-emerald-700" : "text-neutral-700"
                            )}>
                              {forPct.toFixed(0)}% approval
                            </span>
                          )}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </section>
      )}

      {/* ── DAOs Tab ── */}
      {tab === "daos" && (
        <section>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              {daos.length} DAO{daos.length !== 1 ? "s" : ""}
            </h2>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowDaoForm(!showDaoForm)}
              className="text-xs font-mono uppercase tracking-wider border-white/10 hover:border-[#d4af37] hover:text-[#d4af37]"
            >
              {showDaoForm ? "Cancel" : "Create DAO"}
            </Button>
          </div>

          {/* Create DAO Form */}
          {showDaoForm && (
            <Card className="bg-white/[0.02] border-white/[0.06] mb-8">
              <CardContent className="p-6 space-y-4">
                <div>
                  <label className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 block mb-2">
                    Name
                  </label>
                  <input
                    type="text"
                    value={daoName}
                    onChange={(e) => setDaoName(e.target.value)}
                    placeholder="DAO name"
                    className="w-full bg-black/40 border border-white/[0.08] rounded px-3 py-2 text-sm font-light placeholder:text-neutral-700 focus:outline-none focus:border-[#d4af37]/40"
                  />
                </div>
                <div>
                  <label className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 block mb-2">
                    Description
                  </label>
                  <textarea
                    value={daoDesc}
                    onChange={(e) => setDaoDesc(e.target.value)}
                    placeholder="What is this DAO about?"
                    rows={3}
                    className="w-full bg-black/40 border border-white/[0.08] rounded px-3 py-2 text-sm font-light placeholder:text-neutral-700 focus:outline-none focus:border-[#d4af37]/40 resize-none"
                  />
                </div>
                <Button
                  onClick={handleCreateDao}
                  disabled={submitting || !daoName.trim()}
                  className="bg-[#d4af37] text-black hover:bg-[#c4a030] text-xs font-mono uppercase tracking-wider disabled:opacity-30"
                >
                  {submitting ? "Creating..." : "Create DAO"}
                </Button>
              </CardContent>
            </Card>
          )}

          {daos.length === 0 ? (
            <EmptyState
              icon={<GovernanceIllustration />}
              title="No DAOs yet"
              description="Create a decentralized autonomous organization to coordinate governance with your community."
              action={
                <button
                  onClick={() => setShowDaoForm(true)}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  Create DAO
                </button>
              }
            />
          ) : (
            <div className="space-y-px stagger-in">
              {daos.map((d) => (
                <Card
                  key={d.id}
                  className={cn(
                    "bg-transparent border-0 rounded-none cursor-pointer transition-colors duration-150",
                    selectedDao?.id === d.id
                      ? "bg-white/[0.02]"
                      : "hover:bg-white/[0.01]"
                  )}
                  onClick={() =>
                    selectedDao?.id === d.id
                      ? setSelectedDao(null)
                      : handleSelectDao(d)
                  }
                >
                  <CardContent className="p-5">
                    <div className="flex items-start justify-between mb-1">
                      <div>
                        <h3 className="text-sm font-light">{d.name}</h3>
                        <p className="text-xs text-neutral-600 font-light mt-1">
                          {d.description}
                        </p>
                      </div>
                      <span className="text-[10px] font-mono text-neutral-700 shrink-0 ml-4">
                        {d.member_count} member
                        {d.member_count !== 1 ? "s" : ""}
                      </span>
                    </div>

                    {selectedDao?.id === d.id && (
                      <div className="mt-4 pt-4 border-t border-white/[0.04]">
                        <p className="text-[10px] font-mono text-neutral-700 mb-3">
                          ID: {d.id}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-700 mb-4">
                          Quorum: {(selectedDao.default_quorum * 100).toFixed(0)}
                          % | Threshold:{" "}
                          {(selectedDao.default_threshold * 100).toFixed(0)}%
                        </p>
                        <div className="space-y-1">
                          <p className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">
                            Members
                          </p>
                          {selectedDao.members.map((m) => (
                            <div
                              key={m.did}
                              className="flex items-center justify-between py-1"
                            >
                              <span className="text-xs font-mono text-neutral-500">
                                {m.did.length > 36
                                  ? `${m.did.slice(0, 30)}...`
                                  : m.did}
                              </span>
                              <span className="text-[10px] font-mono text-neutral-700">
                                {m.role.toLowerCase()}
                              </span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </section>
      )}

      {/* ── Delegation Tab ──────────────────────────────────────── */}
      {tab === "delegation" && (
        <section>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-sm font-mono uppercase tracking-[0.2em] text-neutral-400">
              Vote Delegation
            </h2>
            <Button
              onClick={() => {
                setShowDelegateForm((v) => !v);
                loadDelegations();
              }}
              className="text-xs"
            >
              {showDelegateForm ? "Cancel" : "Delegate"}
            </Button>
          </div>

          {showDelegateForm && (
            <Card className="bg-white/[0.02] border-white/[0.06] mb-8">
              <CardContent className="p-6 space-y-4">
                <p className="text-xs text-neutral-500 font-light mb-4">
                  Delegate your voting power to another member. They will vote
                  on your behalf with your credits.
                </p>
                <div>
                  <label className="block text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-1.5">
                    Delegate to (DID)
                  </label>
                  <input
                    value={delegateTo}
                    onChange={(e) => setDelegateTo(e.target.value)}
                    placeholder="did:key:z..."
                    className="w-full bg-white/[0.03] border border-white/[0.06] px-3 py-2 text-sm font-mono placeholder:text-neutral-700 focus:border-[#d4af37]/40 focus:outline-none transition-colors"
                  />
                </div>
                <div>
                  <label className="block text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-1.5">
                    DAO Scope
                  </label>
                  <select
                    value={delegateScope}
                    onChange={(e) => {
                      setDelegateScope(e.target.value);
                      if (e.target.value) loadPower(e.target.value);
                    }}
                    className="w-full bg-white/[0.03] border border-white/[0.06] px-3 py-2 text-sm font-mono focus:border-[#d4af37]/40 focus:outline-none transition-colors"
                  >
                    <option value="">Select DAO...</option>
                    {daos.map((d) => (
                      <option key={d.id} value={d.id}>
                        {d.name}
                      </option>
                    ))}
                  </select>
                </div>
                <Button
                  onClick={handleDelegate}
                  disabled={submitting || !delegateTo.trim() || !delegateScope}
                  className="text-xs"
                >
                  {submitting ? "Delegating..." : "Confirm Delegation"}
                </Button>
              </CardContent>
            </Card>
          )}

          {/* Active Delegations */}
          <div className="mb-10">
            <h3 className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-4">
              Your Delegations
            </h3>
            {delegations.length === 0 ? (
              <EmptyState
                icon={<DelegationIllustration />}
                title="No active delegations"
                description={userDid ? "Delegate your voting power to a trusted member of your DAO." : "Connect an identity in Settings to manage delegations."}
              />
            ) : (
              <div className="space-y-2 stagger-in">
                {delegations.map((d) => (
                  <Card
                    key={d.id}
                    className="bg-white/[0.02] border-white/[0.06] card-lift"
                  >
                    <CardContent className="p-4 flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <Avatar did={d.to_did} size="sm" />
                        <div className="space-y-1">
                        <p className="text-sm font-mono">
                          <span className="text-neutral-600">to </span>
                          <span className="text-[#d4af37]">
                            {d.to_did.length > 24
                              ? `${d.to_did.slice(0, 12)}...${d.to_did.slice(-8)}`
                              : d.to_did}
                          </span>
                        </p>
                        <p className="text-[10px] text-neutral-600 font-mono">
                          {d.scope_type}:{d.scope_id.length > 20 ? `${d.scope_id.slice(0, 20)}...` : d.scope_id}
                          {d.expires_at && (
                            <span className="ml-2">
                              expires {new Date(d.expires_at).toLocaleDateString()}
                            </span>
                          )}
                        </p>
                        </div>
                      </div>
                      <Button
                        onClick={() => handleRevoke(d.id)}
                        className="text-[10px] text-red-700 hover:text-red-500 bg-transparent hover:bg-white/[0.02]"
                      >
                        Revoke
                      </Button>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </div>

          {/* Effective Power — Bar Chart */}
          {powerMap.length > 0 && (
            <div>
              <h3 className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-4">
                Effective Voting Power
              </h3>
              {(() => {
                const maxPower = Math.max(...powerMap.map((p) => Math.max(p.base_credits, p.effective_credits)), 1);
                const sorted = [...powerMap].sort((a, b) => b.effective_credits - a.effective_credits);
                return (
                  <Card className="bg-white/[0.02] border-white/[0.06]">
                    <CardContent className="p-5">
                      <div className="space-y-4 stagger-in">
                        {sorted.map((p) => {
                          const basePct = (p.base_credits / maxPower) * 100;
                          const effectivePct = (p.effective_credits / maxPower) * 100;
                          const gained = p.effective_credits > p.base_credits;
                          const lost = p.effective_credits < p.base_credits;
                          const delta = p.effective_credits - p.base_credits;
                          return (
                            <div key={p.did}>
                              {/* Member row */}
                              <div className="flex items-center justify-between mb-2">
                                <span className="flex items-center gap-2 font-mono text-xs truncate">
                                  <Avatar did={p.did} size="xs" />
                                  {p.did.length > 20
                                    ? `${p.did.slice(0, 10)}...${p.did.slice(-6)}`
                                    : p.did}
                                </span>
                                <div className="flex items-baseline gap-3">
                                  <span className="text-[10px] font-mono text-neutral-600">
                                    base {p.base_credits}
                                  </span>
                                  <span
                                    className={cn(
                                      "text-xs font-mono font-medium",
                                      gained ? "text-[#d4af37]" : lost ? "text-neutral-600" : "text-neutral-300"
                                    )}
                                  >
                                    {p.effective_credits}
                                    {delta !== 0 && (
                                      <span className={cn("text-[10px] ml-1", gained ? "text-[#d4af37]/60" : "text-neutral-700")}>
                                        {delta > 0 ? `+${delta}` : delta}
                                      </span>
                                    )}
                                  </span>
                                </div>
                              </div>
                              {/* Bar */}
                              <div className="relative h-2 bg-white/[0.04] rounded-full overflow-hidden">
                                {/* Base power (background layer) */}
                                <div
                                  className="absolute inset-y-0 left-0 bg-white/[0.08] rounded-full power-bar-enter"
                                  style={{ width: `${basePct}%` }}
                                />
                                {/* Effective power (foreground layer) */}
                                <div
                                  className={cn(
                                    "absolute inset-y-0 left-0 rounded-full power-bar-enter",
                                    gained
                                      ? "bg-[#d4af37]/50"
                                      : lost
                                        ? "bg-neutral-600/50"
                                        : "bg-white/[0.15]"
                                  )}
                                  style={{
                                    width: `${effectivePct}%`,
                                    animationDelay: "100ms",
                                  }}
                                />
                                {/* Gold glow on gained power */}
                                {gained && (
                                  <div
                                    className="absolute inset-y-0 left-0 rounded-full bg-[#d4af37]/20 blur-[2px] power-bar-enter"
                                    style={{
                                      width: `${effectivePct}%`,
                                      animationDelay: "200ms",
                                    }}
                                  />
                                )}
                              </div>
                            </div>
                          );
                        })}
                      </div>
                      {/* Legend */}
                      <div className="flex items-center gap-6 mt-6 pt-4 border-t border-white/[0.04]">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-1.5 rounded-full bg-white/[0.08]" />
                          <span className="text-[10px] font-mono text-neutral-600">Base</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-1.5 rounded-full bg-[#d4af37]/50" />
                          <span className="text-[10px] font-mono text-neutral-600">Gained via delegation</span>
                        </div>
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-1.5 rounded-full bg-neutral-600/50" />
                          <span className="text-[10px] font-mono text-neutral-600">Delegated away</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })()}
            </div>
          )}
        </section>
      )}

      <ProposalDetailSheet
        proposal={sheetProposal}
        tally={sheetProposal ? tallies[sheetProposal.id] ?? null : null}
        dao={sheetProposal ? daos.find((d) => d.id === sheetProposal.dao_id) ?? null : null}
        open={sheetProposal !== null}
        onClose={() => setSheetProposal(null)}
        onVote={handleVote}
      />
    </div>
  );
}
