"use client";

import * as React from "react";
import { cn } from "@/lib/utils";

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, id, ...props }, ref) => {
    const inputId = id || React.useId();

    return (
      <div className="space-y-2">
        {label && (
          <label
            htmlFor={inputId}
            className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 block"
          >
            {label}
          </label>
        )}
        <input
          id={inputId}
          ref={ref}
          className={cn(
            "w-full bg-black/40 border rounded-sm px-3 py-2",
            "text-sm font-light text-white",
            "placeholder:text-neutral-700",
            "outline-none transition-colors duration-200",
            error
              ? "border-red-500/40 focus:border-red-500/60"
              : "border-white/[0.08] focus:border-[#d4af37]/40",
            "disabled:opacity-40 disabled:cursor-not-allowed",
            className,
          )}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? `${inputId}-error` : undefined}
          {...props}
        />
        {error && (
          <p
            id={`${inputId}-error`}
            className="text-xs text-red-400 font-light"
          >
            {error}
          </p>
        )}
      </div>
    );
  },
);

Input.displayName = "Input";

export { Input };
