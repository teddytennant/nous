"use client";

import { useCallback, useEffect, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { DataTable, type Column } from "@/components/ui/data-table";
import { Tooltip } from "@/components/ui/tooltip";
import { Avatar } from "@/components/avatar";
import {
  marketplace,
  orders,
  disputes,
  offers,
  type ListingResponse,
  type OrderResponse,
  type DisputeResponse,
  type OfferResponse,
  type SellerRating,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import { EmptyState, MarketplaceIllustration, OrdersIllustration, DisputeIllustration, OffersIllustration } from "@/components/empty-state";
import { PageHeader } from "@/components/page-header";
import { useToast } from "@/components/toast";
import { usePageShortcuts } from "@/components/keyboard-shortcuts";
import { ChevronDown, ShoppingCart, MessageSquare, Star, Clock, Package, User } from "lucide-react";
import { Dialog, DialogHeader, DialogTitle, DialogDescription, DialogBody, DialogFooter } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Select, type SelectOption } from "@/components/ui/select";

const CATEGORY_OPTIONS: SelectOption[] = [
  { value: "physical", label: "Physical" },
  { value: "digital", label: "Digital" },
  { value: "service", label: "Service" },
  { value: "nft", label: "NFT" },
  { value: "data", label: "Data" },
  { value: "other", label: "Other" },
];

const PRICE_TOKEN_OPTIONS: SelectOption[] = [
  { value: "USDC", label: "USDC" },
  { value: "ETH", label: "ETH" },
  { value: "NOUS", label: "NOUS" },
];

type Tab = "listings" | "orders" | "disputes" | "offers";
type SortKey = "newest" | "price-low" | "price-high" | "title";

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

const SORT_OPTIONS: { key: SortKey; label: string }[] = [
  { key: "newest", label: "Newest" },
  { key: "price-low", label: "Price ↑" },
  { key: "price-high", label: "Price ↓" },
  { key: "title", label: "Title A–Z" },
];


// ── Category icons (inline SVG) ─────────────────────────────────────────

function CategoryIcon({ category, className }: { category: string; className?: string }) {
  const c = category.toLowerCase();
  const baseClass = `${className ?? "w-4 h-4"} shrink-0 category-icon-enter`;

  // Physical — package/box
  if (c === "physical") return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <path d="M12 2l9 4.5v11L12 22l-9-4.5v-11L12 2z" />
      <path d="M12 22V11" />
      <path d="M21 6.5l-9 4.5-9-4.5" />
      <path d="M7.5 4.2L16.5 9" />
    </svg>
  );

  // Digital — code/download
  if (c === "digital") return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <polyline points="16 18 22 12 16 6" />
      <polyline points="8 6 2 12 8 18" />
      <line x1="14" y1="4" x2="10" y2="20" />
    </svg>
  );

  // Service — wrench
  if (c === "service") return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
    </svg>
  );

  // NFT — diamond/gem
  if (c === "nft") return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <path d="M6 3h12l4 6-10 13L2 9l4-6z" />
      <path d="M2 9h20" />
      <path d="M10 3l-2 6 4 13 4-13-2-6" />
    </svg>
  );

  // Data — database
  if (c === "data") return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <ellipse cx="12" cy="5" rx="9" ry="3" />
      <path d="M21 12c0 1.66-4.03 3-9 3s-9-1.34-9-3" />
      <path d="M3 5v14c0 1.66 4.03 3 9 3s9-1.34 9-3V5" />
    </svg>
  );

  // Other/fallback — layers
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" className={baseClass}>
      <polygon points="12 2 2 7 12 12 22 7 12 2" />
      <polyline points="2 17 12 22 22 17" />
      <polyline points="2 12 12 17 22 12" />
    </svg>
  );
}

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

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

function formatRelativeTime(iso: string): string {
  const diff = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (diff < 60) return "just now";
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 604800) return `${Math.floor(diff / 86400)}d ago`;
  return formatDate(iso);
}

function ListingsTab() {
  const [listingsList, setListings] = useState<ListingResponse[]>([]);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [sortKey, setSortKey] = useState<SortKey>("newest");
  const [loading, setLoading] = useState(true);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [sellerRatings, setSellerRatings] = useState<Record<string, SellerRating>>({});
  const { toast } = useToast();
  const [creating, setCreating] = useState(false);
  const [newListing, setNewListing] = useState({
    title: "",
    description: "",
    category: "digital",
    price_token: "USDC",
    price_amount: "",
    tags: "",
  });

  // Listen for shortcut-triggered create
  useEffect(() => {
    function onCreateListing() {
      setCreating(true);
    }
    window.addEventListener("nous:create-listing", onCreateListing);
    return () => window.removeEventListener("nous:create-listing", onCreateListing);
  }, []);

  const fetchListings = useCallback(async () => {
    try {
      const params: { text?: string; category?: string; limit?: number } = {
        limit: 50,
      };
      if (search) params.text = search;
      if (category !== "All") params.category = category.toLowerCase();
      const res = await marketplace.search(params);
      setListings(res.listings || []);
    } catch {
      toast({ title: "API offline", variant: "error" });
      setListings([]);
    } finally {
      setLoading(false);
    }
  }, [search, category, toast]);

  useEffect(() => {
    startTransition(() => {
      fetchListings();
    });
  }, [fetchListings]);

  // Fetch seller rating when a listing is expanded
  useEffect(() => {
    if (!selectedId) return;
    const listing = listingsList.find((l) => l.id === selectedId);
    if (!listing || sellerRatings[listing.seller_did]) return;
    marketplace.getSellerRating(listing.seller_did).then((rating) => {
      setSellerRatings((prev) => ({ ...prev, [listing.seller_did]: rating }));
    }).catch(() => {
      // Rating not available — that's fine
    });
  }, [selectedId, listingsList, sellerRatings]);

  function toggleListing(id: string) {
    setSelectedId((prev) => (prev === id ? null : id));
  }

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
      const savedTitle = newListing.title;
      setNewListing({
        title: "",
        description: "",
        category: "digital",
        price_token: "USDC",
        price_amount: "",
        tags: "",
      });
      fetchListings();
      toast({ title: "Listing published", description: savedTitle, variant: "success" });
    } catch (e) {
      toast({ title: "Failed to create listing", description: e instanceof Error ? e.message : undefined, variant: "error" });
    }
  }

  return (
    <>
      <div className="flex justify-end mb-8">
        <button
          onClick={() => setCreating(true)}
          className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150"
        >
          New Listing
        </button>
      </div>

      <Dialog open={creating} onOpenChange={setCreating}>
        <DialogHeader>
          <DialogTitle>Create Listing</DialogTitle>
          <DialogDescription>Publish to the decentralized marketplace — escrow-backed, reputation-gated</DialogDescription>
        </DialogHeader>
        <DialogBody className="space-y-4">
          <div className="grid grid-cols-[1fr_auto] gap-3">
            <Input
              value={newListing.title}
              onChange={(e) =>
                setNewListing((p) => ({ ...p, title: e.target.value }))
              }
              placeholder="What are you selling?"
              label="Title"
            />
            <Select
              value={newListing.category}
              onValueChange={(val) =>
                setNewListing((p) => ({ ...p, category: val }))
              }
              options={CATEGORY_OPTIONS}
              label="Category"
            />
          </div>
          <Textarea
            value={newListing.description}
            onChange={(e) =>
              setNewListing((p) => ({ ...p, description: e.target.value }))
            }
            placeholder="Describe your listing in detail..."
            label="Description"
            rows={3}
          />
          <div className="grid grid-cols-3 gap-3">
            <Input
              value={newListing.price_amount}
              onChange={(e) =>
                setNewListing((p) => ({
                  ...p,
                  price_amount: e.target.value,
                }))
              }
              placeholder="0"
              type="number"
              label="Price (minor units)"
            />
            <Select
              value={newListing.price_token}
              onValueChange={(val) =>
                setNewListing((p) => ({
                  ...p,
                  price_token: val,
                }))
              }
              options={PRICE_TOKEN_OPTIONS}
              label="Token"
            />
            <Input
              value={newListing.tags}
              onChange={(e) =>
                setNewListing((p) => ({ ...p, tags: e.target.value }))
              }
              placeholder="art, digital"
              label="Tags"
            />
          </div>
        </DialogBody>
        <DialogFooter>
          <button
            onClick={() => setCreating(false)}
            className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 text-neutral-500 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={createListing}
            className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
          >
            Publish
          </button>
        </DialogFooter>
      </Dialog>

      <section className="mb-8">
        <div className="flex gap-4 items-center mb-6">
          <input
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search listings..."
            className="flex-1 bg-white/[0.02] text-sm font-light px-4 py-3 outline-none placeholder:text-neutral-700"
          />
        </div>
        <div className="flex flex-col sm:flex-row gap-3 sm:items-center sm:justify-between">
          <div className="flex gap-2 flex-wrap">
            {CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setCategory(cat)}
                className={cn(
                  "flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wider px-4 py-2 border transition-all duration-150",
                  category === cat
                    ? "border-[#d4af37]/30 text-[#d4af37] bg-[#d4af37]/[0.06]"
                    : "border-white/[0.06] text-neutral-600 hover:text-neutral-400"
                )}
              >
                {cat !== "All" && <CategoryIcon category={cat} className="w-3 h-3" />}
                {cat}
              </button>
            ))}
          </div>
          <div className="flex gap-2">
            {SORT_OPTIONS.map((opt) => (
              <button
                key={opt.key}
                onClick={() => setSortKey(opt.key)}
                className={cn(
                  "text-[10px] font-mono uppercase tracking-wider px-3 py-2 transition-all duration-150",
                  sortKey === opt.key
                    ? "text-white bg-white/[0.06]"
                    : "text-neutral-700 hover:text-neutral-400"
                )}
              >
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      </section>

      <section>
        {(() => {
          const sortedListings = [...listingsList].sort((a, b) => {
            switch (sortKey) {
              case "newest":
                return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
              case "price-low":
                return a.price_amount - b.price_amount;
              case "price-high":
                return b.price_amount - a.price_amount;
              case "title":
                return a.title.localeCompare(b.title);
              default:
                return 0;
            }
          });

          return loading ? (
          <div className="grid grid-cols-2 gap-px bg-white/[0.03]">
            {Array.from({ length: 4 }).map((_, i) => (
              <Card key={i} className="bg-black border-0 rounded-none">
                <CardContent className="p-6">
                  <div className="flex items-start justify-between mb-3">
                    <Skeleton className="h-3.5 w-32" />
                    <Skeleton className="h-2.5 w-14" />
                  </div>
                  <div className="space-y-1.5 mb-4">
                    <Skeleton className="h-3 w-full" />
                    <Skeleton className="h-3 w-3/4" />
                  </div>
                  <div className="flex items-baseline justify-between">
                    <Skeleton className="h-5 w-20" />
                    <Skeleton className="h-2.5 w-14" />
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        ) : sortedListings.length === 0 ? (
          listingsList.length > 0 ? (
            <EmptyState
              icon={<MarketplaceIllustration />}
              title="No matching listings"
              description={`No listings match the current${category !== "All" ? ` "${category}"` : ""} filter.`}
              action={
                <button
                  onClick={() => { setCategory("All"); setSearch(""); }}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-400 hover:text-white hover:border-white/20 transition-all duration-150"
                >
                  Show all
                </button>
              }
            />
          ) : (
            <EmptyState
              icon={<MarketplaceIllustration />}
              title="No listings found"
              description="Be the first to list something on the decentralized marketplace. Escrow-backed, reputation-gated."
              action={
                <button
                  onClick={() => setCreating(true)}
                  className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                >
                  Create Listing
                </button>
              }
            />
          )
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-px bg-white/[0.03] stagger-in">
            {sortedListings.map((listing) => {
              const isExpanded = selectedId === listing.id;
              const rating = sellerRatings[listing.seller_did];

              return (
              <Card
                key={listing.id}
                onClick={() => toggleListing(listing.id)}
                className={cn(
                  "bg-black border-0 rounded-none cursor-pointer transition-colors duration-150",
                  isExpanded
                    ? "sm:col-span-2 bg-white/[0.01]"
                    : "card-lift"
                )}
              >
                <CardContent className="p-6">
                  {/* Header row */}
                  <div className="flex items-start justify-between mb-3">
                    <h3 className="text-sm font-light">{listing.title}</h3>
                    <div className="flex items-center gap-3 shrink-0 ml-3">
                      <span
                        className={cn(
                          "text-[10px] font-mono uppercase tracking-wider",
                          statusColor(listing.status)
                        )}
                      >
                        {listing.status}
                      </span>
                      <ChevronDown
                        size={14}
                        className={cn(
                          "text-neutral-700 transition-transform duration-200",
                          isExpanded && "rotate-180"
                        )}
                      />
                    </div>
                  </div>

                  {/* Description — truncated when collapsed, full when expanded */}
                  <p className={cn(
                    "text-xs text-neutral-500 font-light mb-4",
                    !isExpanded && "line-clamp-2"
                  )}>
                    {listing.description}
                  </p>

                  {/* Price + category row */}
                  <div className="flex items-center justify-between">
                    <p className="text-lg font-extralight">
                      {formatPrice(listing.price_amount, listing.price_token)}
                    </p>
                    <span className="flex items-center gap-1.5 text-[10px] font-mono text-neutral-700">
                      <CategoryIcon category={listing.category} className="w-3 h-3" />
                      {listing.category}
                    </span>
                  </div>

                  {/* Tags */}
                  {listing.tags && listing.tags.length > 0 && (
                    <div className="flex gap-2 mt-3 flex-wrap">
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

                  {/* Collapsed: seller DID */}
                  {!isExpanded && (
                    <p className="text-[10px] font-mono text-neutral-800 mt-3">
                      {truncateDid(listing.seller_did)}
                    </p>
                  )}

                  {/* Expanded detail section */}
                  <div
                    className="listing-detail"
                    data-expanded={isExpanded}
                  >
                    <div className="listing-detail-inner">
                      <div className="pt-5 mt-5 border-t border-white/[0.04]">
                        {/* Detail grid */}
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 mb-5">
                          <div>
                            <span className="flex items-center gap-1.5 text-neutral-700 font-mono text-[10px] uppercase tracking-wider mb-1">
                              <User size={10} />
                              Seller
                            </span>
                            <div className="flex items-center gap-2">
                              <Avatar did={listing.seller_did} size="xs" />
                              <p className="text-xs text-neutral-500 font-light truncate">
                                {truncateDid(listing.seller_did)}
                              </p>
                            </div>
                          </div>
                          <div>
                            <span className="flex items-center gap-1.5 text-neutral-700 font-mono text-[10px] uppercase tracking-wider mb-1">
                              <Package size={10} />
                              Quantity
                            </span>
                            <p className="text-xs text-neutral-500 font-light">
                              {listing.quantity} available
                            </p>
                          </div>
                          <div>
                            <span className="flex items-center gap-1.5 text-neutral-700 font-mono text-[10px] uppercase tracking-wider mb-1">
                              <Clock size={10} />
                              Listed
                            </span>
                            <p className="text-xs text-neutral-500 font-light">
                              {formatRelativeTime(listing.created_at)}
                            </p>
                          </div>
                          <div>
                            <span className="flex items-center gap-1.5 text-neutral-700 font-mono text-[10px] uppercase tracking-wider mb-1">
                              <Star size={10} />
                              Rating
                            </span>
                            <p className="text-xs text-neutral-500 font-light">
                              {rating
                                ? `${rating.average_rating.toFixed(1)}/5 (${rating.total_reviews} review${rating.total_reviews !== 1 ? "s" : ""})`
                                : "No reviews yet"}
                            </p>
                          </div>
                        </div>

                        {/* Listing ID */}
                        <p className="text-[10px] font-mono text-neutral-800 mb-5">
                          ID: {listing.id}
                        </p>

                        {/* Actions */}
                        {listing.status === "Active" && (
                          <div className="flex flex-wrap gap-3">
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                toast({ title: "Purchase started", description: `Buying "${listing.title}"`, variant: "success" });
                              }}
                              className="flex items-center gap-2 text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-[#d4af37]/30 text-[#d4af37] hover:bg-[#d4af37]/5 transition-all duration-150"
                            >
                              <ShoppingCart size={12} />
                              Buy Now
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                toast({ title: "Offer flow", description: "Make an offer for this listing", variant: "success" });
                              }}
                              className="flex items-center gap-2 text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150"
                            >
                              Make Offer
                            </button>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                toast({ title: "Message sent", description: `Contacting seller`, variant: "success" });
                              }}
                              className="flex items-center gap-2 text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-white hover:border-white/20 transition-all duration-150"
                            >
                              <MessageSquare size={12} />
                              Contact
                            </button>
                          </div>
                        )}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
              );
            })}
          </div>
        );
        })()}
      </section>
    </>
  );
}


// ── Orders Tab ────────────────────────────────────────────────────────────

function OrdersTab() {
  const [ordersList, setOrders] = useState<OrderResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();

  useEffect(() => {
    orders
      .list()
      .then((res) => setOrders(res.orders || []))
      .catch(() => {
        toast({ title: "API offline", variant: "error" });
        setOrders([]);
      })
      .finally(() => setLoading(false));
  }, [toast]);

  const columns: Column<OrderResponse>[] = [
    {
      id: "id",
      header: "Order",
      cell: (row) => (
        <Tooltip content={row.id}>
          <span className="text-xs font-mono text-neutral-500 cursor-default hover:text-neutral-300 transition-colors duration-150">
            {row.id.slice(0, 8)}...
          </span>
        </Tooltip>
      ),
    },
    {
      id: "amount",
      header: "Amount",
      sortable: true,
      compare: (a, b) => a.amount - b.amount,
      cell: (row) => (
        <span className="text-sm font-light">
          {row.quantity}x @ {formatPrice(row.amount, row.token)}
        </span>
      ),
    },
    {
      id: "buyer",
      header: "Buyer",
      hideBelow: "md",
      cell: (row) => (
        <Tooltip content={row.buyer_did}>
          <span className="text-xs text-neutral-500 font-light cursor-default hover:text-neutral-300 transition-colors duration-150">
            {truncateDid(row.buyer_did)}
          </span>
        </Tooltip>
      ),
    },
    {
      id: "seller",
      header: "Seller",
      hideBelow: "md",
      cell: (row) => (
        <Tooltip content={row.seller_did}>
          <span className="text-xs text-neutral-500 font-light cursor-default hover:text-neutral-300 transition-colors duration-150">
            {truncateDid(row.seller_did)}
          </span>
        </Tooltip>
      ),
    },
    {
      id: "status",
      header: "Status",
      sortable: true,
      compare: (a, b) => a.status.localeCompare(b.status),
      cell: (row) => (
        <span className={cn("text-[10px] font-mono uppercase tracking-wider", statusColor(row.status))}>
          {row.status}
        </span>
      ),
    },
    {
      id: "created",
      header: "Created",
      sortable: true,
      compare: (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      hideBelow: "lg",
      cell: (row) => (
        <Tooltip content={new Date(row.created_at).toLocaleString()}>
          <span className="text-xs font-mono text-neutral-700 cursor-default hover:text-neutral-500 transition-colors duration-150">
            {formatRelativeTime(row.created_at)}
          </span>
        </Tooltip>
      ),
    },
  ];

  return (
    <>
      {loading ? (
        <div className="space-y-px">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="flex items-center gap-4 py-3">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="h-3 w-32" />
              <Skeleton className="h-3 w-24 hidden md:block" />
              <Skeleton className="h-3 w-24 hidden md:block" />
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-14 hidden lg:block" />
            </div>
          ))}
        </div>
      ) : (
        <DataTable
          columns={columns}
          data={ordersList}
          rowKey={(row) => row.id}
          defaultSortId="created"
          defaultSortDir="desc"
          emptyState={
            <EmptyState
              icon={<OrdersIllustration />}
              title="No orders yet"
              description="Purchase a listing to create your first order. All transactions are escrow-backed for safety."
            />
          }
        />
      )}
    </>
  );
}

// ── Disputes Tab ──────────────────────────────────────────────────────────

function DisputesTab() {
  const [disputesList, setDisputes] = useState<DisputeResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();

  useEffect(() => {
    disputes
      .list()
      .then((res) => setDisputes(res.disputes || []))
      .catch(() => {
        toast({ title: "API offline", variant: "error" });
        setDisputes([]);
      })
      .finally(() => setLoading(false));
  }, [toast]);

  const columns: Column<DisputeResponse>[] = [
    {
      id: "id",
      header: "Dispute",
      cell: (row) => (
        <Tooltip content={row.id}>
          <span className="text-xs font-mono text-neutral-500 cursor-default hover:text-neutral-300 transition-colors duration-150">
            {row.id.slice(0, 8)}...
          </span>
        </Tooltip>
      ),
    },
    {
      id: "reason",
      header: "Reason",
      sortable: true,
      compare: (a, b) => a.reason.localeCompare(b.reason),
      cell: (row) => (
        <span className="text-xs text-neutral-400 font-light">
          {row.reason.replace(/([A-Z])/g, " $1").trim()}
        </span>
      ),
    },
    {
      id: "evidence",
      header: "Evidence",
      sortable: true,
      compare: (a, b) => a.evidence_count - b.evidence_count,
      align: "center",
      cell: (row) => (
        <span className="text-xs font-mono text-neutral-500">
          {row.evidence_count}
        </span>
      ),
    },
    {
      id: "initiator",
      header: "Initiator",
      hideBelow: "md",
      cell: (row) => (
        <Tooltip content={row.initiator_did}>
          <span className="text-xs text-neutral-500 font-light cursor-default hover:text-neutral-300 transition-colors duration-150">
            {truncateDid(row.initiator_did)}
          </span>
        </Tooltip>
      ),
    },
    {
      id: "arbiter",
      header: "Arbiter",
      hideBelow: "lg",
      cell: (row) =>
        row.arbiter_did ? (
          <Tooltip content={row.arbiter_did}>
            <span className="text-xs text-neutral-500 font-light cursor-default hover:text-neutral-300 transition-colors duration-150">
              {truncateDid(row.arbiter_did)}
            </span>
          </Tooltip>
        ) : (
          <span className="text-xs text-neutral-700 italic">Unassigned</span>
        ),
    },
    {
      id: "status",
      header: "Status",
      sortable: true,
      compare: (a, b) => a.status.localeCompare(b.status),
      cell: (row) => (
        <span className={cn("text-[10px] font-mono uppercase tracking-wider", statusColor(row.status))}>
          {row.status}
        </span>
      ),
    },
    {
      id: "created",
      header: "Created",
      sortable: true,
      compare: (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime(),
      hideBelow: "lg",
      cell: (row) => (
        <Tooltip content={new Date(row.created_at).toLocaleString()}>
          <span className="text-xs font-mono text-neutral-700 cursor-default hover:text-neutral-500 transition-colors duration-150">
            {formatRelativeTime(row.created_at)}
          </span>
        </Tooltip>
      ),
    },
  ];

  return (
    <>
      {loading ? (
        <div className="space-y-px">
          {Array.from({ length: 2 }).map((_, i) => (
            <div key={i} className="flex items-center gap-4 py-3">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="h-3 w-28" />
              <Skeleton className="h-3 w-10" />
              <Skeleton className="h-3 w-24 hidden md:block" />
              <Skeleton className="h-3 w-24 hidden lg:block" />
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-14 hidden lg:block" />
            </div>
          ))}
        </div>
      ) : (
        <DataTable
          columns={columns}
          data={disputesList}
          rowKey={(row) => row.id}
          defaultSortId="created"
          defaultSortDir="desc"
          emptyState={
            <EmptyState
              icon={<DisputeIllustration />}
              title="No disputes"
              description="Disputes appear when an order is contested. An arbiter reviews evidence and resolves the case."
            />
          }
        />
      )}
    </>
  );
}

// ── Offers Tab ────────────────────────────────────────────────────────────

function OffersTab() {
  const [offersList, setOffers] = useState<OfferResponse[]>([]);
  const [loading, setLoading] = useState(true);
  const { toast } = useToast();

  useEffect(() => {
    offers
      .list()
      .then((res) => setOffers(res.offers || []))
      .catch(() => {
        toast({ title: "API offline", variant: "error" });
        setOffers([]);
      })
      .finally(() => setLoading(false));
  }, [toast]);

  const columns: Column<OfferResponse>[] = [
    {
      id: "id",
      header: "Offer",
      cell: (row) => (
        <Tooltip content={row.id}>
          <span className="text-xs font-mono text-neutral-500 cursor-default hover:text-neutral-300 transition-colors duration-150">
            {row.id.slice(0, 8)}...
          </span>
        </Tooltip>
      ),
    },
    {
      id: "amount",
      header: "Amount",
      sortable: true,
      compare: (a, b) => a.amount - b.amount,
      cell: (row) => (
        <div>
          <span className="text-sm font-light">
            {formatPrice(row.amount, row.token)}
          </span>
          {row.counter_amount !== null && (
            <span className="text-[10px] font-mono text-blue-400 ml-2">
              counter: {formatPrice(row.counter_amount, row.token)}
            </span>
          )}
        </div>
      ),
    },
    {
      id: "buyer",
      header: "Buyer",
      hideBelow: "md",
      cell: (row) => (
        <Tooltip content={row.buyer_did}>
          <span className="text-xs text-neutral-500 font-light cursor-default hover:text-neutral-300 transition-colors duration-150">
            {truncateDid(row.buyer_did)}
          </span>
        </Tooltip>
      ),
    },
    {
      id: "message",
      header: "Message",
      hideBelow: "lg",
      cell: (row) =>
        row.message ? (
          <Tooltip content={row.message}>
            <span className="text-xs text-neutral-600 font-light italic cursor-default hover:text-neutral-400 transition-colors duration-150 truncate max-w-[160px] inline-block">
              &ldquo;{row.message.length > 30 ? `${row.message.slice(0, 30)}...` : row.message}&rdquo;
            </span>
          </Tooltip>
        ) : (
          <span className="text-xs text-neutral-800">&mdash;</span>
        ),
    },
    {
      id: "status",
      header: "Status",
      sortable: true,
      compare: (a, b) => a.status.localeCompare(b.status),
      cell: (row) => (
        <span className={cn("text-[10px] font-mono uppercase tracking-wider", statusColor(row.status))}>
          {row.status}
        </span>
      ),
    },
    {
      id: "expires",
      header: "Expires",
      sortable: true,
      compare: (a, b) => new Date(a.expires_at).getTime() - new Date(b.expires_at).getTime(),
      cell: (row) => (
        <Tooltip content={new Date(row.expires_at).toLocaleString()}>
          <span className="text-xs font-mono text-neutral-700 cursor-default hover:text-neutral-500 transition-colors duration-150">
            {formatDate(row.expires_at)}
          </span>
        </Tooltip>
      ),
    },
  ];

  return (
    <>
      {loading ? (
        <div className="space-y-px">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="flex items-center gap-4 py-3">
              <Skeleton className="h-3 w-20" />
              <Skeleton className="h-3 w-28" />
              <Skeleton className="h-3 w-24 hidden md:block" />
              <Skeleton className="h-3 w-20 hidden lg:block" />
              <Skeleton className="h-3 w-16" />
              <Skeleton className="h-3 w-20" />
            </div>
          ))}
        </div>
      ) : (
        <DataTable
          columns={columns}
          data={offersList}
          rowKey={(row) => row.id}
          defaultSortId="expires"
          defaultSortDir="asc"
          emptyState={
            <EmptyState
              icon={<OffersIllustration />}
              title="No offers yet"
              description="Make an offer on a listing to negotiate a better price. Sellers can accept, reject, or counter."
            />
          }
        />
      )}
    </>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────

export default function MarketplacePage() {
  const [tab, setTab] = useState<Tab>("listings");

  usePageShortcuts({
    n: () => {
      setTab("listings");
      // Small delay to let ListingsTab mount if switching tabs
      setTimeout(() => {
        window.dispatchEvent(new CustomEvent("nous:create-listing"));
      }, 50);
    },
  });

  return (
    <div className="p-4 sm:p-8 max-w-5xl">
      <PageHeader title="Marketplace" subtitle="P2P. Reputation-gated. Escrow-backed." />

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
