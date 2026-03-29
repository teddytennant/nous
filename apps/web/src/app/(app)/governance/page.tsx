"use client";

import { useState, useEffect, useCallback } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  governance,
  identity,
  type DaoResponse,
  type DaoDetailResponse,
  type ProposalResponse,
  type VoteResultResponse,
} from "@/lib/api";

type Tab = "proposals" | "daos";

export default function GovernancePage() {
  const [tab, setTab] = useState<Tab>("proposals");
  const [daos, setDaos] = useState<DaoResponse[]>([]);
  const [selectedDao, setSelectedDao] = useState<DaoDetailResponse | null>(
    null
  );
  const [proposals, setProposals] = useState<ProposalResponse[]>([]);
  const [tallies, setTallies] = useState<Record<string, VoteResultResponse>>(
    {}
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
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
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load proposals");
    } finally {
      setLoading(false);
    }
  }, []);

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
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create DAO");
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
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create proposal");
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
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to cast vote");
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
    <div className="p-8 max-w-4xl">
      <header className="mb-12">
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
          <button
            onClick={() => setError(null)}
            className="ml-3 text-neutral-600 hover:text-neutral-400"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-6 mb-10 border-b border-white/[0.06] pb-3">
        {(["proposals", "daos"] as Tab[]).map((t) => (
          <button
            key={t}
            onClick={() => {
              setTab(t);
              setSelectedDao(null);
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

      {/* ── Proposals Tab ── */}
      {tab === "proposals" && (
        <section>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              {proposals.length} Proposal{proposals.length !== 1 ? "s" : ""}
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
                            Proposed by{" "}
                            {p.proposer_did.length > 30
                              ? `${p.proposer_did.slice(0, 24)}...`
                              : p.proposer_did}
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
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleVote(p.id, "for");
                                }}
                                className="text-xs font-mono uppercase tracking-wider border-emerald-900 text-emerald-600 hover:bg-emerald-950"
                              >
                                Vote For
                              </Button>
                              <Button
                                variant="outline"
                                size="sm"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleVote(p.id, "against");
                                }}
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
            <p className="text-sm text-neutral-600 font-light">
              No DAOs yet. Create the first one.
            </p>
          ) : (
            <div className="space-y-px">
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
    </div>
  );
}
