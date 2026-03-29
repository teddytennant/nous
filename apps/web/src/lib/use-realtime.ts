"use client";

import { useEffect, useRef, useCallback, useState } from "react";
import { realtime, type EventMap, type EventType } from "./realtime";

/**
 * React hook for subscribing to real-time server events.
 *
 * Connects to the SSE stream on mount, disconnects on unmount.
 * Each call with a unique event type registers a listener that is
 * automatically cleaned up when the component unmounts.
 *
 * Usage:
 *   const posts = useRealtime("new_post", (data) => {
 *     setPosts(prev => [data, ...prev]);
 *   });
 */
export function useRealtime<T extends EventType>(
  type: T,
  handler: (data: EventMap[T]) => void
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    realtime.connect();

    const unsubscribe = realtime.on(type, (data) => {
      handlerRef.current(data);
    });

    return () => {
      unsubscribe();
    };
  }, [type]);
}

/**
 * Hook that returns the current connection status of the realtime client.
 * Re-checks every 2 seconds.
 */
export function useRealtimeStatus(): boolean {
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    realtime.connect();

    const check = () => setConnected(realtime.connected);
    check();

    const interval = setInterval(check, 2000);
    return () => clearInterval(interval);
  }, []);

  return connected;
}

/**
 * Hook that collects real-time events into an array state.
 * New events are prepended (newest first). Limited to `maxItems`.
 *
 * Usage:
 *   const messages = useRealtimeList("new_message", 100);
 */
export function useRealtimeList<T extends EventType>(
  type: T,
  maxItems = 100
): EventMap[T][] {
  const [items, setItems] = useState<EventMap[T][]>([]);

  useRealtime(type, useCallback((data: EventMap[T]) => {
    setItems((prev) => {
      const next = [data, ...prev];
      return next.length > maxItems ? next.slice(0, maxItems) : next;
    });
  }, [maxItems]));

  return items;
}
