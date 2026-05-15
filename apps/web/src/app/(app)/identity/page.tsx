"use client";

import { useCallback, useEffect, useRef, useState } from "react";
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
import { Tooltip } from "@/components/ui/tooltip";
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
      toast({ title: "DID copied", variant: "success" });
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
    return `${did.slice(0, 20)}…${did.slice(-12)}`;
  }

  // ── Loading skeleton ─────────────────────────────────────────────────────
  if (loading) {
    return (
      <div className="p-4 sm:p-8 max-w-4xl">
        <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

        <section className="border-t border-rule pt-8 mb-16">
          <div className="flex items-start gap-6">
            <Skeleton className="w-24 h-24 shrink-0 rounded-none" />
            <div className="flex-1 min-w-0 pt-1">
              <Skeleton className="h-8 w-48 mb-3" />
              <Skeleton className="h-3 w-full max-w-sm mb-6" />
              <div className="flex gap-4">
                <Skeleton className="h-7 w-24" />
                <Skeleton className="h-7 w-28" />
              </div>
            </div>
          </div>
        </section>

        <section className="border-t border-rule">
          <div className="grid grid-cols-3 divide-x divide-rule">
            {[0, 1, 2].map((i) => (
              <div key={i} className="py-6 px-6">
                <Skeleton className="h-3 w-20 mb-4" />
                <Skeleton className="h-7 w-12" />
              </div>
            ))}
          </div>
        </section>
      </div>
    );
  }

  // ── Empty state — no DID yet ─────────────────────────────────────────────
  if (!currentIdentity) {
    return (
      <div className="p-4 sm:p-8 max-w-4xl">
        <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

        <section className="border-t border-rule pt-16 pb-16">
          <div className="grid grid-cols-1 lg:grid-cols-12 gap-12 lg:gap-16">
            <div className="lg:col-span-7">
              <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-6">
                Step one
              </p>
              <h2 className="font-display text-[2.5rem] sm:text-[3rem] leading-[1.05] tracking-[-0.02em] text-foreground mb-6">
                Generate your identity.
              </h2>
              <p className="font-mono text-[0.8125rem] text-stone tracking-[0.01em] leading-relaxed max-w-md mb-10">
                A self-sovereign DID:key identifier — generated locally, never leaves
                your device. The signing key proves authorship; the exchange key
                receives encrypted messages.
              </p>

              <div className="space-y-4 max-w-md">
                <input
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && createIdentity()}
                  placeholder="Display name (optional)"
                  className="w-full bg-transparent border-b border-rule focus:border-oxblood text-sm font-light px-0 py-2 outline-none placeholder:text-stone/60 transition-colors duration-[160ms]"
                />
                <button
                  onClick={createIdentity}
                  disabled={creating}
                  className="inline-flex items-center gap-2 bg-oxblood text-ivory px-6 h-10 text-sm font-medium hover:bg-oxblood-dim transition-colors duration-[160ms] disabled:opacity-40"
                >
                  {creating ? (
                    "Generating…"
                  ) : (
                    <>
                      <Key className="w-3.5 h-3.5" />
                      Generate DID
                    </>
                  )}
                </button>
              </div>
            </div>

            <aside className="lg:col-span-5 lg:pl-8 lg:border-l lg:border-rule">
              <div className="text-stone/50 mb-8">
                <IdentityKeyIllustration />
              </div>
              <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-3">
                What you get
              </p>
              <dl className="space-y-3 text-[13px]">
                <div className="flex items-baseline gap-3 border-t border-rule pt-3 first:border-t-0 first:pt-0">
                  <dt className="font-mono text-stone tabular-nums w-6 shrink-0">01</dt>
                  <dd className="text-foreground/80 font-light">A portable DID — yours, not a vendor&apos;s.</dd>
                </div>
                <div className="flex items-baseline gap-3 border-t border-rule pt-3">
                  <dt className="font-mono text-stone tabular-nums w-6 shrink-0">02</dt>
                  <dd className="text-foreground/80 font-light">An ed25519 signing key for authorship.</dd>
                </div>
                <div className="flex items-baseline gap-3 border-t border-rule pt-3">
                  <dt className="font-mono text-stone tabular-nums w-6 shrink-0">03</dt>
                  <dd className="text-foreground/80 font-light">An x25519 exchange key for encrypted messages.</dd>
                </div>
              </dl>
            </aside>
          </div>
        </section>
      </div>
    );
  }

  const categories = reputation
    ? Object.entries(reputation.scores).filter(([, v]) => v !== 0)
    : [];

  const resolvedName =
    currentIdentity.display_name ||
    (typeof window !== "undefined" ? localStorage.getItem("nous_display_name") : null);

  return (
    <div className="p-4 sm:p-8 max-w-4xl">
      <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

      {/* ── Profile ──────────────────────────────────────────────────────── */}
      <section className="border-t border-rule pt-8 mb-16">
        <div className="flex flex-col sm:flex-row items-start gap-6 sm:gap-8">
          <DidAvatarLarge did={currentIdentity.did} size={96} className="shrink-0" />

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-2">
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
                  className="bg-transparent border-b border-oxblood font-display text-[1.75rem] tracking-[-0.02em] px-0 py-0 outline-none placeholder:text-stone/60 w-full max-w-[280px]"
                />
              ) : (
                <>
                  <h2 className="font-display text-[1.75rem] tracking-[-0.02em] text-foreground">
                    {resolvedName || "Anonymous"}
                  </h2>
                  <button
                    onClick={startEditingName}
                    className="p-1 text-stone hover:text-oxblood transition-colors duration-[160ms]"
                    aria-label="Edit display name"
                  >
                    <Pencil className="w-3 h-3" />
                  </button>
                </>
              )}
            </div>

            <button
              onClick={copyDid}
              className="group inline-flex items-center gap-2 mb-6"
            >
              <Tooltip content={currentIdentity.did}>
                <code className="text-xs font-mono text-stone group-hover:text-foreground/80 transition-colors duration-[160ms]">
                  {truncateDid(currentIdentity.did)}
                </code>
              </Tooltip>
              {copied ? (
                <Check className="w-3 h-3 text-sage" />
              ) : (
                <Copy className="w-3 h-3 text-stone group-hover:text-oxblood transition-colors duration-[160ms]" />
              )}
            </button>

            <div className="flex items-center gap-6">
              <button
                onClick={copyDid}
                className="inline-flex items-center gap-1.5 text-xs font-light text-foreground border-b border-rule hover:border-oxblood pb-0.5 transition-colors duration-[160ms]"
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
                className="inline-flex items-center gap-1.5 text-xs font-light text-stone border-b border-rule hover:border-oxblood hover:text-oxblood pb-0.5 transition-colors duration-[160ms]"
              >
                <Repeat className="w-3 h-3" />
                Switch identity
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* ── Stats trio ───────────────────────────────────────────────────── */}
      <section className="border-t border-rule mb-16">
        <div className="grid grid-cols-1 sm:grid-cols-3 sm:divide-x divide-rule">
          <Stat
            icon={<Star className="w-3 h-3" />}
            label="Reputation"
            value={reputation?.total_score ?? 0}
            sub={`${reputation?.event_count ?? 0} event${(reputation?.event_count ?? 0) !== 1 ? "s" : ""}`}
          />
          <Stat
            icon={<Shield className="w-3 h-3" />}
            label="Credentials"
            value={credentials.length}
            sub={`${credentials.filter(c => !c.expired).length} active`}
          />
          <Stat
            icon={<Key className="w-3 h-3" />}
            label="Key pairs"
            value={2}
            sub="signing + exchange"
          />
        </div>
      </section>

      {/* ── Reputation breakdown ─────────────────────────────────────────── */}
      {reputation && categories.length > 0 && (
        <section className="border-t border-rule pt-8 mb-16">
          <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-8">
            Reputation breakdown
          </p>
          <div className="space-y-5">
            {categories.map(([cat, score]) => {
              const maxScore = Math.max(...categories.map(([, s]) => s), 1);
              const pct = Math.round((score / maxScore) * 100);
              return (
                <div key={cat}>
                  <div className="flex items-baseline justify-between mb-2">
                    <p className="text-sm font-light text-foreground capitalize">{cat}</p>
                    <p className="font-mono text-stone tabular-nums text-xs">{score}</p>
                  </div>
                  <div className="h-px bg-rule">
                    <div
                      className="h-px bg-oxblood transition-all duration-[240ms]"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </section>
      )}

      {/* ── Key pairs ────────────────────────────────────────────────────── */}
      <section className="border-t border-rule pt-8 mb-16">
        <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-8">
          Key pairs
        </p>
        <KeyRow
          label="Signing"
          type={currentIdentity.signing_key_type}
          value={currentIdentity.did.slice(-12)}
          accent
        />
        <KeyRow
          label="Exchange"
          type={currentIdentity.exchange_key_type}
          value="derived"
        />
      </section>

      {/* ── Full identifier ──────────────────────────────────────────────── */}
      <section className="border-t border-rule pt-8 mb-16">
        <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-6">
          Full identifier
        </p>
        <div className="flex items-start gap-4">
          <DidAvatar did={currentIdentity.did} size={32} className="shrink-0 mt-1" />
          <div className="flex-1 min-w-0">
            <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-2">
              DID:key
            </p>
            <p className="text-xs font-mono font-light break-all leading-relaxed text-foreground/80">
              {currentIdentity.did}
            </p>
          </div>
        </div>
      </section>

      {/* ── Verifiable credentials ───────────────────────────────────────── */}
      <section className="border-t border-rule pt-8">
        <p className="text-[10px] font-mono uppercase tracking-[0.25em] text-stone mb-8">
          Verifiable credentials
        </p>
        {credentials.length === 0 ? (
          <EmptyState
            icon={<CredentialIllustration />}
            title="No credentials yet"
            description="Verifiable credentials are issued by trusted parties to attest to claims about your identity. They appear here once issued."
            action={
              <button className="inline-flex items-center gap-1.5 text-xs text-oxblood font-medium hover:text-oxblood-dim transition-colors duration-[160ms]">
                Learn about credentials
                <ArrowRight className="w-3 h-3" />
              </button>
            }
          />
        ) : (
          <div>
            {credentials.map((cred, i) => (
              <article
                key={cred.id}
                className={`py-6 ${i > 0 ? "border-t border-rule" : ""}`}
              >
                <div className="flex items-start justify-between mb-3">
                  <h3 className="font-display text-lg tracking-[-0.01em] text-foreground">
                    {cred.credential_type.join(", ")}
                  </h3>
                  <span
                    className={`text-[10px] font-mono uppercase tracking-[0.25em] ${
                      cred.expired ? "text-stone" : "text-oxblood"
                    }`}
                  >
                    {cred.expired ? "expired" : "valid"}
                  </span>
                </div>
                <pre className="text-[11px] font-mono text-stone mb-4 overflow-x-auto leading-relaxed">
                  {JSON.stringify(cred.claims, null, 2)}
                </pre>
                <div className="flex flex-wrap gap-x-8 gap-y-1 text-[10px] font-mono uppercase tracking-[0.2em] text-stone">
                  <Tooltip content={cred.issuer}>
                    <span className="cursor-default hover:text-foreground/70 transition-colors duration-[160ms]">
                      Issuer · {cred.issuer.slice(0, 16)}…{cred.issuer.slice(-8)}
                    </span>
                  </Tooltip>
                  <Tooltip content={new Date(cred.issuance_date).toLocaleString()}>
                    <span className="cursor-default hover:text-foreground/70 transition-colors duration-[160ms]">
                      Issued · {new Date(cred.issuance_date).toLocaleDateString()}
                    </span>
                  </Tooltip>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

// ── Subcomponents ──────────────────────────────────────────────────────────

function Stat({
  icon,
  label,
  value,
  sub,
}: {
  icon: React.ReactNode;
  label: string;
  value: number;
  sub: string;
}) {
  return (
    <div className="py-6 px-6 first:pl-0 last:pr-0 sm:px-6">
      <div className="flex items-center gap-2 mb-4 text-stone">
        {icon}
        <p className="text-[10px] font-mono uppercase tracking-[0.25em]">{label}</p>
      </div>
      <p className="font-display text-[2rem] tracking-[-0.02em] tabular-nums text-foreground leading-none">
        {value}
      </p>
      <p className="text-[11px] text-stone mt-2 font-light">{sub}</p>
    </div>
  );
}

function KeyRow({
  label,
  type,
  value,
  accent = false,
}: {
  label: string;
  type: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div className="flex items-baseline justify-between py-4 border-t border-rule first:border-t-0">
      <div className="flex items-baseline gap-4">
        <span className={`font-mono text-[10px] uppercase tracking-[0.25em] w-2 ${accent ? "text-oxblood" : "text-stone"}`}>
          ▌
        </span>
        <div>
          <p className="text-sm font-light text-foreground">{label}</p>
          <p className="text-[10px] font-mono text-stone mt-1">{type}</p>
        </div>
      </div>
      <span className="text-xs font-mono text-stone tabular-nums">{value}</span>
    </div>
  );
}
