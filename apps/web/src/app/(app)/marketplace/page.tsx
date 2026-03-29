"use client";

import { useCallback, useEffect, useState, startTransition } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { marketplace, type ListingResponse } from "@/lib/api";
import { cn } from "@/lib/utils";

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

export default function MarketplacePage() {
  const [listings, setListings] = useState<ListingResponse[]>([]);
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
    <div className="p-8 max-w-5xl">
      <header className="mb-16">
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-extralight tracking-[-0.03em] mb-2">
              Marketplace
            </h1>
            <p className="text-sm text-neutral-500 font-light">
              P2P. Reputation-gated. Escrow-backed.
            </p>
          </div>
          <button
            onClick={() => setCreating(!creating)}
            className="text-xs font-mono uppercase tracking-wider px-5 py-2.5 border border-white/10 text-neutral-500 hover:text-[#d4af37] hover:border-[#d4af37]/30 transition-all duration-150"
          >
            {creating ? "Cancel" : "New Listing"}
          </button>
        </div>
      </header>

      {error && (
        <p className="text-xs text-red-500 mb-6">{error}</p>
      )}

      {/* Create listing form */}
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

      {/* Search and filters */}
      <section className="mb-12">
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

      {/* Listings */}
      <section>
        {listings.length === 0 ? (
          <div className="py-16 text-center">
            <p className="text-sm text-neutral-700 font-light">
              No listings found
            </p>
            <p className="text-[10px] font-mono text-neutral-800 mt-2">
              Create one to get started
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-px bg-white/[0.03]">
            {listings.map((listing) => (
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
                        listing.status === "Active"
                          ? "text-[#d4af37]"
                          : "text-neutral-600"
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
                    {listing.seller_did.slice(0, 16)}...
                    {listing.seller_did.slice(-6)}
                  </p>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
