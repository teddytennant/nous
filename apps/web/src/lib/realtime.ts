/**
 * Real-time event stream client using Server-Sent Events.
 *
 * Connects to the Nous API's SSE endpoint (`/api/v1/events`) and dispatches
 * typed events to registered listeners. Automatically reconnects on failure.
 *
 * Usage:
 *   import { realtime } from "@/lib/realtime";
 *   realtime.on("new_post", (data) => console.log(data));
 *   realtime.connect();
 */

const API_BASE =
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api/v1";

// ── Event types matching the server's RealtimeEvent enum ──────────

export interface NewPostEvent {
  id: string;
  author: string;
  content: string;
}

export interface NewMessageEvent {
  channel_id: string;
  sender: string;
  content: string;
}

export interface VoteCastEvent {
  proposal_id: string;
  voter: string;
}

export interface DaoCreatedEvent {
  id: string;
  name: string;
}

export interface ProposalCreatedEvent {
  id: string;
  title: string;
  dao_id: string;
}

export interface TransferEvent {
  from: string;
  to: string;
  amount: string;
  token: string;
}

export interface ListingUpdateEvent {
  id: string;
  title: string;
}

export type EventMap = {
  new_post: NewPostEvent;
  new_message: NewMessageEvent;
  vote_cast: VoteCastEvent;
  dao_created: DaoCreatedEvent;
  proposal_created: ProposalCreatedEvent;
  transfer: TransferEvent;
  listing_update: ListingUpdateEvent;
};

export type EventType = keyof EventMap;
type Listener<T extends EventType> = (data: EventMap[T]) => void;

// ── Client ────────────────────────────────────────────────────────

class RealtimeClient {
  private source: EventSource | null = null;
  private listeners: Map<string, Set<Listener<EventType>>> = new Map();
  private reconnectMs = 3000;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  /** Register a listener for a specific event type. Returns an unsubscribe function. */
  on<T extends EventType>(type: T, listener: Listener<T>): () => void {
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    const set = this.listeners.get(type)!;
    set.add(listener as Listener<EventType>);

    // If already connected, register the SSE listener
    if (this.source) {
      this.source.addEventListener(type, this.handleSseEvent(type));
    }

    return () => {
      set.delete(listener as Listener<EventType>);
      if (set.size === 0) {
        this.listeners.delete(type);
      }
    };
  }

  /** Connect to the SSE stream. Safe to call multiple times. */
  connect(): void {
    if (typeof window === "undefined") return; // SSR guard
    if (this.source) return;

    this.source = new EventSource(`${API_BASE}/events`);

    // Register handlers for all known event types
    for (const type of this.listeners.keys()) {
      this.source.addEventListener(type, this.handleSseEvent(type as EventType));
    }

    this.source.onerror = () => {
      this.disconnect();
      this.scheduleReconnect();
    };
  }

  /** Disconnect and stop receiving events. */
  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.source) {
      this.source.close();
      this.source = null;
    }
  }

  /** Whether the client is currently connected. */
  get connected(): boolean {
    return this.source?.readyState === EventSource.OPEN;
  }

  private handleSseEvent(type: EventType) {
    return (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        const set = this.listeners.get(type);
        if (set) {
          for (const listener of set) {
            listener(data);
          }
        }
      } catch {
        // Malformed event data — skip
      }
    };
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, this.reconnectMs);
  }
}

/** Singleton real-time client. */
export const realtime = new RealtimeClient();
