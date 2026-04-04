"use client";

import {
  useCallback,
  useEffect,
  useRef,
  type ReactNode,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { cn } from "@/lib/utils";

interface SheetProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  /** Width class — defaults to "w-full sm:w-[480px] lg:w-[540px]" */
  className?: string;
  /** Side to slide in from — defaults to "right" */
  side?: "right" | "left";
}

export function Sheet({
  open,
  onClose,
  children,
  className,
  side = "right",
}: SheetProps) {
  const sheetRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  // Trap focus inside the sheet when open
  useEffect(() => {
    if (open) {
      previousFocus.current = document.activeElement as HTMLElement;
      // Small delay to let the animation start, then focus the sheet
      const timer = setTimeout(() => {
        sheetRef.current?.focus();
      }, 50);
      return () => clearTimeout(timer);
    } else if (previousFocus.current) {
      previousFocus.current.focus();
      previousFocus.current = null;
    }
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: globalThis.KeyboardEvent) {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  // Lock body scroll when open
  useEffect(() => {
    if (open) {
      const prev = document.body.style.overflow;
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = prev;
      };
    }
  }, [open]);

  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose]
  );

  const handleKeyDown = useCallback(
    (e: ReactKeyboardEvent<HTMLDivElement>) => {
      // Tab trapping
      if (e.key === "Tab" && sheetRef.current) {
        const focusable = sheetRef.current.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusable.length === 0) return;
        const first = focusable[0];
        const last = focusable[focusable.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    },
    []
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 sheet-backdrop"
      onClick={handleBackdropClick}
      aria-modal="true"
      role="dialog"
    >
      <div
        ref={sheetRef}
        tabIndex={-1}
        onKeyDown={handleKeyDown}
        className={cn(
          "fixed top-0 bottom-0 bg-black border-white/[0.06] overflow-y-auto outline-none sheet-panel",
          side === "right"
            ? "right-0 border-l sheet-slide-left"
            : "left-0 border-r sheet-slide-right",
          className || "w-full sm:w-[480px] lg:w-[540px]"
        )}
      >
        {children}
      </div>
    </div>
  );
}

interface SheetHeaderProps {
  children: ReactNode;
  onClose: () => void;
  className?: string;
}

export function SheetHeader({ children, onClose, className }: SheetHeaderProps) {
  return (
    <div
      className={cn(
        "sticky top-0 z-10 flex items-center justify-between px-6 py-4 bg-black/90 backdrop-blur-xl border-b border-white/[0.06]",
        className
      )}
    >
      <div className="flex-1 min-w-0">{children}</div>
      <button
        onClick={onClose}
        className="shrink-0 ml-4 p-1.5 rounded-md text-neutral-600 hover:text-white hover:bg-white/[0.04] transition-colors duration-150"
        aria-label="Close"
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
        >
          <path d="M4 4l8 8M12 4l-8 8" />
        </svg>
      </button>
    </div>
  );
}

export function SheetBody({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn("px-6 py-6", className)}>{children}</div>;
}
