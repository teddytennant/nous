"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

export interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, label, error, id, ...props }, ref) => {
    const textareaId = id || React.useId();

    return (
      <div className="space-y-2">
        {label && (
          <label
            htmlFor={textareaId}
            className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 block"
          >
            {label}
          </label>
        )}
        <textarea
          id={textareaId}
          ref={ref}
          className={cn(
            "w-full bg-black/40 border rounded-sm px-3 py-2",
            "text-sm font-light text-white",
            "placeholder:text-neutral-700",
            "outline-none transition-colors duration-200 resize-none",
            error
              ? "border-red-500/40 focus:border-red-500/60"
              : "border-white/[0.08] focus:border-[#d4af37]/40",
            "disabled:opacity-40 disabled:cursor-not-allowed",
            className,
          )}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${textareaId}-error` : undefined}
          {...props}
        />
        {error && (
          <p
            id={`${textareaId}-error`}
            className="text-xs text-red-400 font-light"
          >
            {error}
          </p>
        )}
      </div>
    );
  },
);

Textarea.displayName = "Textarea";

export { Textarea };
