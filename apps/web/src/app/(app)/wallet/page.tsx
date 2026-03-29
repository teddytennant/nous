import { Card, CardContent } from "@/components/ui/card";

const balances = [
  { token: "ETH", amount: "0.000", usd: "$0.00" },
  { token: "NOUS", amount: "0", usd: "$0.00" },
  { token: "USDC", amount: "0.00", usd: "$0.00" },
];

const transactions: {
  id: string;
  type: "send" | "receive";
  amount: string;
  token: string;
  peer: string;
  time: string;
  status: string;
}[] = [];

export default function WalletPage() {
  return (
    <div className="p-8 max-w-4xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Wallet
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          Multi-chain. Escrow-backed. Trustless.
        </p>
      </header>

      <section className="mb-16">
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Balances
        </h2>
        <div className="grid grid-cols-3 gap-px bg-white/[0.03]">
          {balances.map((b) => (
            <Card
              key={b.token}
              className="bg-black border-0 rounded-none p-6"
            >
              <CardContent className="p-0">
                <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                  {b.token}
                </p>
                <p className="text-2xl font-extralight mb-1">{b.amount}</p>
                <p className="text-xs text-neutral-700 font-mono">{b.usd}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section className="mb-16">
        <div className="flex gap-3">
          <button className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150">
            Send
          </button>
          <button className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150">
            Receive
          </button>
          <button className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150">
            Swap
          </button>
        </div>
      </section>

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Transactions
        </h2>
        {transactions.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-neutral-700 font-light">
              No transactions yet
            </p>
          </div>
        ) : (
          <div className="space-y-px">
            {transactions.map((tx) => (
              <div
                key={tx.id}
                className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150"
              >
                <div>
                  <p className="text-sm font-light">
                    {tx.type === "send" ? "Sent" : "Received"} {tx.amount}{" "}
                    {tx.token}
                  </p>
                  <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                    {tx.type === "send" ? "to" : "from"} {tx.peer}
                  </p>
                </div>
                <div className="text-right">
                  <p className="text-[10px] text-neutral-700">{tx.time}</p>
                  <p className="text-[10px] font-mono text-neutral-600">
                    {tx.status}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
