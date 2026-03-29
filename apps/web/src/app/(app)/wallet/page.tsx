"use client";

import { useCallback, useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import {
  node,
  payments,
  type BalanceEntry,
  type TransactionResponse,
  type WalletResponse,
} from "@/lib/api";

export default function WalletPage() {
  const [online, setOnline] = useState(false);
  const [wallet, setWallet] = useState<WalletResponse | null>(null);
  const [transactions, setTransactions] = useState<TransactionResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [sendModal, setSendModal] = useState(false);
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [sendToken, setSendToken] = useState("ETH");
  const [sendMemo, setSendMemo] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const userDid =
    typeof window !== "undefined"
      ? localStorage.getItem("nous_did") || ""
      : "";

  const loadWallet = useCallback(async () => {
    if (!userDid) {
      setLoading(false);
      return;
    }
    try {
      const w = await payments.getWallet(userDid);
      setWallet(w);
      const txs = await payments.getTransactions(userDid, 50);
      setTransactions(txs);
    } catch {
      // Wallet may not exist yet — that's fine
    }
    setLoading(false);
  }, [userDid]);

  useEffect(() => {
    node
      .health()
      .then(() => setOnline(true))
      .catch(() => setOnline(false));
    loadWallet();
  }, [loadWallet]);

  const handleCreateWallet = async () => {
    if (!userDid) {
      setError("Generate an identity first from the Identity page");
      return;
    }
    try {
      const w = await payments.createWallet(userDid);
      setWallet(w);
      setError(null);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to create wallet");
    }
  };

  const handleSend = async () => {
    if (!sendTo || !sendAmount || !userDid) return;
    setSending(true);
    setError(null);
    try {
      await payments.transfer({
        from_did: userDid,
        to_did: sendTo,
        token: sendToken,
        amount: Number(sendAmount),
        memo: sendMemo || undefined,
      });
      setSendModal(false);
      setSendTo("");
      setSendAmount("");
      setSendMemo("");
      await loadWallet();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Transfer failed");
    }
    setSending(false);
  };

  const displayBalances: BalanceEntry[] = wallet?.balances.length
    ? wallet.balances
    : [
        { token: "ETH", amount: "0" },
        { token: "NOUS", amount: "0" },
        { token: "USDC", amount: "0" },
      ];

  const formatTime = (ts: string) => {
    const d = new Date(ts);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    if (diff < 60_000) return "just now";
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  };

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

      {error && (
        <div className="mb-8 px-4 py-3 text-xs font-mono text-red-400 border border-red-900/30 bg-red-950/20">
          {error}
          <button
            onClick={() => setError(null)}
            className="ml-3 text-neutral-600 hover:text-white"
          >
            dismiss
          </button>
        </div>
      )}

      {!wallet && !loading && (
        <section className="mb-16 py-16 text-center">
          <p className="text-sm text-neutral-600 font-light mb-6">
            No wallet found for your identity
          </p>
          <button
            onClick={handleCreateWallet}
            className="text-xs font-mono uppercase tracking-wider px-6 py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
          >
            Create Wallet
          </button>
        </section>
      )}

      {(wallet || loading) && (
        <>
          <section className="mb-16">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-8">
              Balances
            </h2>
            <div className="grid grid-cols-3 gap-px bg-white/[0.03]">
              {displayBalances.map((b) => (
                <Card
                  key={b.token}
                  className="bg-black border-0 rounded-none p-6"
                >
                  <CardContent className="p-0">
                    <p className="text-xs font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                      {b.token}
                    </p>
                    <p className="text-2xl font-extralight mb-1">{b.amount}</p>
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
                    <input
                      value={sendMemo}
                      onChange={(e) => setSendMemo(e.target.value)}
                      placeholder="Memo (optional)"
                      className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                    />
                    <button
                      onClick={handleSend}
                      disabled={sending || !sendTo || !sendAmount}
                      className="w-full text-xs font-mono uppercase tracking-wider py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150 disabled:opacity-30 disabled:cursor-not-allowed"
                    >
                      {sending ? "Sending..." : "Confirm Send"}
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
                {transactions.map((tx) => {
                  const isSend = tx.from_did === userDid;
                  return (
                    <div
                      key={tx.id}
                      className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors duration-150"
                    >
                      <div>
                        <p className="text-sm font-light">
                          {isSend ? "Sent" : "Received"} {tx.amount} {tx.token}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                          {isSend ? "to" : "from"}{" "}
                          {isSend ? tx.to_did : tx.from_did}
                        </p>
                        {tx.memo && (
                          <p className="text-[10px] text-neutral-600 mt-1 italic">
                            {tx.memo}
                          </p>
                        )}
                      </div>
                      <div className="text-right">
                        <p className="text-[10px] text-neutral-700">
                          {formatTime(tx.timestamp)}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-600">
                          {tx.status}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </section>
        </>
      )}
    </div>
  );
}
