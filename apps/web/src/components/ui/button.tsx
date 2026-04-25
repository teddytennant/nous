"use client"

import { Button as ButtonPrimitive } from "@base-ui/react/button"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

/**
 * Editorial button primitive.
 *
 *   primary    — oxblood field, ivory text, no shadow, color shifts on press.
 *   secondary  — transparent, hairline rule. Hover deepens the rule.
 *   ghost      — no chrome, oxblood text, underline on hover.
 *   destructive — clay, used sparingly.
 *   link       — text link with the editorial underline reveal.
 *
 * Sharp corners (radius 0/2). Sizes follow the type scale (0.875 / 1 / 1.125).
 * Icon + label gap is 8px. No icon-only variant with shadow — use a secondary
 * with size="icon" if you need it.
 */
const buttonVariants = cva(
  "group/button inline-flex shrink-0 items-center justify-center gap-2 whitespace-nowrap font-medium tracking-tight transition-colors duration-[160ms] outline-none select-none focus-visible:outline focus-visible:outline-1 focus-visible:outline-oxblood focus-visible:outline-offset-2 disabled:pointer-events-none disabled:opacity-50 aria-invalid:text-destructive [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        // Primary — oxblood field, ivory text, sharp.
        default:
          "bg-oxblood text-ivory rounded-[2px] hover:bg-oxblood-dim active:bg-oxblood-dim",
        // Secondary — transparent with hairline rule.
        secondary:
          "bg-transparent text-foreground border border-border rounded-[2px] hover:border-foreground",
        outline:
          "bg-transparent text-foreground border border-border rounded-[2px] hover:border-foreground",
        // Ghost — no chrome, oxblood text, underline reveal.
        ghost:
          "bg-transparent text-oxblood rounded-[2px] hover:underline underline-offset-4 decoration-1",
        destructive:
          "bg-transparent text-destructive border border-destructive/40 rounded-[2px] hover:border-destructive",
        // Subtle text link, no border, no fill.
        link: "bg-transparent text-foreground rounded-none px-0 hover:text-oxblood",
        // Backwards-compat aliases — map to editorial variants.
        gold:
          "bg-oxblood text-ivory rounded-[2px] hover:bg-oxblood-dim active:bg-oxblood-dim",
        "ghost-dark":
          "bg-transparent text-foreground rounded-[2px] hover:text-oxblood",
        "outline-dark":
          "bg-transparent text-foreground border border-border rounded-[2px] hover:border-foreground",
      },
      size: {
        default: "h-8 px-3 text-[0.875rem]",
        xs: "h-6 px-2 text-[0.75rem]",
        sm: "h-7 px-2.5 text-[0.8125rem]",
        lg: "h-10 px-4 text-[1rem]",
        icon: "size-8",
        "icon-xs": "size-6",
        "icon-sm": "size-7",
        "icon-lg": "size-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

function Button({
  className,
  variant = "default",
  size = "default",
  ...props
}: ButtonPrimitive.Props & VariantProps<typeof buttonVariants>) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  )
}

export { Button, buttonVariants }
