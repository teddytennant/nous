"use client";

import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import { node, type HealthResponse } from "@/lib/api";

type ConnectionState = "connecting" | "online" | "offline";

interface ConnectionContextValue {
  status: ConnectionState;
  health: HealthResponse | null;
  retry: () => void;
}

const ConnectionContext = createContext<ConnectionContextValue>({
  status: "connecting",
  health: null,
  retry: () => {},
});

export function useConnection() {
  return useContext(ConnectionContext);
}

export function ConnectionProvider({ children }: { children: ReactNode }) {
  const [status, setStatus] = useState<ConnectionState>("connecting");
  const [health, setHealth] = useState<HealthResponse | null>(null);

  const check = useCallback(async () => {
    try {
      const h = await node.health();
      setHealth(h);
      setStatus("online");
    } catch {
      setHealth(null);
      setStatus("offline");
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      try {
        const h = await node.health();
        if (!cancelled) {
          setHealth(h);
          setStatus("online");
        }
      } catch {
        if (!cancelled) {
          setHealth(null);
          setStatus("offline");
        }
      }
    };
    run();
    const interval = setInterval(run, 15000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  return (
    <ConnectionContext.Provider value={{ status, health, retry: check }}>
      {children}
    </ConnectionContext.Provider>
  );
}
