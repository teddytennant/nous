"use client";

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useEffect,
  useRef,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";
import { CheckCircle2, XCircle, Info, X } from "lucide-react";

// ── Types ────────────────────────────────────────────────────────────────

interface Toast {
  id: string;
  title: string;
  description?: string;
  variant?: "default" | "success" | "error" | "info";
  action?: { label: string; onClick: () => void };
  duration?: number;
  exiting?: boolean;
  createdAt: number;
}

type ToastInput = Omit<Toast, "id" | "exiting" | "createdAt">;

interface ToastContextValue {
  toast: (t: ToastInput) => string;
  dismiss: (id: string) => void;
}

const ToastContext = createContext<ToastContextValue>({
  toast: () => "",
  dismiss: () => {},
});

export function useToast() {
  return useContext(ToastContext);
}

// ── Constants ────────────────────────────────────────────────────────────

const DEFAULT_DURATION = 3000;
const EXIT_DURATION = 300;
const MAX_VISIBLE = 3;

// ── Variant config ───────────────────────────────────────────────────────

const variantConfig = {
  default: {
    icon: null,
    titleClass: "text-white",
    accentClass: "bg-white/20",
  },
  success: {
    icon: CheckCircle2,
    titleClass: "text-emerald-400",
    accentClass: "bg-emerald-500",
  },
  error: {
    icon: XCircle,
    titleClass: "text-red-400",
    accentClass: "bg-red-500",
  },
  info: {
    icon: Info,
    titleClass: "text-blue-400",
    accentClass: "bg-blue-500",
  },
};

// ── Progress bar ─────────────────────────────────────────────────────────

function ToastProgress({
  duration,
  paused,
  accentClass,
}: {
  duration: number;
  paused: boolean;
  accentClass: string;
}) {
  return (
    <div className="absolute bottom-0 left-0 right-0 h-[2px] bg-white/[0.04] overflow-hidden rounded-b-md">
      <div
        className={cn("h-full toast-progress", accentClass)}
        style={{
          animationDuration: `${duration}ms`,
          animationPlayState: paused ? "paused" : "running",
        }}
      />
    </div>
  );
}

// ── Single toast ─────────────────────────────────────────────────────────

function ToastItem({
  toast: t,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: string) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const variant = t.variant ?? "default";
  const config = variantConfig[variant];
  const Icon = config.icon;
  const duration = t.duration ?? DEFAULT_DURATION;

  return (
    <div
      role="status"
      aria-live="polite"
      className={cn(
        "relative bg-neutral-900 border border-white/[0.08] rounded-md shadow-2xl max-w-sm w-full overflow-hidden",
        t.exiting ? "toast-exit" : "toast-enter",
      )}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div className="flex items-start gap-3 px-4 py-3 pr-9">
        {Icon && (
          <Icon
            className={cn("w-4 h-4 mt-0.5 shrink-0", config.titleClass)}
            aria-hidden="true"
          />
        )}
        <div className="flex-1 min-w-0">
          <p className={cn("text-sm font-medium", config.titleClass)}>
            {t.title}
          </p>
          {t.description && (
            <p className="text-xs text-neutral-500 font-light mt-1">
              {t.description}
            </p>
          )}
          {t.action && (
            <button
              type="button"
              onClick={() => {
                t.action!.onClick();
                onDismiss(t.id);
              }}
              className="mt-2 text-xs font-medium text-[#d4af37] hover:text-[#e5c548] transition-colors duration-150"
            >
              {t.action.label}
            </button>
          )}
        </div>
      </div>

      {/* Dismiss button */}
      <button
        type="button"
        onClick={() => onDismiss(t.id)}
        className="absolute top-2.5 right-2.5 p-1 rounded-sm text-neutral-600 hover:text-neutral-300 hover:bg-white/[0.06] transition-all duration-150"
        aria-label="Dismiss"
      >
        <X className="w-3 h-3" />
      </button>

      {/* Progress bar */}
      <ToastProgress
        duration={duration}
        paused={hovered || !!t.exiting}
        accentClass={config.accentClass}
      />
    </div>
  );
}

// ── Provider ─────────────────────────────────────────────────────────────

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const timersRef = useRef<Map<string, { exit: ReturnType<typeof setTimeout>; remove: ReturnType<typeof setTimeout> }>>(new Map());

  // Check reduced motion preference
  const reducedMotion = useRef(false);
  useEffect(() => {
    reducedMotion.current = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }, []);

  const dismiss = useCallback((id: string) => {
    // Clear existing timers for this toast
    const existing = timersRef.current.get(id);
    if (existing) {
      clearTimeout(existing.exit);
      clearTimeout(existing.remove);
      timersRef.current.delete(id);
    }

    if (reducedMotion.current) {
      // Skip exit animation
      setToasts((prev) => prev.filter((x) => x.id !== id));
    } else {
      // Mark as exiting, then remove after animation
      setToasts((prev) =>
        prev.map((x) => (x.id === id ? { ...x, exiting: true } : x)),
      );
      setTimeout(() => {
        setToasts((prev) => prev.filter((x) => x.id !== id));
      }, EXIT_DURATION);
    }
  }, []);

  const addToast = useCallback(
    (input: ToastInput): string => {
      const id = crypto.randomUUID();
      const duration = input.duration ?? DEFAULT_DURATION;

      setToasts((prev) => [...prev, { ...input, id, createdAt: Date.now() }]);

      // Schedule auto-dismiss
      const exitTimer = setTimeout(() => {
        setToasts((prev) =>
          prev.map((x) => (x.id === id ? { ...x, exiting: true } : x)),
        );
      }, duration);

      const removeTimer = setTimeout(() => {
        setToasts((prev) => prev.filter((x) => x.id !== id));
        timersRef.current.delete(id);
      }, duration + EXIT_DURATION);

      timersRef.current.set(id, { exit: exitTimer, remove: removeTimer });

      return id;
    },
    [],
  );

  // Cleanup timers on unmount
  useEffect(() => {
    return () => {
      for (const { exit, remove } of timersRef.current.values()) {
        clearTimeout(exit);
        clearTimeout(remove);
      }
    };
  }, []);

  // Only show the most recent MAX_VISIBLE toasts
  const visibleToasts = toasts.slice(-MAX_VISIBLE);

  return (
    <ToastContext.Provider value={{ toast: addToast, dismiss }}>
      {children}
      {visibleToasts.length > 0 && (
        <div
          className="fixed bottom-20 right-4 left-4 md:left-auto md:bottom-6 md:right-6 z-[100] flex flex-col gap-2 items-end"
          aria-label="Notifications"
        >
          {visibleToasts.map((t) => (
            <ToastItem key={t.id} toast={t} onDismiss={dismiss} />
          ))}
        </div>
      )}
    </ToastContext.Provider>
  );
}
