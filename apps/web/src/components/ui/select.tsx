"use client";

import * as React from "react";
import { Select as SelectPrimitive } from "@base-ui/react/select";
import { cn } from "@/lib/utils";
import { ChevronDown, Check } from "lucide-react";

// ── Simple Select ───────────────────────────────────────────────────────

interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface SelectProps {
  value: string;
  onValueChange: (value: string) => void;
  options: SelectOption[];
  placeholder?: string;
  label?: string;
  disabled?: boolean;
  className?: string;
}

function Select({
  value,
  onValueChange,
  options,
  placeholder = "Select...",
  label,
  disabled,
  className,
}: SelectProps) {
  return (
    <div className={cn("space-y-2", className)}>
      {label && (
        <span className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 block">
          {label}
        </span>
      )}
      <SelectPrimitive.Root
        value={value}
        onValueChange={(val) => {
          if (val !== null) onValueChange(val as string);
        }}
        disabled={disabled}
      >
        <SelectPrimitive.Trigger
          className={cn(
            "flex items-center justify-between w-full",
            "bg-black/40 border border-white/[0.08] rounded-sm px-3 py-2",
            "text-sm font-light text-white",
            "outline-none transition-colors duration-200",
            "hover:border-white/[0.12]",
            "focus:border-[#d4af37]/40",
            "disabled:opacity-40 disabled:cursor-not-allowed",
            "data-[placeholder]:text-neutral-700",
          )}
        >
          <SelectPrimitive.Value placeholder={placeholder} />
          <SelectPrimitive.Icon>
            <ChevronDown className="w-3.5 h-3.5 text-neutral-600" />
          </SelectPrimitive.Icon>
        </SelectPrimitive.Trigger>

        <SelectPrimitive.Portal>
          <SelectPrimitive.Positioner
            className="z-[100]"
            sideOffset={4}
            alignItemWithTrigger={false}
          >
            <SelectPrimitive.Popup
              className={cn(
                "min-w-[var(--anchor-width)] max-h-[300px] overflow-auto",
                "rounded-sm border border-white/[0.08] bg-[#0a0a0a] shadow-xl",
                "py-1",
                "outline-none",
                "transition-all duration-150",
                "data-[ending-style]:scale-95 data-[ending-style]:opacity-0",
                "data-[starting-style]:scale-95 data-[starting-style]:opacity-0",
              )}
            >
              <SelectPrimitive.List>
                {options.map((opt) => (
                  <SelectPrimitive.Item
                    key={opt.value}
                    value={opt.value}
                    disabled={opt.disabled}
                    className={cn(
                      "flex items-center gap-2 px-3 py-2 cursor-default",
                      "text-sm font-light text-neutral-400",
                      "outline-none transition-colors duration-100",
                      "data-[highlighted]:bg-white/[0.04] data-[highlighted]:text-white",
                      "data-[selected]:text-[#d4af37]",
                      "data-[disabled]:opacity-40 data-[disabled]:pointer-events-none",
                    )}
                  >
                    <SelectPrimitive.ItemIndicator className="w-4 flex items-center justify-center">
                      <Check className="w-3 h-3" />
                    </SelectPrimitive.ItemIndicator>
                    <SelectPrimitive.ItemText>
                      {opt.label}
                    </SelectPrimitive.ItemText>
                  </SelectPrimitive.Item>
                ))}
              </SelectPrimitive.List>
            </SelectPrimitive.Popup>
          </SelectPrimitive.Positioner>
        </SelectPrimitive.Portal>
      </SelectPrimitive.Root>
    </div>
  );
}

export { Select, type SelectOption };
