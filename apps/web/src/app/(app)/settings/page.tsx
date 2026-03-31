"use client";

import { useEffect, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  node,
  identity,
  type IdentityResponse,
  type HealthResponse,
  type CredentialResponse,
  type ReputationResponse,
} from "@/lib/api";
import { useToast } from "@/components/toast";
import { PageHeader } from "@/components/page-header";

type Theme = "dark" | "light";

const NOTIFICATION_CATEGORIES = [
  { key: "social", label: "Social", description: "Posts, follows, reactions, and mentions" },
  { key: "governance", label: "Governance", description: "Proposals, votes, and DAO activity" },
  { key: "payments", label: "Payments", description: "Transfers, invoices, and escrow updates" },
  { key: "marketplace", label: "Marketplace", description: "Listings, orders, and offers" },
  { key: "messages", label: "Messages", description: "Direct messages and channel activity" },
  { key: "system", label: "System", description: "Connection status, updates, and errors" },
] as const;

type NotifPrefs = Record<string, boolean>;

function loadNotifPrefs(): NotifPrefs {
  if (typeof window === "undefined") return {};
  try {
    const raw = localStorage.getItem("nous_notif_prefs");
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

export default function SettingsPage() {
  const [loading, setLoading] = useState(true);
  const [online, setOnline] = useState(false);
  const [nodeInfo, setNodeInfo] = useState<HealthResponse | null>(null);
  const [did, setDid] = useState("");
  const [userIdentity, setUserIdentity] = useState<IdentityResponse | null>(null);
  const [displayName, setDisplayName] = useState("");
  const [theme, setTheme] = useState<Theme>("dark");
  const [apiUrl, setApiUrl] = useState("http://localhost:8080/api/v1");
  const [saved, setSaved] = useState(false);
  const [credentials, setCredentials] = useState<CredentialResponse[]>([]);
  const [reputation, setReputation] = useState<ReputationResponse | null>(null);
  const [notifPrefs, setNotifPrefs] = useState<NotifPrefs>({});
  const { toast } = useToast();

  useEffect(() => {
    const storedDid = localStorage.getItem("nous_did") || "";
    const storedName = localStorage.getItem("nous_display_name") || "";
    const storedTheme = (localStorage.getItem("nous_theme") as Theme) || "dark";
    const storedApi = localStorage.getItem("nous_api_url") || "http://localhost:8080/api/v1";
    const storedNotifPrefs = loadNotifPrefs();
    startTransition(() => {
      setDid(storedDid);
      setDisplayName(storedName);
      setTheme(storedTheme);
      setApiUrl(storedApi);
      setNotifPrefs(storedNotifPrefs);
    });

    const promises: Promise<void>[] = [];

    promises.push(
      node
        .health()
        .then((h) => { setOnline(true); setNodeInfo(h); })
        .catch(() => setOnline(false))
    );

    if (storedDid) {
      promises.push(identity.get(storedDid).then(setUserIdentity).catch(() => {}));
      promises.push(identity.listCredentials(storedDid).then(setCredentials).catch(() => {}));
      promises.push(identity.getReputation(storedDid).then(setReputation).catch(() => {}));
    }

    Promise.allSettled(promises).then(() => setLoading(false));
  }, []);

  const handleSave = () => {
    localStorage.setItem("nous_did", did);
    localStorage.setItem("nous_display_name", displayName);
    localStorage.setItem("nous_theme", theme);
    localStorage.setItem("nous_api_url", apiUrl);
    localStorage.setItem("nous_notif_prefs", JSON.stringify(notifPrefs));
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
    toast({ title: "Settings saved", variant: "success" });
  };

  const toggleNotifCategory = (key: string) => {
    setNotifPrefs((prev) => ({ ...prev, [key]: !(prev[key] ?? true) }));
  };

  const handleClearData = () => {
    localStorage.removeItem("nous_did");
    localStorage.removeItem("nous_display_name");
    localStorage.removeItem("nous_theme");
    localStorage.removeItem("nous_api_url");
    localStorage.removeItem("nous_notif_prefs");
    setDid("");
    setDisplayName("");
    setTheme("dark");
    setApiUrl("http://localhost:8080/api/v1");
    setUserIdentity(null);
    setCredentials([]);
    setReputation(null);
    setNotifPrefs({});
    setSaved(false);
    toast({ title: "Data cleared", description: "All local data has been removed" });
  };

  const exportDIDDocument = async () => {
    if (!did) return;
    try {
      const doc = await identity.getDocument(did);
      const blob = new Blob([JSON.stringify(doc.document, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `did-document-${did.slice(-8)}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      // silently fail
    }
  };

  if (loading) {
    return (
      <div className="p-4 sm:p-8 max-w-3xl">
        <PageHeader title="Settings" subtitle="Identity, credentials, and preferences" />

        {/* Identity skeleton */}
        <section className="mb-16">
          <Skeleton className="h-3 w-16 mb-8" />
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
            <CardContent className="p-6 space-y-6">
              <div>
                <Skeleton className="h-2.5 w-20 mb-2" />
                <Skeleton className="h-11 w-full" />
              </div>
              <div>
                <Skeleton className="h-2.5 w-24 mb-2" />
                <Skeleton className="h-11 w-full" />
              </div>
            </CardContent>
          </Card>
        </section>

        {/* Appearance skeleton */}
        <section className="mb-16">
          <Skeleton className="h-3 w-24 mb-8" />
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
            <CardContent className="p-6">
              <Skeleton className="h-2.5 w-12 mb-3" />
              <div className="flex gap-3">
                <Skeleton className="h-10 w-20" />
                <Skeleton className="h-10 w-20" />
              </div>
            </CardContent>
          </Card>
        </section>

        {/* Network skeleton */}
        <section className="mb-16">
          <Skeleton className="h-3 w-20 mb-8" />
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
            <CardContent className="p-6 space-y-6">
              <div>
                <Skeleton className="h-2.5 w-24 mb-2" />
                <Skeleton className="h-11 w-full" />
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 sm:gap-6">
                <div><Skeleton className="h-2.5 w-12 mb-1" /><Skeleton className="h-5 w-16" /></div>
                <div><Skeleton className="h-2.5 w-14 mb-1" /><Skeleton className="h-5 w-12" /></div>
                <div><Skeleton className="h-2.5 w-14 mb-1" /><Skeleton className="h-5 w-10" /></div>
              </div>
            </CardContent>
          </Card>
        </section>

        {/* Actions skeleton */}
        <section className="flex items-center gap-4">
          <Skeleton className="h-11 w-32" />
          <Skeleton className="h-11 w-36" />
        </section>
      </div>
    );
  }

  return (
    <div className="p-4 sm:p-8 max-w-3xl">
      <PageHeader title="Settings" subtitle="Identity, credentials, and preferences" status={online ? "online" : "offline"} />

      {/* Identity section */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">Identity</h2>
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
          <CardContent className="p-6 space-y-6">
            <div>
              <label className="block text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">Active DID</label>
              <input
                value={did}
                onChange={(e) => setDid(e.target.value)}
                placeholder="did:key:z..."
                className="w-full bg-white/[0.02] border border-transparent text-sm font-light font-mono px-4 py-3 outline-none placeholder:text-neutral-700 transition-colors duration-150"
              />
              {userIdentity && (
                <p className="text-[10px] font-mono text-emerald-700 mt-2">Identity verified on node</p>
              )}
            </div>
            <div>
              <label className="block text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">Display Name</label>
              <input
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                placeholder="Anonymous"
                className="w-full bg-white/[0.02] border border-transparent text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700 transition-colors duration-150"
              />
            </div>
            {userIdentity && (
              <div className="grid grid-cols-2 gap-6 pt-2">
                <div>
                  <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider mb-1">Signing Key</p>
                  <p className="text-sm font-light">{userIdentity.signing_key_type}</p>
                </div>
                <div>
                  <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider mb-1">Exchange Key</p>
                  <p className="text-sm font-light">{userIdentity.exchange_key_type}</p>
                </div>
              </div>
            )}
            {did && (
              <button
                onClick={exportDIDDocument}
                className="text-[10px] font-mono uppercase tracking-wider px-4 py-2 border border-white/10 text-neutral-600 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150"
              >
                Export DID Document
              </button>
            )}
          </CardContent>
        </Card>
      </section>

      {/* Reputation section */}
      {reputation && reputation.event_count > 0 && (
        <section className="mb-16">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">Reputation</h2>
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
            <CardContent className="p-6">
              <div className="flex items-baseline gap-4 mb-6">
                <p className="text-2xl font-extralight">{reputation.total_score}</p>
                <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider">
                  Total Score · {reputation.event_count} events
                </p>
              </div>
              {Object.keys(reputation.scores).length > 0 && (
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-px bg-white/[0.03]">
                  {Object.entries(reputation.scores).map(([category, score]) => (
                    <div key={category} className="bg-black p-4">
                      <p className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">
                        {category}
                      </p>
                      <p className="text-lg font-extralight">{score}</p>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </section>
      )}

      {/* Credentials section */}
      {credentials.length > 0 && (
        <section className="mb-16">
          <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
            Credentials
            <span className="ml-3 text-neutral-700">{credentials.length}</span>
          </h2>
          <div className="space-y-px">
            {credentials.map((cred) => (
              <div key={cred.id} className="py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors">
                <div className="flex items-baseline justify-between mb-1">
                  <p className="text-sm font-light">{cred.credential_type.join(", ")}</p>
                  <span className={`text-[10px] font-mono ${cred.expired ? "text-red-400" : "text-emerald-600"}`}>
                    {cred.expired ? "Expired" : "Valid"}
                  </span>
                </div>
                <p className="text-[10px] font-mono text-neutral-700">
                  Issued by {cred.issuer.length > 30 ? `${cred.issuer.slice(0, 16)}...${cred.issuer.slice(-6)}` : cred.issuer}
                  {" · "}
                  {new Date(cred.issuance_date).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}
                </p>
                {Object.keys(cred.claims).length > 0 && (
                  <div className="mt-2 grid grid-cols-2 gap-x-6 gap-y-1">
                    {Object.entries(cred.claims).map(([key, value]) => (
                      <div key={key} className="flex items-baseline gap-2">
                        <span className="text-[10px] font-mono text-neutral-700">{key}:</span>
                        <span className="text-[10px] font-light text-neutral-400">{String(value)}</span>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Appearance section */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">Appearance</h2>
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
          <CardContent className="p-6">
            <div>
              <label className="block text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-3">Theme</label>
              <div className="flex gap-3">
                {(["dark", "light"] as Theme[]).map((t) => (
                  <button
                    key={t}
                    onClick={() => setTheme(t)}
                    className={`text-xs font-mono uppercase tracking-wider px-5 py-2.5 border transition-all duration-150 ${
                      theme === t
                        ? "border-[#d4af37]/30 text-[#d4af37]"
                        : "border-white/10 text-neutral-600 hover:text-white hover:border-white/20"
                    }`}
                  >
                    {t}
                  </button>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Notifications section */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">Notifications</h2>
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
          <CardContent className="p-6">
            <p className="text-xs text-neutral-500 font-light mb-6">
              Choose which notification categories appear in your notification panel.
            </p>
            <div className="space-y-0">
              {NOTIFICATION_CATEGORIES.map((cat, i) => {
                const enabled = notifPrefs[cat.key] ?? true;
                return (
                  <div
                    key={cat.key}
                    className={`flex items-center justify-between py-4 ${
                      i > 0 ? "border-t border-white/[0.04]" : ""
                    }`}
                  >
                    <div>
                      <p className="text-sm font-light">{cat.label}</p>
                      <p className="text-[10px] text-neutral-600 font-light mt-0.5">
                        {cat.description}
                      </p>
                    </div>
                    <button
                      onClick={() => toggleNotifCategory(cat.key)}
                      className={`relative w-9 h-5 rounded-full transition-colors duration-200 ${
                        enabled
                          ? "bg-[#d4af37]/30"
                          : "bg-white/[0.06]"
                      }`}
                      role="switch"
                      aria-checked={enabled}
                      aria-label={`Toggle ${cat.label} notifications`}
                    >
                      <span
                        className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full transition-all duration-200 ${
                          enabled
                            ? "translate-x-4 bg-[#d4af37]"
                            : "translate-x-0 bg-neutral-500"
                        }`}
                      />
                    </button>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Network section */}
      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">Network</h2>
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
          <CardContent className="p-6 space-y-6">
            <div>
              <label className="block text-[10px] font-mono uppercase tracking-wider text-neutral-600 mb-2">API Endpoint</label>
              <input
                value={apiUrl}
                onChange={(e) => setApiUrl(e.target.value)}
                className="w-full bg-white/[0.02] border border-transparent text-sm font-light font-mono px-4 py-3 outline-none placeholder:text-neutral-700 transition-colors duration-150"
              />
            </div>
            {nodeInfo && (
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 sm:gap-6">
                <div>
                  <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider mb-1">Status</p>
                  <p className="text-sm font-light text-emerald-500">{nodeInfo.status}</p>
                </div>
                <div>
                  <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider mb-1">Version</p>
                  <p className="text-sm font-light">{nodeInfo.version}</p>
                </div>
                <div>
                  <p className="text-[10px] font-mono text-neutral-600 uppercase tracking-wider mb-1">Uptime</p>
                  <p className="text-sm font-light">{Math.floor(nodeInfo.uptime_ms / 1000)}s</p>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </section>

      {/* Actions */}
      <section className="flex items-center gap-4">
        <button
          onClick={handleSave}
          className="text-xs font-mono uppercase tracking-wider px-6 py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
        >
          {saved ? "Saved" : "Save Settings"}
        </button>
        <button
          onClick={handleClearData}
          className="text-xs font-mono uppercase tracking-wider px-6 py-3 border border-white/10 text-neutral-600 hover:text-red-400 hover:border-red-900/30 transition-all duration-150"
        >
          Clear Local Data
        </button>
      </section>
    </div>
  );
}
