"use client";

/**
 * Deterministic avatar generated from a DID/pubkey string.
 * Same input always produces the same color + initials.
 */

// Simple hash function for deterministic color generation
function hashString(str: string): number {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = (hash << 5) - hash + str.charCodeAt(i);
    hash |= 0;
  }
  return Math.abs(hash);
}

// Muted, dark-mode-friendly palette that works with the design system
const AVATAR_COLORS = [
  ["#6366f1", "#4f46e5"], // indigo
  ["#8b5cf6", "#7c3aed"], // violet
  ["#a855f7", "#9333ea"], // purple
  ["#d946ef", "#c026d3"], // fuchsia
  ["#ec4899", "#db2777"], // pink
  ["#f43f5e", "#e11d48"], // rose
  ["#ef4444", "#dc2626"], // red
  ["#f97316", "#ea580c"], // orange
  ["#eab308", "#ca8a04"], // yellow
  ["#84cc16", "#65a30d"], // lime
  ["#22c55e", "#16a34a"], // green
  ["#14b8a6", "#0d9488"], // teal
  ["#06b6d4", "#0891b2"], // cyan
  ["#3b82f6", "#2563eb"], // blue
  ["#d4af37", "#b8972e"], // gold (Nous accent)
];

function getInitials(did: string): string {
  // Strip common prefixes to get the unique part
  const stripped = did
    .replace(/^did:key:z/, "")
    .replace(/^did:key:/, "")
    .replace(/^npub/, "")
    .replace(/^0x/, "");

  // Take first 2 chars of the unique part, uppercase
  return stripped.slice(0, 2).toUpperCase();
}

export function Avatar({
  did,
  size = "sm",
  className,
}: {
  did: string;
  size?: "xs" | "sm" | "md" | "lg";
  className?: string;
}) {
  const hash = hashString(did);
  const [colorA, colorB] = AVATAR_COLORS[hash % AVATAR_COLORS.length];
  const initials = getInitials(did);

  const sizeClasses = {
    xs: "w-5 h-5 text-[7px]",
    sm: "w-7 h-7 text-[9px]",
    md: "w-9 h-9 text-[10px]",
    lg: "w-12 h-12 text-xs",
  };

  // Rotate gradient based on hash for more variation
  const rotation = (hash % 360);

  return (
    <div
      className={`shrink-0 rounded-full flex items-center justify-center font-mono font-medium text-white/90 select-none ${sizeClasses[size]} ${className || ""}`}
      style={{
        background: `linear-gradient(${rotation}deg, ${colorA}, ${colorB})`,
      }}
      title={did}
      aria-label={`Avatar for ${did}`}
    >
      {initials}
    </div>
  );
}
