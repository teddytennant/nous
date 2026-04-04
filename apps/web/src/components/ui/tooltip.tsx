"use client";

import * as React from "react";
import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";
import { cn } from "@/lib/utils";

// ── Provider ────────────���──────────────────────────────────────────────

const TooltipProvider = TooltipPrimitive.Provider;

// ── Simple Tooltip ──────────────────���──────────────────────────────────

interface TooltipProps {
  /** The content shown inside the tooltip popup */
  content: React.ReactNode;
  /** Which side of the trigger to place the tooltip */
  side?: "top" | "bottom" | "left" | "right";
  /** Alignment relative to the trigger */
  align?: "start" | "center" | "end";
  /** Offset from the trigger in px */
  sideOffset?: number;
  /** The trigger element */
  children: React.ReactElement;
  /** Additional className for the popup */
  className?: string;
}

function Tooltip({
  content,
  side = "top",
  align = "center",
  sideOffset = 6,
  children,
  className,
}: TooltipProps) {
  return (
    <TooltipPrimitive.Root>
      <TooltipPrimitive.Trigger
        render={children}
      />
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Positioner
          side={side}
          align={align}
          sideOffset={sideOffset}
        >
          <TooltipPrimitive.Popup
            className={cn(
              "z-[200] max-w-xs px-3 py-1.5 rounded-sm",
              "bg-neutral-900 border border-white/[0.08] shadow-xl",
              "text-xs text-neutral-300 font-light leading-relaxed",
              "transition-all duration-150",
              "data-[ending-style]:scale-95 data-[ending-style]:opacity-0",
              "data-[starting-style]:scale-95 data-[starting-style]:opacity-0",
              className,
            )}
          >
            {content}
          </TooltipPrimitive.Popup>
        </TooltipPrimitive.Positioner>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}

// ── Composed parts (for custom layouts) ────────────────────────────────

const TooltipRoot = TooltipPrimitive.Root;
const TooltipTrigger = TooltipPrimitive.Trigger;
const TooltipPortal = TooltipPrimitive.Portal;
const TooltipPositioner = TooltipPrimitive.Positioner;

function TooltipPopup({
  className,
  ...props
}: TooltipPrimitive.Popup.Props) {
  return (
    <TooltipPrimitive.Popup
      className={cn(
        "z-[200] max-w-xs px-3 py-1.5 rounded-sm",
        "bg-neutral-900 border border-white/[0.08] shadow-xl",
        "text-xs text-neutral-300 font-light leading-relaxed",
        "transition-all duration-150",
        "data-[ending-style]:scale-95 data-[ending-style]:opacity-0",
        "data-[starting-style]:scale-95 data-[starting-style]:opacity-0",
        className,
      )}
      {...props}
    />
  );
}

function TooltipArrow({
  className,
  ...props
}: TooltipPrimitive.Arrow.Props) {
  return (
    <TooltipPrimitive.Arrow
      className={cn(
        "fill-neutral-900 [&>path]:stroke-white/[0.08]",
        className,
      )}
      {...props}
    />
  );
}

export {
  Tooltip,
  TooltipProvider,
  TooltipRoot,
  TooltipTrigger,
  TooltipPortal,
  TooltipPositioner,
  TooltipPopup,
  TooltipArrow,
};
