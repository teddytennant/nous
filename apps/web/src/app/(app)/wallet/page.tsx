"use client";

import { useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { node, type HealthResponse } from "@/lib/api";

interface Balance {
  token: string;
  amount: string;
  usd: string;
}

interface Transaction {
  id: string;
  type: "send" | "receive";
  amount: string;
  token: string;
  peer: string;
  time: string;
  status: string;
}

const defaultBalances: Balance[] = [
  { token: "ETH", amount: "0.000", usd: "$0.00" },
  { token: "NOUS", amount: "0", usd: "$0.00" },
  { token: "USDC", amount: "0.00", usd: "$0.00" },
];

export default function WalletPage() {
  const [online, setOnline] = useState(false);
  const [balances] = useState<Balance[]>(defaultBalances);
  const [transactions] = useState<Transaction[]>([]);
  const [sendModal, setSendModal] = useState(false);
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [sendToken, setSendToken] = useState("ETH");

  useEffect(() => {
    node
      .health()
      .then(() => setOnline(true))
      .catch(() => setOnline(false));
  }, []);

  return (
    <div className="p-8 max-w-4xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Wallet
        </h1>
        <div className="flex items-center gap-3">
          <p className="text-sm text-neutral-500 font-light">
            Multi-chain. Escrow-backed. Trustless.
          </p>
          <span
            className={`inline-block w-1.5 h-1.5 rounded-full ${
              online ? "bg-emerald-500" : "bg-red-500"
            }`}
          />
        </div>
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
          <button
            onClick={() => setSendModal(true)}
            className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150"
          >
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

      {/* Send modal */}
      {sendModal && (
        <section className="mb-16">
          <Card className="bg-white/[0.01] border-white/[0.06] rounded-none max-w-md">
            <CardContent className="p-6">
              <div className="flex items-center justify-between mb-6">
                <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
                  Send
                </p>
                <button
                  onClick={() => setSendModal(false)}
                  className="text-[10px] font-mono text-neutral-600 hover:text-white transition-colors"
                >
                  Close
                </button>
              </div>
              <div className="space-y-4">
                <input
                  value={sendTo}
                  onChange={(e) => setSendTo(e.target.value)}
                  placeholder="Recipient DID"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                />
                <div className="flex gap-3">
                  <input
                    value={sendAmount}
                    onChange={(e) => setSendAmount(e.target.value)}
                    placeholder="Amount"
                    type="number"
                    className="flex-1 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                  />
                  <select
                    value={sendToken}
                    onChange={(e) => setSendToken(e.target.value)}
                    className="bg-white/[0.02] text-xs font-mono px-4 py-3 outline-none text-neutral-400"
                  >
                    <option value="ETH">ETH</option>
                    <option value="NOUS">NOUS</option>
                    <option value="USDC">USDC</option>
                  </select>
                </div>
                <button className="w-full text-xs font-mono uppercase tracking-wider py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150">
                  Confirm Send
                </button>
              </div>
            </CardContent>
          </Card>
        </section>
      )}

      <section>
        <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
          Transactions
        </h2>
        {transactions.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-neutral-700 font-light">
              No transactions yet
            </p>
            <p className="text-[10px] font-mono text-neutral-800 mt-2">
              Send or receive tokens to see activity
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
