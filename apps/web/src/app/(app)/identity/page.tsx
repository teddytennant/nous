"use client";

import { useCallback, useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import {
  identity,
  type IdentityResponse,
  type CredentialResponse,
  type ReputationResponse,
} from "@/lib/api";

export default function IdentityPage() {
  const [currentIdentity, setCurrentIdentity] =
    useState<IdentityResponse | null>(null);
  const [credentials, setCredentials] = useState<CredentialResponse[]>([]);
  const [reputation, setReputation] = useState<ReputationResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [displayName, setDisplayName] = useState("");
  const [copied, setCopied] = useState(false);

  const fetchIdentity = useCallback(async (did: string) => {
    try {
      const [id, creds, rep] = await Promise.all([
        identity.get(did),
        identity.listCredentials(did),
        identity.getReputation(did),
      ]);
      setCurrentIdentity(id);
      setCredentials(creds);
      setReputation(rep);
      setError(null);
    } catch {
      setError("Failed to fetch identity");
    }
  }, []);

  // Try to load identity from localStorage on mount
  useEffect(() => {
    const storedDid = localStorage.getItem("nous_did");
    if (storedDid) {
      fetchIdentity(storedDid);
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
      setError(null);
      // Fetch full details
      fetchIdentity(id.did);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create identity");
    } finally {
      setCreating(false);
    }
  }

  function copyDid() {
    if (currentIdentity) {
      navigator.clipboard.writeText(currentIdentity.did);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }

  // No identity yet — show creation form
  if (!currentIdentity) {
    return (
      <div className="p-8 max-w-4xl">
        <header className="mb-16">
          <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
            Identity
          </h1>
          <p className="text-sm text-neutral-500 font-light">
            Self-sovereign. Your keys, your identity.
          </p>
        </header>

        {error && (
          <p className="text-xs text-red-500 mb-6">{error}</p>
        )}

        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none max-w-md">
          <CardContent className="p-6">
            <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-6">
              Generate Identity
            </p>
            <input
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && createIdentity()}
              placeholder="Display name (optional)"
              className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700 mb-4"
            />
            <button
              onClick={createIdentity}
              disabled={creating}
              className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150 disabled:opacity-50"
            >
              {creating ? "Generating..." : "Generate DID"}
            </button>
          </CardContent>
        </Card>
      </div>
    );
  }

  const categories = reputation
    ? Object.entries(reputation.scores).filter(([, v]) => v !== 0)
    : [];

  return (
    <div className="p-8 max-w-4xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Identity
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          Self-sovereign. Your keys, your identity.
        </p>
      </header>

      {error && (
        <p className="text-xs text-red-500 mb-6">{error}</p>
      )}

      <section className="mb-16">
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
                className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150"
              >
                {copied ? "Copied" : "Copy"}
              </button>
              <button
                onClick={() => {
                  localStorage.removeItem("nous_did");
                  setCurrentIdentity(null);
                }}
                className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-red-400 transition-colors duration-150"
              >
                Switch
              </button>
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Key Pairs
        </h2>
        <div className="space-y-px">
          <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01]">
            <div>
              <p className="text-sm font-light">Signing</p>
              <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                {currentIdentity.signing_key_type}
              </p>
            </div>
            <span className="text-xs font-mono text-neutral-600">
              {currentIdentity.did.slice(-12)}
            </span>
          </div>
          <div className="flex items-center justify-between py-4 px-5 bg-white/[0.01]">
            <div>
              <p className="text-sm font-light">Exchange</p>
              <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                {currentIdentity.exchange_key_type}
              </p>
            </div>
            <span className="text-xs font-mono text-neutral-600">derived</span>
          </div>
        </div>
      </section>

      {reputation && (
        <section className="mb-16">
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
            <div className="grid grid-cols-3 gap-px bg-white/[0.03]">
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

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Verifiable Credentials
        </h2>
        {credentials.length === 0 ? (
          <p className="text-sm text-neutral-700 font-light py-8 text-center">
            No credentials issued yet
          </p>
        ) : (
          <div className="space-y-px">
            {credentials.map((cred) => (
              <Card
                key={cred.id}
                className="bg-white/[0.01] border-white/[0.06] rounded-none"
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
  );
}
