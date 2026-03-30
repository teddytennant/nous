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

  const apiUrl =
    process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080/api/v1";

  return (
    <ConnectionContext.Provider value={{ status, health, retry: check }}>
      {status !== "online" && (
        <div className="px-4 py-3 bg-white/[0.02] border-b border-white/[0.06] flex items-center justify-between">
          <div className="flex items-center gap-3">
            <span
              className={`inline-block w-2 h-2 rounded-full ${status === "connecting" ? "bg-yellow-500 animate-pulse" : "bg-red-500"}`}
            />
            <p className="text-xs font-mono text-neutral-400">
              {status === "connecting"
                ? "Connecting to API..."
                : `Unable to reach API at ${apiUrl}`}
            </p>
          </div>
          <button
            onClick={check}
            className="text-[10px] font-mono uppercase tracking-wider text-neutral-500 hover:text-[#d4af37] transition-colors duration-150"
          >
            Retry
          </button>
        </div>
      )}
      {children}
    </ConnectionContext.Provider>
  );
}
