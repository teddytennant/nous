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

  createListing: (listing: {
    seller_did: string;
    title: string;
    description: string;
    category: string;
    price_token: string;
    price_amount: number;
    tags?: string[];
  }) =>
    request<ListingResponse>("/listings", {
      method: "POST",
      body: JSON.stringify(listing),
    }),
};

// ── Messaging ─────────────────────────────────────────────────────────────

export interface ChannelResponse {
  id: string;
  kind: string;
  name: string | null;
  members: string[];
  created_at: string;
}

export interface MessageResponse {
  id: string;
  channel_id: string;
  sender: string;
  content: string;
  reply_to: string | null;
  timestamp: string;
}

export const messaging = {
  createChannel: (data: {
    creator_did: string;
    kind: string;
    name?: string;
    peer_did?: string;
    members?: string[];
  }) =>
    request<ChannelResponse>("/channels", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  listChannels: (did: string) =>
    request<ChannelResponse[]>(`/channels?did=${encodeURIComponent(did)}`),

  getChannel: (channelId: string) =>
    request<ChannelResponse>(`/channels/${channelId}`),

  sendMessage: (data: {
    channel_id: string;
    sender_did: string;
    content: string;
    reply_to?: string;
  }) =>
    request<MessageResponse>("/messages", {
      method: "POST",
      body: JSON.stringify(data),
    }),

  getMessages: (channelId: string, limit?: number) => {
    const qs = limit ? `?limit=${limit}` : "";
    return request<MessageResponse[]>(`/channels/${channelId}/messages${qs}`);
  },

  deleteMessage: (messageId: string) =>
    request<void>(`/messages/${messageId}`, { method: "DELETE" }),
};

// ── Identity ──────────────────────────────────────────────────────────────

export interface IdentityResponse {
  did: string;
  display_name: string | null;
  signing_key_type: string;
  exchange_key_type: string;
}

export interface DocumentResponse {
  did: string;
  document: Record<string, unknown>;
}

export interface CredentialResponse {
  id: string;
  credential_type: string[];
  issuer: string;
  subject: string;
  issuance_date: string;
  expiration_date: string | null;
  expired: boolean;
  claims: Record<string, unknown>;
}

export interface ReputationResponse {
  did: string;
  total_score: number;
  scores: Record<string, number>;
  event_count: number;
}

export const identity = {
  create: (displayName?: string) =>
    request<IdentityResponse>("/identities", {
      method: "POST",
      body: JSON.stringify({ display_name: displayName }),
    }),

  get: (did: string) => request<IdentityResponse>(`/identities/${did}`),

  getDocument: (did: string) =>
    request<DocumentResponse>(`/identities/${did}/document`),

  listCredentials: (did: string) =>
    request<CredentialResponse[]>(`/identities/${did}/credentials`),

  issueCredential: (
    did: string,
    data: {
      subject_did: string;
      issuer_did: string;
      credential_type: string;
      claims: Record<string, unknown>;
    }
  ) =>
    request<CredentialResponse>(`/identities/${did}/credentials`, {
      method: "POST",
      body: JSON.stringify(data),
    }),

  getReputation: (did: string) =>
    request<ReputationResponse>(`/identities/${did}/reputation`),
};
