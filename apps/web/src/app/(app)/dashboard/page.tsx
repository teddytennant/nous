import { Card, CardContent } from "@/components/ui/card";

const stats = [
  { label: "Identity", value: "Active", detail: "did:key:z6Mk...x3rW" },
  { label: "Events", value: "0", detail: "posts published" },
  { label: "Following", value: "0", detail: "accounts" },
  { label: "Peers", value: "0", detail: "connected nodes" },
];

const features = [
  { name: "X3DH", status: "live", description: "Extended Triple Diffie-Hellman key agreement" },
  { name: "Double Ratchet", status: "live", description: "Forward-secret message encryption" },
  { name: "ZK Proofs", status: "live", description: "Schnorr proofs and Pedersen commitments" },
  { name: "GraphQL", status: "live", description: "Full query and mutation API" },
  { name: "Quadratic Voting", status: "live", description: "Sybil-resistant governance" },
  { name: "CRDT Storage", status: "live", description: "Conflict-free replicated data" },
];

export default function DashboardPage() {
  return (
    <div className="p-8 max-w-5xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Dashboard
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          System overview and node status
        </p>
      </header>

      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Status
        </h2>
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-px bg-white/[0.03]">
          {stats.map((stat) => (
            <Card
              key={stat.label}
              className="bg-black border-0 rounded-none p-6"
            >
              <CardContent className="p-0">
                <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                  {stat.label}
                </p>
                <p className="text-2xl font-extralight mb-1">{stat.value}</p>
                <p className="text-xs text-neutral-600 font-light font-mono truncate">
                  {stat.detail}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Protocol Modules
        </h2>
        <div className="space-y-px">
          {features.map((f) => (
            <div
              key={f.name}
              className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150"
            >
              <div>
                <p className="text-sm font-light">{f.name}</p>
                <p className="text-xs text-neutral-600 font-light mt-0.5">
                  {f.description}
                </p>
              </div>
              <span className="text-[10px] font-mono uppercase tracking-wider text-[#d4af37]">
                {f.status}
              </span>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
