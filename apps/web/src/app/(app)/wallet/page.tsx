"use client";

import { useCallback, useEffect, useState } from "react";
import { Card, CardContent } from "@/components/ui/card";
import {
  node,
  payments,
  type BalanceEntry,
  type TransactionResponse,
  type WalletResponse,
  type InvoiceResponse,
  type EscrowResponse,
} from "@/lib/api";

type WalletTab = "balances" | "invoices" | "escrow";

export default function WalletPage() {
  const [online, setOnline] = useState(false);
  const [tab, setTab] = useState<WalletTab>("balances");
  const [wallet, setWallet] = useState<WalletResponse | null>(null);
  const [transactions, setTransactions] = useState<TransactionResponse[]>([]);
  const [invoices, setInvoices] = useState<InvoiceResponse[]>([]);
  const [escrows, setEscrows] = useState<EscrowResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Send modal state
  const [sendModal, setSendModal] = useState(false);
  const [sendTo, setSendTo] = useState("");
  const [sendAmount, setSendAmount] = useState("");
  const [sendToken, setSendToken] = useState("ETH");
  const [sendMemo, setSendMemo] = useState("");
  const [sending, setSending] = useState(false);

  // Invoice create state
  const [showInvoiceForm, setShowInvoiceForm] = useState(false);
  const [invoiceTo, setInvoiceTo] = useState("");
  const [invoiceToken, setInvoiceToken] = useState("NOUS");
  const [invoiceMemo, setInvoiceMemo] = useState("");
  const [invoiceDueDays, setInvoiceDueDays] = useState("30");
  const [invoiceItems, setInvoiceItems] = useState([
    { description: "", quantity: "1", unit_price: "" },
  ]);

  // Escrow create state
  const [showEscrowForm, setShowEscrowForm] = useState(false);
  const [escrowSeller, setEscrowSeller] = useState("");
  const [escrowToken, setEscrowToken] = useState("NOUS");
  const [escrowAmount, setEscrowAmount] = useState("");
  const [escrowDesc, setEscrowDesc] = useState("");
  const [escrowHours, setEscrowHours] = useState("72");
  const [escrowConditions, setEscrowConditions] = useState("");

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
      // Wallet may not exist yet
    }
    setLoading(false);
  }, [userDid]);

  const loadInvoices = useCallback(async () => {
    if (!userDid) return;
    try {
      const inv = await payments.listInvoices(userDid);
      setInvoices(inv);
    } catch {
      // No invoices yet
    }
  }, [userDid]);

  useEffect(() => {
    node
      .health()
      .then(() => setOnline(true))
      .catch(() => setOnline(false));
    loadWallet();
    loadInvoices();
  }, [loadWallet, loadInvoices]);

  const handleCreateWallet = async () => {
    if (!userDid) {
      setError("Generate an identity first");
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

  const handleCreateInvoice = async () => {
    if (!userDid || !invoiceTo) return;
    try {
      const items = invoiceItems
        .filter((i) => i.description && i.unit_price)
        .map((i) => ({
          description: i.description,
          quantity: Number(i.quantity) || 1,
          unit_price: Number(i.unit_price),
        }));
      await payments.createInvoice({
        from_did: userDid,
        to_did: invoiceTo,
        token: invoiceToken,
        days_until_due: Number(invoiceDueDays) || 30,
        memo: invoiceMemo || undefined,
        items,
      });
      setShowInvoiceForm(false);
      setInvoiceTo("");
      setInvoiceMemo("");
      setInvoiceItems([{ description: "", quantity: "1", unit_price: "" }]);
      await loadInvoices();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to create invoice");
    }
  };

  const handlePayInvoice = async (invoiceId: string) => {
    try {
      await payments.payInvoice(invoiceId);
      await loadInvoices();
      await loadWallet();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Payment failed");
    }
  };

  const handleCancelInvoice = async (invoiceId: string) => {
    try {
      await payments.cancelInvoice(invoiceId);
      await loadInvoices();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Cancel failed");
    }
  };

  const handleCreateEscrow = async () => {
    if (!userDid || !escrowSeller || !escrowAmount) return;
    try {
      const conditions = escrowConditions
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
      await payments.createEscrow({
        buyer_did: userDid,
        seller_did: escrowSeller,
        token: escrowToken,
        amount: Number(escrowAmount),
        description: escrowDesc,
        duration_hours: Number(escrowHours) || 72,
        conditions: conditions.length > 0 ? conditions : undefined,
      });
      setShowEscrowForm(false);
      setEscrowSeller("");
      setEscrowAmount("");
      setEscrowDesc("");
      setEscrowConditions("");
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Escrow creation failed");
    }
  };

  const handleReleaseEscrow = async (escrowId: string) => {
    try {
      await payments.releaseEscrow(escrowId, userDid);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Release failed");
    }
  };

  function addInvoiceItem() {
    setInvoiceItems((prev) => [
      ...prev,
      { description: "", quantity: "1", unit_price: "" },
    ]);
  }

  function updateInvoiceItem(idx: number, field: string, value: string) {
    setInvoiceItems((prev) =>
      prev.map((item, i) => (i === idx ? { ...item, [field]: value } : item))
    );
  }

  function removeInvoiceItem(idx: number) {
    setInvoiceItems((prev) => prev.filter((_, i) => i !== idx));
  }

  const displayBalances: BalanceEntry[] = wallet?.balances.length
    ? wallet.balances
    : [
        { token: "ETH", amount: "0" },
        { token: "NOUS", amount: "0" },
        { token: "USDC", amount: "0" },
      ];

  function formatTime(ts: string) {
    const d = new Date(ts);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    if (diff < 60_000) return "just now";
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  function truncateDid(did: string): string {
    if (did.length > 30) return `${did.slice(0, 16)}...${did.slice(-6)}`;
    return did;
  }

  function statusColor(status: string): string {
    switch (status.toLowerCase()) {
      case "paid":
      case "released":
      case "confirmed":
        return "text-emerald-500";
      case "pending":
      case "active":
        return "text-[#d4af37]";
      case "cancelled":
      case "refunded":
        return "text-neutral-600";
      case "disputed":
      case "failed":
        return "text-red-400";
      default:
        return "text-neutral-500";
    }
  }

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
        <div className="mb-8 px-4 py-3 text-xs font-mono text-red-400 border border-red-900/30 bg-red-950/20 flex items-center justify-between">
          <span>{error}</span>
          <button
            onClick={() => setError(null)}
            className="text-neutral-600 hover:text-white"
          >
            dismiss
          </button>
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-8 mb-12">
        {(["balances", "invoices", "escrow"] as WalletTab[]).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`text-xs font-mono uppercase tracking-[0.2em] pb-2 transition-colors duration-150 ${
              tab === t
                ? "text-[#d4af37] border-b border-[#d4af37]"
                : "text-neutral-600 hover:text-neutral-400"
            }`}
          >
            {t}
          </button>
        ))}
      </div>

      {!wallet && !loading && tab === "balances" && (
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

      {/* === BALANCES TAB === */}
      {tab === "balances" && (wallet || loading) && (
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
                      className="text-[10px] font-mono text-neutral-600 hover:text-white"
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
                      className="w-full text-xs font-mono uppercase tracking-wider py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150 disabled:opacity-30"
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
              </div>
            ) : (
              <div className="space-y-px">
                {transactions.map((tx) => {
                  const isSend = tx.from_did === userDid;
                  return (
                    <div
                      key={tx.id}
                      className="flex items-center justify-between py-4 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors"
                    >
                      <div>
                        <p className="text-sm font-light">
                          {isSend ? "Sent" : "Received"} {tx.amount} {tx.token}
                        </p>
                        <p className="text-[10px] font-mono text-neutral-700 mt-0.5">
                          {isSend ? "to" : "from"}{" "}
                          {truncateDid(isSend ? tx.to_did : tx.from_did)}
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
                        <p
                          className={`text-[10px] font-mono ${statusColor(tx.status)}`}
                        >
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

      {/* === INVOICES TAB === */}
      {tab === "invoices" && (
        <>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              Invoices
            </h2>
            <button
              onClick={() => setShowInvoiceForm(!showInvoiceForm)}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
            >
              {showInvoiceForm ? "Cancel" : "Create Invoice"}
            </button>
          </div>

          {showInvoiceForm && (
            <Card className="bg-white/[0.01] border-white/[0.06] rounded-none mb-12">
              <CardContent className="p-6 space-y-4">
                <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-2">
                  New Invoice
                </p>
                <input
                  value={invoiceTo}
                  onChange={(e) => setInvoiceTo(e.target.value)}
                  placeholder="Recipient DID"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                />
                <div className="flex gap-3">
                  <select
                    value={invoiceToken}
                    onChange={(e) => setInvoiceToken(e.target.value)}
                    className="bg-white/[0.02] text-xs font-mono px-4 py-3 outline-none text-neutral-400"
                  >
                    <option value="NOUS">NOUS</option>
                    <option value="ETH">ETH</option>
                    <option value="USDC">USDC</option>
                  </select>
                  <input
                    value={invoiceDueDays}
                    onChange={(e) => setInvoiceDueDays(e.target.value)}
                    placeholder="Due in days"
                    type="number"
                    className="w-32 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                  />
                </div>
                <input
                  value={invoiceMemo}
                  onChange={(e) => setInvoiceMemo(e.target.value)}
                  placeholder="Memo (optional)"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                />

                <div className="space-y-3">
                  <p className="text-[10px] font-mono uppercase tracking-wider text-neutral-600">
                    Line Items
                  </p>
                  {invoiceItems.map((item, idx) => (
                    <div key={idx} className="flex gap-2 items-center">
                      <input
                        value={item.description}
                        onChange={(e) =>
                          updateInvoiceItem(idx, "description", e.target.value)
                        }
                        placeholder="Description"
                        className="flex-1 bg-white/[0.02] text-sm font-light px-3 py-2 outline-none placeholder:text-neutral-700"
                      />
                      <input
                        value={item.quantity}
                        onChange={(e) =>
                          updateInvoiceItem(idx, "quantity", e.target.value)
                        }
                        placeholder="Qty"
                        type="number"
                        className="w-16 bg-white/[0.02] text-sm font-light px-3 py-2 outline-none placeholder:text-neutral-700"
                      />
                      <input
                        value={item.unit_price}
                        onChange={(e) =>
                          updateInvoiceItem(idx, "unit_price", e.target.value)
                        }
                        placeholder="Price"
                        type="number"
                        className="w-24 bg-white/[0.02] text-sm font-light px-3 py-2 outline-none placeholder:text-neutral-700"
                      />
                      {invoiceItems.length > 1 && (
                        <button
                          onClick={() => removeInvoiceItem(idx)}
                          className="text-[10px] font-mono text-neutral-700 hover:text-red-400"
                        >
                          x
                        </button>
                      )}
                    </div>
                  ))}
                  <button
                    onClick={addInvoiceItem}
                    className="text-[10px] font-mono text-neutral-600 hover:text-white"
                  >
                    + Add item
                  </button>
                </div>

                <button
                  onClick={handleCreateInvoice}
                  disabled={
                    !invoiceTo ||
                    invoiceItems.every((i) => !i.description)
                  }
                  className="w-full text-xs font-mono uppercase tracking-wider py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150 disabled:opacity-30"
                >
                  Create Invoice
                </button>
              </CardContent>
            </Card>
          )}

          {invoices.length === 0 ? (
            <div className="py-16 text-center">
              <p className="text-sm text-neutral-700 font-light">
                No invoices yet
              </p>
              <p className="text-[10px] font-mono text-neutral-800 mt-2">
                Create an invoice to request payment
              </p>
            </div>
          ) : (
            <div className="space-y-px">
              {invoices.map((inv) => {
                const isIssuer = inv.from_did === userDid;
                return (
                  <div
                    key={inv.id}
                    className="py-5 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors"
                  >
                    <div className="flex items-start justify-between">
                      <div>
                        <div className="flex items-baseline gap-3 mb-1">
                          <p className="text-sm font-light">
                            {inv.total} {inv.token}
                          </p>
                          <span
                            className={`text-[10px] font-mono uppercase ${statusColor(inv.status)}`}
                          >
                            {inv.status}
                          </span>
                        </div>
                        <p className="text-[10px] font-mono text-neutral-700">
                          {isIssuer
                            ? `To: ${truncateDid(inv.to_did)}`
                            : `From: ${truncateDid(inv.from_did)}`}
                        </p>
                        {inv.memo && (
                          <p className="text-[10px] text-neutral-600 mt-1 italic">
                            {inv.memo}
                          </p>
                        )}
                        <p className="text-[10px] text-neutral-700 mt-1">
                          {inv.items.length} item
                          {inv.items.length !== 1 ? "s" : ""}
                          {" · "}Due:{" "}
                          {new Date(inv.due_at).toLocaleDateString("en-US", {
                            month: "short",
                            day: "numeric",
                          })}
                        </p>
                      </div>
                      <div className="flex gap-2">
                        {!isIssuer && inv.status === "pending" && (
                          <button
                            onClick={() => handlePayInvoice(inv.id)}
                            className="text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5"
                          >
                            Pay
                          </button>
                        )}
                        {isIssuer && inv.status === "pending" && (
                          <button
                            onClick={() => handleCancelInvoice(inv.id)}
                            className="text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 border border-white/10 text-neutral-600 hover:text-red-400 hover:border-red-900/30"
                          >
                            Cancel
                          </button>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* === ESCROW TAB === */}
      {tab === "escrow" && (
        <>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500">
              Escrow
            </h2>
            <button
              onClick={() => setShowEscrowForm(!showEscrowForm)}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-[#d4af37] transition-colors"
            >
              {showEscrowForm ? "Cancel" : "Create Escrow"}
            </button>
          </div>

          {showEscrowForm && (
            <Card className="bg-white/[0.01] border-white/[0.06] rounded-none mb-12">
              <CardContent className="p-6 space-y-4">
                <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-2">
                  New Escrow
                </p>
                <input
                  value={escrowSeller}
                  onChange={(e) => setEscrowSeller(e.target.value)}
                  placeholder="Seller DID"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                />
                <div className="flex gap-3">
                  <input
                    value={escrowAmount}
                    onChange={(e) => setEscrowAmount(e.target.value)}
                    placeholder="Amount"
                    type="number"
                    className="flex-1 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                  />
                  <select
                    value={escrowToken}
                    onChange={(e) => setEscrowToken(e.target.value)}
                    className="bg-white/[0.02] text-xs font-mono px-4 py-3 outline-none text-neutral-400"
                  >
                    <option value="NOUS">NOUS</option>
                    <option value="ETH">ETH</option>
                    <option value="USDC">USDC</option>
                  </select>
                  <input
                    value={escrowHours}
                    onChange={(e) => setEscrowHours(e.target.value)}
                    placeholder="Hours"
                    type="number"
                    className="w-24 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                  />
                </div>
                <input
                  value={escrowDesc}
                  onChange={(e) => setEscrowDesc(e.target.value)}
                  placeholder="Description"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
                />
                <textarea
                  value={escrowConditions}
                  onChange={(e) => setEscrowConditions(e.target.value)}
                  placeholder="Release conditions (one per line)"
                  className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700 resize-none"
                  rows={3}
                />
                <button
                  onClick={handleCreateEscrow}
                  disabled={!escrowSeller || !escrowAmount}
                  className="w-full text-xs font-mono uppercase tracking-wider py-3 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150 disabled:opacity-30"
                >
                  Create Escrow
                </button>
              </CardContent>
            </Card>
          )}

          {escrows.length === 0 ? (
            <div className="py-16 text-center">
              <p className="text-sm text-neutral-700 font-light">
                No active escrows
              </p>
              <p className="text-[10px] font-mono text-neutral-800 mt-2">
                Create an escrow for trustless transactions
              </p>
            </div>
          ) : (
            <div className="space-y-px">
              {escrows.map((esc) => {
                const isBuyer = esc.buyer_did === userDid;
                return (
                  <div
                    key={esc.id}
                    className="py-5 px-5 bg-white/[0.01] hover:bg-white/[0.02] transition-colors"
                  >
                    <div className="flex items-start justify-between">
                      <div>
                        <div className="flex items-baseline gap-3 mb-1">
                          <p className="text-sm font-light">
                            {esc.amount} {esc.token}
                          </p>
                          <span
                            className={`text-[10px] font-mono uppercase ${statusColor(esc.status)}`}
                          >
                            {esc.status}
                          </span>
                        </div>
                        <p className="text-[10px] font-mono text-neutral-700">
                          {isBuyer
                            ? `Seller: ${truncateDid(esc.seller_did)}`
                            : `Buyer: ${truncateDid(esc.buyer_did)}`}
                        </p>
                        <p className="text-[10px] text-neutral-600 mt-1">
                          {esc.description}
                        </p>
                        {esc.conditions.length > 0 && (
                          <div className="mt-2 space-y-0.5">
                            {esc.conditions.map((c, i) => (
                              <p
                                key={i}
                                className="text-[10px] font-mono text-neutral-700"
                              >
                                · {c}
                              </p>
                            ))}
                          </div>
                        )}
                        <p className="text-[10px] text-neutral-700 mt-1">
                          Expires:{" "}
                          {new Date(esc.expires_at).toLocaleDateString(
                            "en-US",
                            {
                              month: "short",
                              day: "numeric",
                              hour: "2-digit",
                              minute: "2-digit",
                            }
                          )}
                        </p>
                      </div>
                      {isBuyer && esc.status === "active" && (
                        <button
                          onClick={() => handleReleaseEscrow(esc.id)}
                          className="text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5"
                        >
                          Release
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );
}
