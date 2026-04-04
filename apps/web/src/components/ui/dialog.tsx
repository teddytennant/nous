"use client";

import * as React from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { cn } from "@/lib/utils";
import { X } from "lucide-react";

// ── Root ────────────────────────────────────────────────────────────────

const DialogRoot = DialogPrimitive.Root;
const DialogTrigger = DialogPrimitive.Trigger;
const DialogClose = DialogPrimitive.Close;
const DialogTitle = DialogPrimitive.Title;
const DialogDescription = DialogPrimitive.Description;
const DialogPortal = DialogPrimitive.Portal;

// ── Backdrop ────────────────────────────────────────────────────────────

function DialogBackdrop({
  className,
  ...props
}: DialogPrimitive.Backdrop.Props) {
  return (
    <DialogPrimitive.Backdrop
      className={cn(
        "fixed inset-0 z-50 bg-black/60 backdrop-blur-sm",
        "transition-opacity duration-200",
        "data-[ending-style]:opacity-0 data-[starting-style]:opacity-0",
        className,
      )}
      {...props}
    />
  );
}

// ── Popup (the actual dialog panel) ─────────────────────────────────────

function DialogPopup({
  className,
  children,
  ...props
}: DialogPrimitive.Popup.Props) {
  return (
    <DialogPrimitive.Popup
      className={cn(
        "fixed left-1/2 top-1/2 z-50 -translate-x-1/2 -translate-y-1/2",
        "w-[calc(100vw-2rem)] max-w-lg",
        "rounded-md border border-white/[0.08] bg-[#0a0a0a] shadow-2xl",
        "outline-none",
        "transition-all duration-200",
        "data-[ending-style]:scale-95 data-[ending-style]:opacity-0",
        "data-[starting-style]:scale-95 data-[starting-style]:opacity-0",
        className,
      )}
      {...props}
    >
      {children}
    </DialogPrimitive.Popup>
  );
}

// ── Header (title + description + close) ────────────────────────────────

function DialogHeader({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex items-start justify-between gap-4 px-6 pt-6 pb-4",
        className,
      )}
      {...props}
    >
      <div className="flex-1 min-w-0">{children}</div>
    </div>
  );
}

function DialogTitleStyled({
  className,
  ...props
}: DialogPrimitive.Title.Props) {
  return (
    <DialogPrimitive.Title
      className={cn(
        "text-lg font-medium tracking-wide text-white",
        className,
      )}
      {...props}
    />
  );
}

function DialogDescriptionStyled({
  className,
  ...props
}: DialogPrimitive.Description.Props) {
  return (
    <DialogPrimitive.Description
      className={cn(
        "text-sm text-neutral-500 font-light leading-relaxed mt-1",
        className,
      )}
      {...props}
    />
  );
}

// ── Body ────────────────────────────────────────────────────────────────

function DialogBody({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("px-6 pb-2", className)} {...props} />
  );
}

// ── Footer ──────────────────────────────────────────────────────────────

function DialogFooter({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex items-center justify-end gap-3 px-6 py-4 border-t border-white/[0.06]",
        className,
      )}
      {...props}
    />
  );
}

// ── Close button (X icon) ───────────────────────────────────────────────

function DialogCloseButton({
  className,
  ...props
}: DialogPrimitive.Close.Props) {
  return (
    <DialogPrimitive.Close
      className={cn(
        "absolute right-4 top-4 p-1.5 rounded-sm",
        "text-neutral-600 hover:text-white hover:bg-white/[0.06]",
        "transition-colors duration-150 outline-none",
        "focus-visible:ring-2 focus-visible:ring-[#d4af37]/40",
        className,
      )}
      aria-label="Close"
      {...props}
    >
      <X className="w-4 h-4" />
    </DialogPrimitive.Close>
  );
}

// ── Composed Dialog ─────────────────────────────────────────────────────

interface DialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  children: React.ReactNode;
  modal?: boolean;
}

function Dialog({ open, onOpenChange, children, modal = true }: DialogProps) {
  return (
    <DialogRoot open={open} onOpenChange={onOpenChange} modal={modal}>
      <DialogPortal>
        <DialogBackdrop />
        <DialogPopup>
          <DialogCloseButton />
          {children}
        </DialogPopup>
      </DialogPortal>
    </DialogRoot>
  );
}

export {
  Dialog,
  DialogRoot,
  DialogTrigger,
  DialogClose,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogHeader,
  DialogTitleStyled as DialogTitle,
  DialogDescriptionStyled as DialogDescription,
  DialogBody,
  DialogFooter,
  DialogCloseButton,
};
