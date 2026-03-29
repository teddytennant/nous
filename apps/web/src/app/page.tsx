import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

const features = [
  {
    name: "Identity",
    description: "Self-sovereign. DID:key identifiers, verifiable credentials, zero-knowledge proofs. You own your identity.",
  },
  {
    name: "Messaging",
    description: "End-to-end encrypted. X25519 key exchange, AES-256-GCM. No server ever reads your messages.",
  },
  {
    name: "Governance",
    description: "Quadratic voting. Proposals, delegation, DAOs. Sybil-resistant. Every voice weighted fairly.",
  },
  {
    name: "Payments",
    description: "Multi-chain wallet. Send, receive, swap. Escrow-backed transactions. Trustless commerce.",
  },
  {
    name: "Social",
    description: "Decentralized feeds. Posts, follows, reactions. Your social graph belongs to you.",
  },
  {
    name: "Storage",
    description: "Local-first with CRDTs. IPFS for distribution. Encrypted vaults. Your data, everywhere, always.",
  },
  {
    name: "AI",
    description: "Local inference. Agent framework. Semantic search across all your data. Intelligence without surveillance.",
  },
  {
    name: "Browser",
    description: "Built-in decentralized browser. IPFS gateway, ENS resolution, per-site identity. The web, evolved.",
  },
];

const platforms = [
  "Web",
  "Android",
  "iOS",
  "Linux",
  "macOS",
  "Terminal",
  "CLI",
  "API",
];

export default function Home() {
  return (
    <div className="flex flex-col min-h-screen">
      {/* Hero */}
      <section className="flex flex-col items-center justify-center px-6 pt-32 pb-24">
        <div className="max-w-3xl text-center">
          <h1 className="text-6xl sm:text-7xl md:text-8xl font-extralight tracking-[-0.04em] mb-8">
            Nous
          </h1>
          <p className="text-lg sm:text-xl text-neutral-500 font-light leading-relaxed max-w-xl mx-auto">
            The decentralized everything-app. Identity, messaging, governance,
            payments, AI — unified under one sovereign protocol.
          </p>
          <div className="flex items-center justify-center gap-3 mt-10">
            <Badge
              variant="outline"
              className="text-xs font-mono tracking-wider uppercase px-3 py-1 border-white/10"
            >
              v0.1.0
            </Badge>
            <Badge
              variant="outline"
              className="text-xs font-mono tracking-wider uppercase px-3 py-1 border-white/10"
            >
              Private Alpha
            </Badge>
          </div>
        </div>
      </section>

      <Separator className="opacity-10" />

      {/* Features */}
      <section className="px-6 py-24 max-w-6xl mx-auto w-full">
        <h2 className="text-sm font-mono uppercase tracking-[0.2em] text-neutral-500 mb-16">
          Architecture
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-px bg-white/[0.03]">
          {features.map((feature) => (
            <Card
              key={feature.name}
              className="bg-black border-0 rounded-none p-8 hover:bg-white/[0.02] transition-colors duration-200"
            >
              <CardContent className="p-0">
                <h3 className="text-base font-medium tracking-wide mb-3">
                  {feature.name}
                </h3>
                <p className="text-sm text-neutral-500 font-light leading-relaxed">
                  {feature.description}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <Separator className="opacity-10" />

      {/* Platforms */}
      <section className="px-6 py-24 max-w-6xl mx-auto w-full">
        <h2 className="text-sm font-mono uppercase tracking-[0.2em] text-neutral-500 mb-16">
          Platforms
        </h2>
        <div className="flex flex-wrap gap-4">
          {platforms.map((platform) => (
            <span
              key={platform}
              className="text-sm font-light text-neutral-500 border border-white/[0.06] px-5 py-2.5 tracking-wide hover:text-white hover:border-white/20 transition-all duration-200"
            >
              {platform}
            </span>
          ))}
        </div>
      </section>

      <Separator className="opacity-10" />

      {/* Primitives */}
      <section className="px-6 py-24 max-w-6xl mx-auto w-full">
        <h2 className="text-sm font-mono uppercase tracking-[0.2em] text-neutral-500 mb-16">
          Primitives
        </h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-y-8 gap-x-12">
          {[
            ["Signing", "ed25519"],
            ["Exchange", "x25519"],
            ["Encryption", "AES-256-GCM"],
            ["Derivation", "HKDF-SHA256"],
            ["Identity", "DID:key"],
            ["Networking", "libp2p"],
            ["Storage", "SQLite + CRDTs"],
            ["Consensus", "Raft"],
          ].map(([label, value]) => (
            <div key={label}>
              <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-1.5">
                {label}
              </p>
              <p className="text-sm font-light">{value}</p>
            </div>
          ))}
        </div>
      </section>

      {/* Footer */}
      <footer className="mt-auto px-6 py-12 border-t border-white/[0.04]">
        <div className="max-w-6xl mx-auto flex justify-between items-center">
          <p className="text-xs text-neutral-600 font-mono">
            nous v0.1.0
          </p>
          <p className="text-xs text-neutral-600 font-light">
            Sovereign. Encrypted. Unstoppable.
          </p>
        </div>
      </footer>
    </div>
  );
}
