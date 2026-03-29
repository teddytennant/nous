const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api/v1";

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });

  if (!res.ok) {
    const error = await res.json().catch(() => ({ error: { message: res.statusText } }));
    throw new Error(error.error?.message || `Request failed: ${res.status}`);
  }

  return res.json();
}

// ── Node ───────────────────────────────────────────────────────────────────

export interface HealthResponse {
  status: string;
  version: string;
  uptime_ms: number;
}

export interface NodeInfo {
  protocol: string;
  version: string;
  features: string[];
}

export const node = {
  health: () => request<HealthResponse>("/health"),
  info: () => request<NodeInfo>("/node"),
};

// ── Social ─────────────────────────────────────────────────────────────────

export interface FeedEvent {
  id: string;
  pubkey: string;
  created_at: string;
  kind: number;
  content: string;
  tags: string[][];
}

export interface FeedResponse {
  events: FeedEvent[];
  count: number;
}

export interface CreatePostRequest {
  author_did: string;
  content: string;
  reply_to?: string;
  hashtags?: string[];
}

export const social = {
  feed: (params?: { author?: string; limit?: number }) => {
    const query = new URLSearchParams();
    if (params?.author) query.set("author", params.author);
    if (params?.limit) query.set("limit", String(params.limit));
    const qs = query.toString();
    return request<FeedResponse>(`/feed${qs ? `?${qs}` : ""}`);
  },

  createPost: (post: CreatePostRequest) =>
    request<FeedEvent>("/events", {
      method: "POST",
      body: JSON.stringify(post),
    }),

  deleteEvent: (eventId: string) =>
    request<void>(`/events/${eventId}`, { method: "DELETE" }),

  follow: (followerDid: string, targetDid: string) =>
    request<void>("/follow", {
      method: "POST",
      body: JSON.stringify({ follower_did: followerDid, target_did: targetDid }),
    }),

  unfollow: (followerDid: string, targetDid: string) =>
    request<void>("/unfollow", {
      method: "POST",
      body: JSON.stringify({ follower_did: followerDid, target_did: targetDid }),
    }),
};

// ── Governance ─────────────────────────────────────────────────────────────

export interface DaoResponse {
  id: string;
  name: string;
  description: string;
  founder_did: string;
  member_count: number;
  created_at: string;
}

export interface DaoListResponse {
  daos: DaoResponse[];
  count: number;
}

export interface ProposalResponse {
  id: string;
  dao_id: string;
  title: string;
  description: string;
  proposer_did: string;
  status: string;
  created_at: string;
  voting_starts: string;
  voting_ends: string;
  quorum: number;
  threshold: number;
}

export interface ProposalListResponse {
  proposals: ProposalResponse[];
  count: number;
}

export interface VoteResultResponse {
  proposal_id: string;
  votes_for: number;
  votes_against: number;
  votes_abstain: number;
  total_voters: number;
  passed: boolean;
}

export const governance = {
  listDaos: () => request<DaoListResponse>("/daos"),

  createDao: (founderDid: string, name: string, description: string) =>
    request<DaoResponse>("/daos", {
      method: "POST",
      body: JSON.stringify({ founder_did: founderDid, name, description }),
    }),

  getDao: (daoId: string) => request<DaoResponse>(`/daos/${daoId}`),

  listProposals: (daoId?: string) => {
    const query = daoId ? `?dao_id=${daoId}` : "";
    return request<ProposalListResponse>(`/proposals${query}`);
  },

  getProposal: (proposalId: string) =>
    request<ProposalResponse>(`/proposals/${proposalId}`),

  getTally: (proposalId: string) =>
    request<VoteResultResponse>(`/votes/${proposalId}`),
};

// ── Marketplace ────────────────────────────────────────────────────────────

export interface ListingResponse {
  id: string;
  seller_did: string;
  title: string;
  description: string;
  category: string;
  price_token: string;
  price_amount: number;
  quantity: number;
  status: string;
  created_at: string;
  tags: string[];
  images: string[];
}

export interface ListingListResponse {
  listings: ListingResponse[];
  count: number;
}

export interface SellerRating {
  seller_did: string;
  total_reviews: number;
  average_rating: number;
  verified_reviews: number;
}

export const marketplace = {
  search: (params?: { text?: string; category?: string; limit?: number }) => {
    const query = new URLSearchParams();
    if (params?.text) query.set("text", params.text);
    if (params?.category) query.set("category", params.category);
    if (params?.limit) query.set("limit", String(params.limit));
    const qs = query.toString();
    return request<ListingListResponse>(`/listings${qs ? `?${qs}` : ""}`);
  },

  getListing: (listingId: string) =>
    request<ListingResponse>(`/listings/${listingId}`),

  getSellerRating: (sellerDid: string) =>
    request<SellerRating>(`/sellers/${sellerDid}/rating`),
};
