"use client";

import {
  createContext,
  useContext,
  useState,
  useCallback,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";

interface Toast {
  id: string;
  title: string;
  description?: string;
  variant?: "default" | "success" | "error";
  exiting?: boolean;
}

interface ToastContextValue {
  toast: (t: Omit<Toast, "id" | "exiting">) => void;
}

const ToastContext = createContext<ToastContextValue>({
  toast: () => {},
});

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = useCallback((t: Omit<Toast, "id" | "exiting">) => {
    const id = crypto.randomUUID();
    setToasts((prev) => [...prev, { ...t, id }]);
    setTimeout(() => {
      setToasts((prev) =>
        prev.map((x) => (x.id === id ? { ...x, exiting: true } : x)),
      );
    }, 2500);
    setTimeout(() => {
      setToasts((prev) => prev.filter((x) => x.id !== id));
    }, 3000);
  }, []);

  return (
    <ToastContext.Provider value={{ toast: addToast }}>
      {children}
      {toasts.length > 0 && (
        <div className="fixed bottom-20 right-4 left-4 md:left-auto md:bottom-6 md:right-6 z-[100] flex flex-col gap-2 items-end">
          {toasts.map((t) => (
            <div
              key={t.id}
              className={cn(
                "bg-neutral-900 border border-white/[0.08] px-4 py-3 rounded-md shadow-2xl max-w-sm",
                t.exiting ? "toast-exit" : "toast-enter",
              )}
            >
              <p
                className={cn(
                  "text-sm font-medium",
                  t.variant === "error"
                    ? "text-red-400"
                    : t.variant === "success"
                      ? "text-emerald-400"
                      : "text-white",
                )}
              >
                {t.title}
              </p>
              {t.description && (
                <p className="text-xs text-neutral-500 font-light mt-1">
                  {t.description}
                </p>
              )}
            </div>
          ))}
        </div>
      )}
    </ToastContext.Provider>
  );
}
