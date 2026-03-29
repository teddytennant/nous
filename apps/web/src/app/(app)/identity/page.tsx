import { Card, CardContent } from "@/components/ui/card";

const credentials = [
  {
    type: "Age Verification",
    issuer: "did:key:z6Mk...issuer",
    issued: "2026-03-28",
    status: "valid",
    detail: "Verified age >= 18 via ZK proof (value not disclosed)",
  },
];

const keys = [
  { purpose: "Signing", algorithm: "ed25519", fingerprint: "z6Mk...x3rW" },
  { purpose: "Exchange", algorithm: "x25519", fingerprint: "z6LS...k9mP" },
];

export default function IdentityPage() {
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

      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Decentralized Identifier
        </h2>
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none">
          <CardContent className="p-6">
            <p className="text-xs font-mono text-neutral-600 mb-2">DID:key</p>
            <p className="text-sm font-mono font-light break-all leading-relaxed">
              did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK
            </p>
            <div className="flex gap-3 mt-4">
              <button className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150">
                Copy
              </button>
              <button className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150">
                Export
              </button>
              <button className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors duration-150">
                QR Code
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
          {keys.map((key) => (
            <div
              key={key.purpose}
              className="flex items-center justify-between py-4 px-5 bg-white/[0.01]"
            >
              <div>
                <p className="text-sm font-light">{key.purpose}</p>
                <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                  {key.algorithm}
                </p>
              </div>
              <span className="text-xs font-mono text-neutral-600">
                {key.fingerprint}
              </span>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Verifiable Credentials
        </h2>
        {credentials.map((cred) => (
          <Card
            key={cred.type}
            className="bg-white/[0.01] border-white/[0.06] rounded-none"
          >
            <CardContent className="p-6">
              <div className="flex items-start justify-between mb-3">
                <h3 className="text-sm font-light">{cred.type}</h3>
                <span className="text-[10px] font-mono uppercase tracking-wider text-[#d4af37]">
                  {cred.status}
                </span>
              </div>
              <p className="text-xs text-neutral-500 font-light mb-3">
                {cred.detail}
              </p>
              <div className="flex gap-6 text-[10px] font-mono text-neutral-700">
                <span>Issuer: {cred.issuer}</span>
                <span>Issued: {cred.issued}</span>
              </div>
            </CardContent>
          </Card>
        ))}
      </section>
    </div>
  );
}
