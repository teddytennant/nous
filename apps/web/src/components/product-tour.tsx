"use client";

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import {
  ArrowRight,
  ArrowLeft,
  X,
  LayoutDashboard,
  Search,
  Bell,
  Zap,
  Command,
  Settings,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

// ── Storage ─────────────────────────────────────────────────────────────

const TOUR_KEY = "nous_tour_completed";

const emptySubscribe = () => () => {};

function getTourCompleted(): boolean {
  if (typeof window === "undefined") return true;
  return localStorage.getItem(TOUR_KEY) === "true";
}

function useTourCompleted(): boolean {
  return useSyncExternalStore(emptySubscribe, getTourCompleted, () => true);
}

export function markTourCompleted() {
  localStorage.setItem(TOUR_KEY, "true");
}

export function resetTour() {
  localStorage.removeItem(TOUR_KEY);
}

// ── Tour Steps ──────────────────────────────────────────────────────────

interface TourStep {
  /** CSS selector for the element to highlight */
  target: string;
  /** Title of this step */
  title: string;
  /** Description text */
  description: string;
  /** Icon to display */
  icon: LucideIcon;
  /** Preferred popover position relative to the target */
  position: "top" | "bottom" | "left" | "right";
}

const TOUR_STEPS: TourStep[] = [
  {
    target: "[data-tour='sidebar']",
    title: "Navigate Your Node",
    description:
      "The sidebar groups 11 subsystems into logical sections. Collapse sections you don't need. The active page is highlighted in gold.",
    icon: LayoutDashboard,
    position: "right",
  },
  {
    target: "[data-tour='search']",
    title: "Command Palette",
    description:
      "Press \u2318K (or Ctrl+K) to instantly jump to any page or action. Search pages, create posts, send tokens — all from the keyboard.",
    icon: Search,
    position: "bottom",
  },
  {
    target: "[data-tour='notifications']",
    title: "Notifications",
    description:
      "Activity from across all subsystems appears here — governance votes, new messages, payments, and more. Configure which categories you see in Settings.",
    icon: Bell,
    position: "bottom",
  },
  {
    target: "[data-tour='stats']",
    title: "System at a Glance",
    description:
      "The dashboard shows real-time status: node health, uptime, active DAOs, and enabled features. Sparklines show trends over time.",
    icon: Zap,
    position: "bottom",
  },
  {
    target: "[data-tour='shortcuts']",
    title: "Quick Actions & Shortcuts",
    description:
      "Jump into common tasks with one click. Press ? anywhere to see keyboard shortcuts — G then D for Dashboard, G then S for Social, J/K to navigate lists.",
    icon: Command,
    position: "top",
  },
  {
    target: "[data-tour='user']",
    title: "Your Identity",
    description:
      "Your DID (Decentralized Identifier) lives here. It's your self-sovereign identity — cryptographic keys generated on your device. No one can revoke it.",
    icon: Settings,
    position: "right",
  },
];

// ── Spotlight Geometry ──────────────────────────────────────────��───────

interface Rect {
  top: number;
  left: number;
  width: number;
  height: number;
}

const PADDING = 8;
const POPOVER_GAP = 16;
const POPOVER_WIDTH = 320;

function getElementRect(selector: string): Rect | null {
  const el = document.querySelector(selector);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return {
    top: r.top + window.scrollY,
    left: r.left + window.scrollX,
    width: r.width,
    height: r.height,
  };
}

function computePopoverPosition(
  rect: Rect,
  position: TourStep["position"],
): { top: number; left: number; actualPosition: TourStep["position"] } {
  const vw = window.innerWidth;
  const vh = window.innerHeight;

  // Convert to viewport-relative for bounds checking
  const viewTop = rect.top - window.scrollY;
  const viewLeft = rect.left - window.scrollX;

  let top: number;
  let left: number;
  let actual = position;

  switch (position) {
    case "bottom":
      top = rect.top + rect.height + PADDING + POPOVER_GAP;
      left = rect.left + rect.width / 2 - POPOVER_WIDTH / 2;
      // If overflows bottom, try top
      if (viewTop + rect.height + POPOVER_GAP + 200 > vh) {
        top = rect.top - PADDING - POPOVER_GAP - 160;
        actual = "top";
      }
      break;
    case "top":
      top = rect.top - PADDING - POPOVER_GAP - 160;
      left = rect.left + rect.width / 2 - POPOVER_WIDTH / 2;
      if (viewTop - POPOVER_GAP - 160 < 0) {
        top = rect.top + rect.height + PADDING + POPOVER_GAP;
        actual = "bottom";
      }
      break;
    case "right":
      top = rect.top + rect.height / 2 - 80;
      left = rect.left + rect.width + PADDING + POPOVER_GAP;
      if (viewLeft + rect.width + POPOVER_GAP + POPOVER_WIDTH > vw) {
        left = rect.left - PADDING - POPOVER_GAP - POPOVER_WIDTH;
        actual = "left";
      }
      break;
    case "left":
      top = rect.top + rect.height / 2 - 80;
      left = rect.left - PADDING - POPOVER_GAP - POPOVER_WIDTH;
      if (viewLeft - POPOVER_GAP - POPOVER_WIDTH < 0) {
        left = rect.left + rect.width + PADDING + POPOVER_GAP;
        actual = "right";
      }
      break;
  }

  // Clamp horizontal
  left = Math.max(16, Math.min(left, vw - POPOVER_WIDTH - 16));
  // Clamp vertical
  top = Math.max(16 + window.scrollY, top);

  return { top, left, actualPosition: actual };
}

// ���─ Component ───────────────────────────────────────────────────────────

export function ProductTour() {
  const completed = useTourCompleted();
  const [active, setActive] = useState(false);
  const [stepIndex, setStepIndex] = useState(0);
  const [targetRect, setTargetRect] = useState<Rect | null>(null);
  const [exiting, setExiting] = useState(false);
  const rafRef = useRef<number>(0);

  // Start tour automatically if not completed
  useEffect(() => {
    if (!completed) {
      // Small delay to let the app render first
      const timer = setTimeout(() => setActive(true), 800);
      return () => clearTimeout(timer);
    }
  }, [completed]);

  // Measure target element on step change and resize/scroll
  useEffect(() => {
    if (!active) return;

    function measure() {
      const s = TOUR_STEPS[stepIndex];
      if (!s) return;
      const rect = getElementRect(s.target);
      setTargetRect(rect);
    }

    measure();

    function onResize() {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(measure);
    }

    window.addEventListener("resize", onResize, { passive: true });
    window.addEventListener("scroll", onResize, { passive: true });
    return () => {
      window.removeEventListener("resize", onResize);
      window.removeEventListener("scroll", onResize);
      cancelAnimationFrame(rafRef.current);
    };
  }, [active, stepIndex]);

  const handleDismiss = useCallback(() => {
    setExiting(true);
    setTimeout(() => {
      setActive(false);
      setExiting(false);
      setStepIndex(0);
      markTourCompleted();
    }, 200);
  }, []);

  const handleNext = useCallback(() => {
    if (stepIndex < TOUR_STEPS.length - 1) {
      setStepIndex((i) => i + 1);
    } else {
      handleDismiss();
    }
  }, [stepIndex, handleDismiss]);

  const handleBack = useCallback(() => {
    if (stepIndex > 0) {
      setStepIndex((i) => i - 1);
    }
  }, [stepIndex]);

  // Keyboard navigation
  useEffect(() => {
    if (!active) return;

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        handleDismiss();
      } else if (e.key === "ArrowRight" || e.key === "Enter") {
        e.preventDefault();
        handleNext();
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        handleBack();
      }
    }

    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () =>
      window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [active, handleDismiss, handleNext, handleBack]);

  const step = TOUR_STEPS[stepIndex];
  const Icon = step?.icon;

  const popoverPos = useMemo(() => {
    if (!targetRect || !step) return null;
    return computePopoverPosition(targetRect, step.position);
  }, [targetRect, step]);

  if (!active || !step) return null;

  // SVG mask: full screen with a rounded-rect cutout over the target
  const spotlightX = targetRect ? targetRect.left - PADDING : -9999;
  const spotlightY = targetRect ? targetRect.top - PADDING : -9999;
  const spotlightW = targetRect ? targetRect.width + PADDING * 2 : 0;
  const spotlightH = targetRect ? targetRect.height + PADDING * 2 : 0;

  return (
    <div
      className={`fixed inset-0 z-[300] ${exiting ? "tour-exit" : "tour-enter"}`}
    >
      {/* SVG overlay with spotlight cutout */}
      <svg
        className="absolute inset-0 w-full h-full pointer-events-none"
        style={{
          position: "fixed",
          top: 0,
          left: 0,
          width: "100vw",
          height: "100vh",
        }}
      >
        <defs>
          <mask id="tour-spotlight-mask">
            <rect width="100%" height="100%" fill="white" />
            {targetRect && (
              <rect
                x={spotlightX - window.scrollX}
                y={spotlightY - window.scrollY}
                width={spotlightW}
                height={spotlightH}
                rx="6"
                ry="6"
                fill="black"
                className="tour-spotlight-rect"
              />
            )}
          </mask>
        </defs>
        <rect
          width="100%"
          height="100%"
          fill="rgba(0,0,0,0.7)"
          mask="url(#tour-spotlight-mask)"
        />
      </svg>

      {/* Spotlight border glow */}
      {targetRect && (
        <div
          className="fixed pointer-events-none tour-spotlight-glow"
          style={{
            top: spotlightY - window.scrollY,
            left: spotlightX - window.scrollX,
            width: spotlightW,
            height: spotlightH,
            borderRadius: 6,
          }}
        />
      )}

      {/* Click blocker (allows clicking the spotlight area) */}
      <div className="fixed inset-0" onClick={handleDismiss} />

      {/* Popover */}
      {popoverPos && (
        <div
          className="fixed tour-popover-enter"
          style={{
            top: popoverPos.top - window.scrollY,
            left: popoverPos.left,
            width: POPOVER_WIDTH,
            zIndex: 301,
          }}
        >
          <div className="bg-neutral-950 border border-white/[0.08] rounded-md shadow-2xl shadow-black/50 overflow-hidden">
            {/* Header */}
            <div className="flex items-center gap-3 px-5 py-4 border-b border-white/[0.06]">
              {Icon && (
                <div className="w-8 h-8 rounded-md bg-[#d4af37]/[0.08] border border-[#d4af37]/20 flex items-center justify-center shrink-0">
                  <Icon className="w-4 h-4 text-[#d4af37]" />
                </div>
              )}
              <div className="flex-1 min-w-0">
                <h3 className="text-sm font-medium">{step.title}</h3>
              </div>
              <button
                onClick={handleDismiss}
                className="p-1 rounded-sm hover:bg-white/[0.04] transition-colors duration-150 shrink-0"
                aria-label="Close tour"
              >
                <X className="w-3.5 h-3.5 text-neutral-600" />
              </button>
            </div>

            {/* Body */}
            <div className="px-5 py-4">
              <p className="text-xs text-neutral-400 font-light leading-relaxed">
                {step.description}
              </p>
            </div>

            {/* Footer */}
            <div className="flex items-center justify-between px-5 py-3 border-t border-white/[0.06]">
              {/* Step indicator */}
              <div className="flex items-center gap-1.5">
                {TOUR_STEPS.map((_, i) => (
                  <div
                    key={i}
                    className={`h-1 rounded-full transition-all duration-200 ${
                      i === stepIndex
                        ? "w-4 bg-[#d4af37]"
                        : i < stepIndex
                          ? "w-1.5 bg-[#d4af37]/40"
                          : "w-1.5 bg-white/[0.08]"
                    }`}
                  />
                ))}
                <span className="text-[10px] font-mono text-neutral-700 ml-2">
                  {stepIndex + 1}/{TOUR_STEPS.length}
                </span>
              </div>

              {/* Navigation */}
              <div className="flex items-center gap-2">
                {stepIndex > 0 && (
                  <button
                    onClick={handleBack}
                    className="flex items-center gap-1 px-3 py-1.5 text-[10px] font-mono text-neutral-500 hover:text-white transition-colors duration-150"
                  >
                    <ArrowLeft className="w-3 h-3" />
                    Back
                  </button>
                )}
                <button
                  onClick={handleNext}
                  className="flex items-center gap-1 px-4 py-1.5 text-[10px] font-mono bg-[#d4af37] text-black rounded-sm hover:bg-[#c4a030] transition-colors duration-150"
                >
                  {stepIndex < TOUR_STEPS.length - 1 ? (
                    <>
                      Next
                      <ArrowRight className="w-3 h-3" />
                    </>
                  ) : (
                    "Done"
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
