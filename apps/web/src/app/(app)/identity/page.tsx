"use client";

import { useCallback, useEffect, useState } from "react";
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
import { Copy, Check, Key, ArrowRight } from "lucide-react";

export default function IdentityPage() {
  const [currentIdentity, setCurrentIdentity] =
    useState<IdentityResponse | null>(null);
  const [credentials, setCredentials] = useState<CredentialResponse[]>([]);
  const [reputation, setReputation] = useState<ReputationResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [creating, setCreating] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [copied, setCopied] = useState(false);
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

  // Try to load identity from localStorage on mount
  useEffect(() => {
    const storedDid = localStorage.getItem("nous_did");
    if (storedDid) {
      fetchIdentity(storedDid);
    } else {
      setLoading(false);
    }
  }, [fetchIdentity]);

  async function createIdentity() {
    setCreating(true);
    try {
      const id = await identity.create(displayName || undefined);
      localStorage.setItem("nous_did", id.did);
      setCurrentIdentity(id);
      setCredentials([]);
      setReputation(null);
      setDisplayName("");
      toast({ title: "Identity created", description: "Your DID has been generated", variant: "success" });
      // Fetch full details
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

  // Loading skeleton
  if (loading) {
    return (
      <div className="p-4 sm:p-8 max-w-4xl">
        <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

        <section className="mb-16">
          <Skeleton className="h-4 w-48 mb-8" />
          <div className="bg-white/[0.01] border border-white/[0.06] p-6">
            <Skeleton className="h-3 w-16 mb-3" />
            <Skeleton className="h-5 w-full max-w-md mb-2" />
            <Skeleton className="h-3 w-24 mt-3" />
            <div className="flex gap-3 mt-4">
              <Skeleton className="h-4 w-12" />
              <Skeleton className="h-4 w-12" />
            </div>
          </div>
        </section>

        <section className="mb-16">
          <Skeleton className="h-4 w-24 mb-8" />
          <div className="space-y-px">
            <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01]">
              <div>
                <Skeleton className="h-4 w-20 mb-1" />
                <Skeleton className="h-3 w-16" />
              </div>
              <Skeleton className="h-3 w-24" />
            </div>
            <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01]">
              <div>
                <Skeleton className="h-4 w-24 mb-1" />
                <Skeleton className="h-3 w-16" />
              </div>
              <Skeleton className="h-3 w-16" />
            </div>
          </div>
        </section>

        <section className="mb-16">
          <Skeleton className="h-4 w-28 mb-8" />
          <div className="flex items-baseline gap-6 mb-6">
            <Skeleton className="h-8 w-12" />
            <Skeleton className="h-3 w-20" />
          </div>
        </section>

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
              className="w-full bg-white/[0.02] border border-transparent text-sm font-light px-4 py-3 rounded-sm outline-none placeholder:text-neutral-700"
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

  return (
    <div className="p-4 sm:p-8 max-w-4xl">
      <PageHeader title="Identity" subtitle="Self-sovereign. Your keys, your identity." />

      <div className="space-y-16 stagger-in">
        {/* DID Section */}
        <section>
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Decentralized Identifier
          </h2>
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
            <CardContent className="p-6">
              <p className="text-xs font-mono text-neutral-600 mb-2">DID:key</p>
              <p className="text-sm font-mono font-light break-all leading-relaxed">
                {currentIdentity.did}
              </p>
              {currentIdentity.display_name && (
                <p className="text-xs text-neutral-500 mt-2">
                  {currentIdentity.display_name}
                </p>
              )}
              <div className="flex gap-3 mt-4">
                <button
                  onClick={copyDid}
                  className="flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150"
                >
                  {copied ? (
                    <Check className="w-3 h-3 text-emerald-500" />
                  ) : (
                    <Copy className="w-3 h-3" />
                  )}
                  {copied ? "Copied" : "Copy"}
                </button>
                <button
                  onClick={() => {
                    localStorage.removeItem("nous_did");
                    setCurrentIdentity(null);
                    setCredentials([]);
                    setReputation(null);
                    setLoading(false);
                  }}
                  className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-red-400 transition-colors duration-150"
                >
                  Switch
                </button>
              </div>
            </CardContent>
          </Card>
        </section>

        {/* Key Pairs Section */}
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

        {/* Reputation Section */}
        {reputation && (
          <section>
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
              Reputation
            </h2>
            <div className="flex items-baseline gap-6 mb-6">
              <span className="text-3xl font-extralight">
                {reputation.total_score}
              </span>
              <span className="text-xs font-mono text-neutral-600">
                {reputation.event_count} event
                {reputation.event_count !== 1 ? "s" : ""}
              </span>
            </div>
            {categories.length > 0 && (
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-white/[0.03]">
                {categories.map(([cat, score]) => (
                  <div key={cat} className="bg-black p-4">
                    <p className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">
                      {cat}
                    </p>
                    <p className="text-lg font-extralight">{score}</p>
                  </div>
                ))}
              </div>
            )}
          </section>
        )}

        {/* Verifiable Credentials Section */}
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
