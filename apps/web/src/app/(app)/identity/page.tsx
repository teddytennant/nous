"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  identity,
  type IdentityResponse,
  type CredentialResponse,
  type ReputationResponse,
} from "@/lib/api";
import { PageHeader } from "@/components/page-header";
import { useToast } from "@/components/toast";
import { EmptyState, CredentialIllustration, IdentityKeyIllustration } from "@/components/empty-state";
import { DidAvatar, DidAvatarLarge } from "@/components/did-avatar";
import { Copy, Check, Key, ArrowRight, Pencil, Shield, Repeat, Star } from "lucide-react";

export default function IdentityPage() {
  const [currentIdentity, setCurrentIdentity] =
    useState<IdentityResponse | null>(null);
  const [credentials, setCredentials] = useState<CredentialResponse[]>([]);
  const [reputation, setReputation] = useState<ReputationResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [copied, setCopied] = useState(false);

  // Inline display name editing
  const [editingName, setEditingName] = useState(false);
  const [editNameValue, setEditNameValue] = useState("");
  const editInputRef = useRef<HTMLInputElement>(null);

  const { toast } = useToast();

  const fetchIdentity = useCallback(async (did: string) => {
    try {
      const [id, creds, rep] = await Promise.allSettled([
        identity.get(did),
        identity.listCredentials(did),
        identity.getReputation(did),
      ]);
      if (id.status === "fulfilled") setCurrentIdentity(id.value);
      if (creds.status === "fulfilled") setCredentials(creds.value);
      if (rep.status === "fulfilled") setReputation(rep.value);

      const failures = [id, creds, rep].filter(r => r.status === "rejected");
      if (failures.length > 0 && id.status === "rejected") {
        toast({ title: "Failed to load identity", variant: "error" });
      }
    } catch {
      toast({ title: "Failed to fetch identity", variant: "error" });
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    const storedDid = localStorage.getItem("nous_did");
    if (storedDid) {
      fetchIdentity(storedDid);
    } else {
      setLoading(false);
    }
  }, [fetchIdentity]);

  // Focus the edit input when entering edit mode
  useEffect(() => {
    if (editingName && editInputRef.current) {
      editInputRef.current.focus();
      editInputRef.current.select();
    }
  }, [editingName]);

  async function createIdentity() {
    setCreating(true);
    try {
      const id = await identity.create(displayName || undefined);
      localStorage.setItem("nous_did", id.did);
      if (displayName) {
        localStorage.setItem("nous_display_name", displayName);
      }
      setCurrentIdentity(id);
      setCredentials([]);
      setReputation(null);
      setDisplayName("");
      toast({ title: "Identity created", description: "Your DID has been generated", variant: "success" });
      fetchIdentity(id.did);
    } catch (e) {
      toast({
        title: "Failed to create identity",
        description: e instanceof Error ? e.message : undefined,
        variant: "error",
      });
    } finally {
      setCreating(false);
    }
  }

  function copyDid() {
    if (currentIdentity) {
      navigator.clipboard.writeText(currentIdentity.did);
      setCopied(true);
      toast({ title: "DID copied to clipboard", variant: "success" });
      setTimeout(() => setCopied(false), 2000);
    }
  }

  function startEditingName() {
    const current =
      currentIdentity?.display_name ||
      localStorage.getItem("nous_display_name") ||
      "";
    setEditNameValue(current);
    setEditingName(true);
  }

  function saveDisplayName() {
    const trimmed = editNameValue.trim();
    if (trimmed) {
      localStorage.setItem("nous_display_name", trimmed);
      if (currentIdentity) {
        setCurrentIdentity({ ...currentIdentity, display_name: trimmed });
      }
      toast({ title: "Display name updated", variant: "success" });
    } else {
      localStorage.removeItem("nous_display_name");
      if (currentIdentity) {
        setCurrentIdentity({ ...currentIdentity, display_name: null });
      }
    }
    setEditingName(false);
  }

  function cancelEditName() {
    setEditingName(false);
  }

  function truncateDid(did: string): string {
    if (did.length <= 32) return did;
    return `${did.slice(0, 20)}...${did.slice(-12)}`;
  }

  // Loading skeleton — matches the new profile card layout
  if (loading) {
    return (
      <div className="p-4 sm:p-8 max-w-4xl">
        <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

        {/* Profile card skeleton */}
        <section className="mb-16">
          <div className="bg-white/[0.01] border border-white/[0.06] p-8">
            <div className="flex items-start gap-6">
              <Skeleton className="w-24 h-24 rounded-[14px] shrink-0" />
              <div className="flex-1 min-w-0 pt-1">
                <Skeleton className="h-6 w-40 mb-2" />
                <Skeleton className="h-4 w-full max-w-xs mb-4" />
                <div className="flex gap-3">
                  <Skeleton className="h-8 w-20" />
                  <Skeleton className="h-8 w-20" />
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Stats skeleton */}
        <section className="mb-16">
          <div className="grid grid-cols-3 gap-px bg-white/[0.03]">
            {[0, 1, 2].map((i) => (
              <div key={i} className="bg-black p-6">
                <Skeleton className="h-3 w-16 mb-3" />
                <Skeleton className="h-7 w-12" />
              </div>
            ))}
          </div>
        </section>

        {/* Key pairs skeleton */}
        <section className="mb-16">
          <Skeleton className="h-4 w-24 mb-8" />
          <div className="space-y-px">
            {[0, 1].map((i) => (
              <div key={i} className="flex items-center justify-between py-4 px-5 bg-white/[0.01]">
                <div className="flex items-center gap-3">
                  <Skeleton className="w-8 h-8 rounded-md" />
                  <div>
                    <Skeleton className="h-4 w-20 mb-1" />
                    <Skeleton className="h-3 w-16" />
                  </div>
                </div>
                <Skeleton className="h-3 w-24" />
              </div>
            ))}
          </div>
        </section>

        {/* Credentials skeleton */}
        <section>
          <Skeleton className="h-4 w-48 mb-8" />
          <Skeleton className="h-32 w-full" />
        </section>
      </div>
    );
  }

  // No identity yet — show creation form
  if (!currentIdentity) {
    return (
      <div className="p-4 sm:p-8 max-w-4xl">
        <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

        <div className="flex flex-col items-center py-12">
          <div className="text-neutral-700 mb-8">
            <IdentityKeyIllustration />
          </div>
          <h3 className="text-lg font-light text-neutral-300 mb-2">
            Generate your identity
          </h3>
          <p className="text-xs text-neutral-600 font-light text-center max-w-sm leading-relaxed mb-8">
            Create a self-sovereign DID:key identifier. Your keys are generated locally and never leave your device.
          </p>

          <div className="w-full max-w-sm space-y-4">
            <input
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createIdentity()}
              placeholder="Display name (optional)"
              className="w-full bg-white/[0.02] border border-white/[0.06] focus:border-[#d4af37]/30 text-sm font-light px-4 py-3 rounded-sm outline-none placeholder:text-neutral-700 transition-colors duration-200"
            />
            <button
              onClick={createIdentity}
              disabled={creating}
              className="w-full flex items-center justify-center gap-2 bg-[#d4af37] text-black px-6 py-3 rounded-md text-sm font-medium hover:bg-[#c4a030] transition-colors duration-200 disabled:opacity-50"
            >
              {creating ? (
                "Generating..."
              ) : (
                <>
                  <Key className="w-4 h-4" />
                  Generate DID
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    );
  }

  const categories = reputation
    ? Object.entries(reputation.scores).filter(([, v]) => v !== 0)
    : [];

  const resolvedName =
    currentIdentity.display_name ||
    localStorage.getItem("nous_display_name");

  return (
    <div className="p-4 sm:p-8 max-w-4xl">
      <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

      <div className="space-y-16 stagger-in">
        {/* ── Profile Card ────────────────────���───────────────────── */}
        <section>
          <div className="bg-white/[0.01] border border-white/[0.06] overflow-hidden">
            {/* Decorative top bar — subtle gold gradient */}
            <div className="h-px bg-gradient-to-r from-transparent via-[#d4af37]/30 to-transparent" />

            <div className="p-6 sm:p-8">
              <div className="flex flex-col sm:flex-row items-center sm:items-start gap-6">
                {/* Avatar */}
                <div className="shrink-0">
                  <DidAvatarLarge did={currentIdentity.did} size={96} />
                </div>

                {/* Identity info */}
                <div className="flex-1 min-w-0 text-center sm:text-left">
                  {/* Display name (editable) */}
                  <div className="flex items-center justify-center sm:justify-start gap-2 mb-1">
                    {editingName ? (
                      <input
                        ref={editInputRef}
                        value={editNameValue}
                        onChange={(e) => setEditNameValue(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") saveDisplayName();
                          if (e.key === "Escape") cancelEditName();
                        }}
                        onBlur={saveDisplayName}
                        placeholder="Display name"
                        className="bg-white/[0.03] border border-[#d4af37]/30 text-lg font-light px-3 py-1 rounded-sm outline-none placeholder:text-neutral-700 w-full max-w-[240px] transition-colors duration-200"
                      />
                    ) : (
                      <>
                        <h2 className="text-xl font-light tracking-[-0.01em]">
                          {resolvedName || "Anonymous"}
                        </h2>
                        <button
                          onClick={startEditingName}
                          className="p-1 rounded hover:bg-white/[0.04] transition-colors duration-150"
                          aria-label="Edit display name"
                        >
                          <Pencil className="w-3.5 h-3.5 text-neutral-600 hover:text-[#d4af37] transition-colors duration-150" />
                        </button>
                      </>
                    )}
                  </div>

                  {/* DID (truncated with copy) */}
                  <button
                    onClick={copyDid}
                    className="group inline-flex items-center gap-2 mb-4"
                  >
                    <code className="text-xs font-mono text-neutral-600 group-hover:text-neutral-400 transition-colors duration-150">
                      {truncateDid(currentIdentity.did)}
                    </code>
                    {copied ? (
                      <Check className="w-3 h-3 text-emerald-500" />
                    ) : (
                      <Copy className="w-3 h-3 text-neutral-700 group-hover:text-[#d4af37] transition-colors duration-150" />
                    )}
                  </button>

                  {/* Action buttons */}
                  <div className="flex items-center justify-center sm:justify-start gap-3">
                    <button
                      onClick={copyDid}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-light border border-white/[0.06] rounded-md hover:border-[#d4af37]/20 hover:bg-[#d4af37]/[0.02] transition-all duration-200"
                    >
                      <Copy className="w-3 h-3" />
                      Share DID
                    </button>
                    <button
                      onClick={() => {
                        localStorage.removeItem("nous_did");
                        localStorage.removeItem("nous_display_name");
                        setCurrentIdentity(null);
                        setCredentials([]);
                        setReputation(null);
                        setLoading(false);
                      }}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-light text-neutral-500 border border-white/[0.06] rounded-md hover:border-red-400/20 hover:text-red-400 hover:bg-red-400/[0.02] transition-all duration-200"
                    >
                      <Repeat className="w-3 h-3" />
                      Switch Identity
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {/* Bottom decorative bar */}
            <div className="h-px bg-gradient-to-r from-transparent via-white/[0.04] to-transparent" />
          </div>
        </section>

        {/* ── Stats Overview ────────────────────���─────────────────── */}
        <section>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-white/[0.04] overflow-hidden">
            <div className="bg-black p-6 group hover:bg-white/[0.01] transition-colors duration-200">
              <div className="flex items-center gap-2 mb-3">
                <Star className="w-3 h-3 text-neutral-700 group-hover:text-[#d4af37] transition-colors duration-300" />
                <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600">
                  Reputation
                </p>
              </div>
              <p className="text-2xl font-extralight tabular-nums group-hover:text-[#d4af37] transition-colors duration-300">
                {reputation?.total_score ?? 0}
              </p>
              <p className="text-[10px] text-neutral-700 mt-1">
                {reputation?.event_count ?? 0} event{reputation?.event_count !== 1 ? "s" : ""}
              </p>
            </div>
            <div className="bg-black p-6 group hover:bg-white/[0.01] transition-colors duration-200">
              <div className="flex items-center gap-2 mb-3">
                <Shield className="w-3 h-3 text-neutral-700 group-hover:text-[#d4af37] transition-colors duration-300" />
                <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600">
                  Credentials
                </p>
              </div>
              <p className="text-2xl font-extralight tabular-nums group-hover:text-[#d4af37] transition-colors duration-300">
                {credentials.length}
              </p>
              <p className="text-[10px] text-neutral-700 mt-1">
                {credentials.filter(c => !c.expired).length} active
              </p>
            </div>
            <div className="bg-black p-6 group hover:bg-white/[0.01] transition-colors duration-200">
              <div className="flex items-center gap-2 mb-3">
                <Key className="w-3 h-3 text-neutral-700 group-hover:text-[#d4af37] transition-colors duration-300" />
                <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600">
                  Key Pairs
                </p>
              </div>
              <p className="text-2xl font-extralight tabular-nums group-hover:text-[#d4af37] transition-colors duration-300">
                2
              </p>
              <p className="text-[10px] text-neutral-700 mt-1">
                signing + exchange
              </p>
            </div>
          </div>
        </section>

        {/* ── Reputation Breakdown ────────────────────────────────── */}
        {reputation && categories.length > 0 && (
          <section>
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
              Reputation Breakdown
            </h2>
            <div className="space-y-4">
              {categories.map(([cat, score]) => {
                const maxScore = Math.max(...categories.map(([, s]) => s), 1);
                const pct = Math.round((score / maxScore) * 100);
                return (
                  <div key={cat} className="group">
                    <div className="flex items-center justify-between mb-2">
                      <p className="text-xs font-light text-neutral-400 capitalize">
                        {cat}
                      </p>
                      <p className="text-xs font-mono text-neutral-600 tabular-nums">
                        {score}
                      </p>
                    </div>
                    <div className="h-1 bg-white/[0.04] rounded-full overflow-hidden">
                      <div
                        className="h-full bg-[#d4af37]/60 rounded-full transition-all duration-700 ease-out group-hover:bg-[#d4af37] "
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        {/* ── Key Pairs ───────────────────────────────────────────── */}
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Key Pairs
          </h2>
          <div className="space-y-px">
            <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01] border border-white/[0.06] border-b-0 card-lift">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-md bg-[#d4af37]/[0.06] border border-[#d4af37]/10 flex items-center justify-center">
                  <Key className="w-3.5 h-3.5 text-[#d4af37]/60" />
                </div>
                <div>
                  <p className="text-sm font-light">Signing</p>
                  <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                    {currentIdentity.signing_key_type}
                  </p>
                </div>
              </div>
              <span className="text-xs font-mono text-neutral-600">
                {currentIdentity.did.slice(-12)}
              </span>
            </div>
            <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01] border border-white/[0.06] card-lift">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-md bg-white/[0.03] border border-white/[0.06] flex items-center justify-center">
                  <Key className="w-3.5 h-3.5 text-neutral-600" />
                </div>
                <div>
                  <p className="text-sm font-light">Exchange</p>
                  <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                    {currentIdentity.exchange_key_type}
                  </p>
                </div>
              </div>
              <span className="text-xs font-mono text-neutral-600">derived</span>
            </div>
          </div>
        </section>

        {/* ── DID Document ────────────────────────────────────────── */}
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Full Identifier
          </h2>
          <div className="bg-white/[0.01] border border-white/[0.06] p-5">
            <div className="flex items-start gap-3">
              <DidAvatar did={currentIdentity.did} size={32} className="shrink-0 mt-0.5" />
              <div className="flex-1 min-w-0">
                <p className="text-xs font-mono text-neutral-600 mb-1.5">DID:key</p>
                <p className="text-xs font-mono font-light break-all leading-relaxed text-neutral-400">
                  {currentIdentity.did}
                </p>
              </div>
            </div>
          </div>
        </section>

        {/* ── Verifiable Credentials ──────────────────────────────── */}
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Verifiable Credentials
          </h2>
          {credentials.length === 0 ? (
            <EmptyState
              icon={<CredentialIllustration />}
              title="No credentials yet"
              description="Verifiable credentials are issued by trusted parties to attest to claims about your identity. They'll appear here once issued."
              action={
                <button className="flex items-center gap-1.5 text-xs text-[#d4af37] font-medium hover:text-[#c4a030] transition-colors duration-200">
                  Learn about credentials
                  <ArrowRight className="w-3 h-3" />
                </button>
              }
            />
          ) : (
            <div className="space-y-px stagger-in">
              {credentials.map((cred) => (
                <Card
                  key={cred.id}
                  className="bg-white/[0.01] border-white/[0.06] rounded-none card-lift"
                >
                  <CardContent className="p-6">
                    <div className="flex items-start justify-between mb-3">
                      <h3 className="text-sm font-light">
                        {cred.credential_type.join(", ")}
                      </h3>
                      <span
                        className={`text-[10px] font-mono uppercase tracking-wider ${
                          cred.expired ? "text-red-400" : "text-[#d4af37]"
                        }`}
                      >
                        {cred.expired ? "expired" : "valid"}
                      </span>
                    </div>
                    <pre className="text-[10px] font-mono text-neutral-600 mb-3 overflow-x-auto">
                      {JSON.stringify(cred.claims, null, 2)}
                    </pre>
                    <div className="flex gap-6 text-[10px] font-mono text-neutral-700">
                      <span>
                        Issuer: {cred.issuer.slice(0, 16)}...
                        {cred.issuer.slice(-8)}
                      </span>
                      <span>
                        Issued: {new Date(cred.issuance_date).toLocaleDateString()}
                      </span>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
