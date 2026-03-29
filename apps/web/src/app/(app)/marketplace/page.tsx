"use client";

import { useCallback, useEffect, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import {
  marketplace,
  orders,
  disputes,
  offers,
  type ListingResponse,
  type OrderResponse,
  type DisputeResponse,
  type OfferResponse,
} from "@/lib/api";
import { cn } from "@/lib/utils";

type Tab = "listings" | "orders" | "disputes" | "offers";

const TABS: { key: Tab; label: string }[] = [
  { key: "listings", label: "Listings" },
  { key: "orders", label: "Orders" },
  { key: "disputes", label: "Disputes" },
  { key: "offers", label: "Offers" },
];

const CATEGORIES = [
  "All",
  "Physical",
  "Digital",
  "Service",
  "NFT",
  "Data",
  "Other",
];

function formatPrice(amount: number, token: string): string {
  if (amount === 0) return "Free";
  return `${(amount / 100).toFixed(2)} ${token}`;
}

function truncateDid(did: string): string {
  if (did.length <= 24) return did;
  return `${did.slice(0, 16)}...${did.slice(-6)}`;
}

function statusColor(status: string): string {
  switch (status) {
    case "Active":
    case "Pending":
    case "Open":
      return "text-[#d4af37]";
    case "Completed":
    case "Accepted":
    case "ResolvedBuyerWins":
    case "ResolvedSellerWins":
      return "text-emerald-500";
    case "Cancelled":
    case "Rejected":
    case "Withdrawn":
    case "Refunded":
      return "text-neutral-600";
    case "Disputed":
    case "Escalated":
      return "text-red-400";
    case "EscrowFunded":
    case "Shipped":
    case "Delivered":
    case "UnderReview":
    case "Countered":
      return "text-blue-400";
    default:
      return "text-neutral-500";
  }
}

// ── Listings Tab ──────────────────────────────────────────────────────────

function ListingsTab() {
  const [listingsList, setListings] = useState<ListingResponse[]>([]);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [newListing, setNewListing] = useState({
    title: "",
    description: "",
    category: "digital",
    price_token: "USDC",
    price_amount: "",
    tags: "",
  });

  const fetchListings = useCallback(async () => {
    try {
      const params: { text?: string; category?: string; limit?: number } = {
        limit: 50,
      };
      if (search) params.text = search;
      if (category !== "All") params.category = category.toLowerCase();
      const res = await marketplace.search(params);
      setListings(res.listings || []);
      setError(null);
    } catch {
      setError("API offline");
      setListings([]);
    }
  }, [search, category]);

  useEffect(() => {
    startTransition(() => {
      fetchListings();
    });
  }, [fetchListings]);

  async function createListing() {
    if (!newListing.title.trim() || !newListing.price_amount) return;
    try {
      await marketplace.createListing({
        seller_did:
          localStorage.getItem("nous_did") ||
          "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
        title: newListing.title,
        description: newListing.description,
        category: newListing.category,
        price_token: newListing.price_token,
        price_amount: parseInt(newListing.price_amount, 10),
        tags: newListing.tags
          ? newListing.tags.split(",").map((t) => t.trim())
          : [],
      });
      setCreating(false);
      setNewListing({
        title: "",
        description: "",
        category: "digital",
        price_token: "USDC",
        price_amount: "",
        tags: "",
      });
      fetchListings();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create listing");
    }
  }

  return (
    <>
      {error && <p className="text-xs text-red-500 mb-6">{error}</p>}

      <div className="flex justify-end mb-8">
        <button
          onClick={() => setCreating(!creating)}
          className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150"
        >
          {creating ? "Cancel" : "New Listing"}
        </button>
      </div>

      {creating && (
        <Card className="bg-white/[0.01] border-white/[0.06] rounded-none mb-12">
          <CardContent className="p-6">
            <p className="text-xs font-mono uppercase tracking-[0.2em] text-neutral-500 mb-6">
              Create Listing
            </p>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <input
                value={newListing.title}
                onChange={(e) =>
                  setNewListing((p) => ({ ...p, title: e.target.value }))
                }
                placeholder="Title"
                className="bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
              />
              <select
                value={newListing.category}
                onChange={(e) =>
                  setNewListing((p) => ({ ...p, category: e.target.value }))
                }
                className="bg-white/[0.02] text-xs font-mono px-4 py-3 outline-none text-neutral-400"
              >
                <option value="physical">Physical</option>
                <option value="digital">Digital</option>
                <option value="service">Service</option>
                <option value="nft">NFT</option>
                <option value="data">Data</option>
                <option value="other">Other</option>
              </select>
            </div>
            <textarea
              value={newListing.description}
              onChange={(e) =>
                setNewListing((p) => ({ ...p, description: e.target.value }))
              }
              placeholder="Description"
              rows={3}
              className="w-full bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700 mb-4 resize-none"
            />
            <div className="grid grid-cols-3 gap-4 mb-4">
              <input
                value={newListing.price_amount}
                onChange={(e) =>
                  setNewListing((p) => ({
                    ...p,
                    price_amount: e.target.value,
                  }))
                }
                placeholder="Price (minor units)"
                type="number"
                className="bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
              />
              <select
                value={newListing.price_token}
                onChange={(e) =>
                  setNewListing((p) => ({
                    ...p,
                    price_token: e.target.value,
                  }))
                }
                className="bg-white/[0.02] text-xs font-mono px-4 py-3 outline-none text-neutral-400"
              >
                <option value="USDC">USDC</option>
                <option value="ETH">ETH</option>
                <option value="NOUS">NOUS</option>
              </select>
              <input
                value={newListing.tags}
                onChange={(e) =>
                  setNewListing((p) => ({ ...p, tags: e.target.value }))
                }
                placeholder="Tags (comma-separated)"
                className="bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
              />
            </div>
            <button
              onClick={createListing}
              className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
            >
              Publish
            </button>
          </CardContent>
        </Card>
      )}

      <section className="mb-8">
        <div className="flex gap-4 items-center mb-6">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search listings..."
            className="flex-1 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
          />
        </div>
        <div className="flex gap-2">
          {CATEGORIES.map((cat) => (
            <button
              key={cat}
              onClick={() => setCategory(cat)}
              className={cn(
                "text-[10px] font-mono uppercase tracking-wider px-4 py-2 border transition-all duration-150",
                category === cat
                  ? "border-[#d4af37]/30 text-[#d4af37]"
                  : "border-white/[0.06] text-neutral-600 hover:text-neutral-400"
              )}
            >
              {cat}
            </button>
          ))}
        </div>
      </section>

      <section>
        {listingsList.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-neutral-700 font-light">
              No listings found
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-px bg-white/[0.03]">
            {listingsList.map((listing) => (
              <Card
                key={listing.id}
                className="bg-black border-0 rounded-none"
              >
                <CardContent className="p-6">
                  <div className="flex items-start justify-between mb-3">
                    <h3 className="text-sm font-light">{listing.title}</h3>
                    <span
                      className={cn(
                        "text-[10px] font-mono uppercase tracking-wider",
                        statusColor(listing.status)
                      )}
                    >
                      {listing.status}
                    </span>
                  </div>
                  <p className="text-xs text-neutral-500 font-light mb-4 line-clamp-2">
                    {listing.description}
                  </p>
                  <div className="flex items-baseline justify-between">
                    <p className="text-lg font-extralight">
                      {formatPrice(listing.price_amount, listing.price_token)}
                    </p>
                    <span className="text-[10px] font-mono text-neutral-700">
                      {listing.category}
                    </span>
                  </div>
                  {listing.tags && listing.tags.length > 0 && (
                    <div className="flex gap-2 mt-3">
                      {listing.tags.map((tag) => (
                        <span
                          key={tag}
                          className="text-[10px] font-mono text-neutral-700 border border-white/[0.04] px-2 py-0.5"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  )}
                  <p className="text-[10px] font-mono text-neutral-800 mt-3">
                    {truncateDid(listing.seller_did)}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>
    </>
  );
}

// ── Orders Tab ────────────────────────────────────────────────────────────

function OrdersTab() {
  const [ordersList, setOrders] = useState<OrderResponse[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    orders
      .list()
      .then((res) => setOrders(res.orders || []))
      .catch(() => {
        setError("API offline");
        setOrders([]);
      });
  }, []);

  return (
    <>
      {error && <p className="text-xs text-red-500 mb-6">{error}</p>}

      {ordersList.length === 0 ? (
        <div className="py-16 text-center">
          <p className="text-sm text-neutral-700 font-light">No orders yet</p>
          <p className="text-[10px] font-mono text-neutral-800 mt-2">
            Purchase a listing to create an order
          </p>
        </div>
      ) : (
        <div className="space-y-px">
          {ordersList.map((order) => (
            <Card
              key={order.id}
              className="bg-white/[0.01] border-white/[0.06] rounded-none"
            >
              <CardContent className="p-6">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <p className="text-xs font-mono text-neutral-600 mb-1">
                      {order.id}
                    </p>
                    <p className="text-sm font-light">
                      {order.quantity}x @ {formatPrice(order.amount, order.token)}
                    </p>
                  </div>
                  <span
                    className={cn(
                      "text-[10px] font-mono uppercase tracking-wider",
                      statusColor(order.status)
                    )}
                  >
                    {order.status}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-4 text-xs text-neutral-500 font-light">
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Buyer
                    </span>
                    <p className="mt-1">{truncateDid(order.buyer_did)}</p>
                  </div>
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Seller
                    </span>
                    <p className="mt-1">{truncateDid(order.seller_did)}</p>
                  </div>
                </div>

                {order.shipping && (
                  <div className="mt-4 pt-4 border-t border-white/[0.04]">
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Shipping
                    </span>
                    <p className="text-xs text-neutral-500 font-light mt-1">
                      {order.shipping.carrier} — {order.shipping.tracking_id}
                    </p>
                  </div>
                )}

                {order.escrow_id && (
                  <p className="text-[10px] font-mono text-neutral-800 mt-3">
                    Escrow: {order.escrow_id}
                  </p>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </>
  );
}

// ── Disputes Tab ──────────────────────────────────────────────────────────

function DisputesTab() {
  const [disputesList, setDisputes] = useState<DisputeResponse[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    disputes
      .list()
      .then((res) => setDisputes(res.disputes || []))
      .catch(() => {
        setError("API offline");
        setDisputes([]);
      });
  }, []);

  return (
    <>
      {error && <p className="text-xs text-red-500 mb-6">{error}</p>}

      {disputesList.length === 0 ? (
        <div className="py-16 text-center">
          <p className="text-sm text-neutral-700 font-light">
            No disputes
          </p>
          <p className="text-[10px] font-mono text-neutral-800 mt-2">
            Disputes appear when an order is contested
          </p>
        </div>
      ) : (
        <div className="space-y-px">
          {disputesList.map((dispute) => (
            <Card
              key={dispute.id}
              className="bg-white/[0.01] border-white/[0.06] rounded-none"
            >
              <CardContent className="p-6">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <p className="text-xs font-mono text-neutral-600 mb-1">
                      {dispute.id}
                    </p>
                    <p className="text-sm font-light">{dispute.description}</p>
                  </div>
                  <span
                    className={cn(
                      "text-[10px] font-mono uppercase tracking-wider",
                      statusColor(dispute.status)
                    )}
                  >
                    {dispute.status}
                  </span>
                </div>

                <div className="grid grid-cols-3 gap-4 text-xs mt-4">
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Reason
                    </span>
                    <p className="text-neutral-500 font-light mt-1">
                      {dispute.reason.replace(/([A-Z])/g, " $1").trim()}
                    </p>
                  </div>
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Evidence
                    </span>
                    <p className="text-neutral-500 font-light mt-1">
                      {dispute.evidence_count} item{dispute.evidence_count !== 1 ? "s" : ""}
                    </p>
                  </div>
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Arbiter
                    </span>
                    <p className="text-neutral-500 font-light mt-1">
                      {dispute.arbiter_did
                        ? truncateDid(dispute.arbiter_did)
                        : "Unassigned"}
                    </p>
                  </div>
                </div>

                {dispute.resolution_note && (
                  <div className="mt-4 pt-4 border-t border-white/[0.04]">
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Resolution
                    </span>
                    <p className="text-xs text-neutral-500 font-light mt-1">
                      {dispute.resolution_note}
                    </p>
                  </div>
                )}

                <div className="flex justify-between mt-4">
                  <p className="text-[10px] font-mono text-neutral-800">
                    Initiator: {truncateDid(dispute.initiator_did)}
                  </p>
                  <p className="text-[10px] font-mono text-neutral-800">
                    Order: {dispute.order_id}
                  </p>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </>
  );
}

// ── Offers Tab ────────────────────────────────────────────────────────────

function OffersTab() {
  const [offersList, setOffers] = useState<OfferResponse[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    offers
      .list()
      .then((res) => setOffers(res.offers || []))
      .catch(() => {
        setError("API offline");
        setOffers([]);
      });
  }, []);

  return (
    <>
      {error && <p className="text-xs text-red-500 mb-6">{error}</p>}

      {offersList.length === 0 ? (
        <div className="py-16 text-center">
          <p className="text-sm text-neutral-700 font-light">No offers yet</p>
          <p className="text-[10px] font-mono text-neutral-800 mt-2">
            Make an offer on a listing to negotiate price
          </p>
        </div>
      ) : (
        <div className="space-y-px">
          {offersList.map((offer) => (
            <Card
              key={offer.id}
              className="bg-white/[0.01] border-white/[0.06] rounded-none"
            >
              <CardContent className="p-6">
                <div className="flex items-start justify-between mb-3">
                  <div>
                    <p className="text-xs font-mono text-neutral-600 mb-1">
                      {offer.id}
                    </p>
                    <p className="text-lg font-extralight">
                      {formatPrice(offer.amount, offer.token)}
                    </p>
                  </div>
                  <span
                    className={cn(
                      "text-[10px] font-mono uppercase tracking-wider",
                      statusColor(offer.status)
                    )}
                  >
                    {offer.status}
                  </span>
                </div>

                {offer.message && (
                  <p className="text-xs text-neutral-500 font-light mb-4 italic">
                    &ldquo;{offer.message}&rdquo;
                  </p>
                )}

                {offer.counter_amount !== null && (
                  <div className="mb-4 px-4 py-3 bg-white/[0.02]">
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Counter
                    </span>
                    <p className="text-sm font-light mt-1">
                      {formatPrice(offer.counter_amount, offer.token)}
                    </p>
                  </div>
                )}

                <div className="grid grid-cols-2 gap-4 text-xs text-neutral-500 font-light">
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Buyer
                    </span>
                    <p className="mt-1">{truncateDid(offer.buyer_did)}</p>
                  </div>
                  <div>
                    <span className="text-neutral-700 font-mono text-[10px] uppercase tracking-wider">
                      Expires
                    </span>
                    <p className="mt-1">
                      {new Date(offer.expires_at).toLocaleDateString()}
                    </p>
                  </div>
                </div>

                <p className="text-[10px] font-mono text-neutral-800 mt-3">
                  Listing: {offer.listing_id}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────

export default function MarketplacePage() {
  const [tab, setTab] = useState<Tab>("listings");

  return (
    <div className="p-8 max-w-5xl">
      <header className="mb-16">
        <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
          Marketplace
        </h1>
        <p className="text-sm text-neutral-500 font-light">
          P2P. Reputation-gated. Escrow-backed.
        </p>
      </header>

      <nav className="flex gap-0 mb-12 border-b border-white/[0.06]">
        {TABS.map(({ key, label }) => (
          <button
            key={key}
            onClick={() => setTab(key)}
            className={cn(
              "text-xs font-mono uppercase tracking-wider px-6 py-3 -mb-px border-b-2 transition-all duration-150",
              tab === key
                ? "border-[#d4af37] text-[#d4af37]"
                : "border-transparent text-neutral-600 hover:text-neutral-400"
            )}
          >
            {label}
          </button>
        ))}
      </nav>

      {tab === "listings" && <ListingsTab />}
      {tab === "orders" && <OrdersTab />}
      {tab === "disputes" && <DisputesTab />}
      {tab === "offers" && <OffersTab />}
    </div>
  );
}
