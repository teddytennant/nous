"use client";

/**
 * Deterministic avatar generated from a DID string.
 * Produces a symmetric 5x5 grid pattern (mirrored horizontally)
 * using colors derived from the DID hash.
 */

interface DidAvatarProps {
  did: string;
  size?: number;
  className?: string;
}

// Simple hash — deterministic, no crypto needed
function hashDid(did: string): number[] {
  const bytes: number[] = [];
  let h = 0;
  for (let i = 0; i < did.length; i++) {
    h = ((h << 5) - h + did.charCodeAt(i)) | 0;
    if (i % 4 === 3) {
      bytes.push(Math.abs(h) % 256);
      h = ((h << 3) ^ (h >>> 2)) | 0;
    }
  }
  // Ensure we have enough bytes
  while (bytes.length < 20) {
    h = ((h << 5) - h + bytes.length * 7) | 0;
    bytes.push(Math.abs(h) % 256);
  }
  return bytes;
}

// Gold-adjacent palette derived from the Nous accent color
const PALETTE = [
  "#d4af37", // gold
  "#c4a030", // darker gold
  "#b8860b", // dark goldenrod
  "#daa520", // goldenrod
  "#cd853f", // peru
  "#d4a574", // warm tan
  "#c9b458", // brass
  "#a0845c", // warm brown
];

function deriveColor(bytes: number[]): string {
  const idx = (bytes[0] + bytes[1]) % PALETTE.length;
  return PALETTE[idx];
}

function derivePattern(bytes: number[]): boolean[][] {
  // 5x5 grid, horizontally mirrored (so we only need 3 columns)
  const grid: boolean[][] = [];
  for (let row = 0; row < 5; row++) {
    const cells: boolean[] = [];
    for (let col = 0; col < 3; col++) {
      const byteIdx = (row * 3 + col + 2) % bytes.length;
      cells.push(bytes[byteIdx] > 128);
    }
    // Mirror: col 3 = col 1, col 4 = col 0
    grid.push([cells[0], cells[1], cells[2], cells[1], cells[0]]);
  }
  return grid;
}

export function DidAvatar({ did, size = 48, className }: DidAvatarProps) {
  const bytes = hashDid(did);
  const color = deriveColor(bytes);
  const pattern = derivePattern(bytes);
  const cellSize = size / 7; // 5 cells + 1 padding on each side
  const padding = cellSize;

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className={className}
      role="img"
      aria-label="Identity avatar"
    >
      {/* Background */}
      <rect
        width={size}
        height={size}
        rx={size * 0.15}
        fill="#0a0a0a"
      />
      {/* Border */}
      <rect
        x="0.5"
        y="0.5"
        width={size - 1}
        height={size - 1}
        rx={size * 0.15}
        fill="none"
        stroke="white"
        strokeOpacity="0.08"
      />
      {/* Pattern cells */}
      {pattern.map((row, ri) =>
        row.map((filled, ci) =>
          filled ? (
            <rect
              key={`${ri}-${ci}`}
              x={padding + ci * cellSize}
              y={padding + ri * cellSize}
              width={cellSize * 0.88}
              height={cellSize * 0.88}
              rx={cellSize * 0.12}
              fill={color}
              opacity={0.7 + (bytes[(ri + ci) % bytes.length] % 30) / 100}
            />
          ) : null,
        ),
      )}
    </svg>
  );
}

// Larger variant with glow effect for profile cards
export function DidAvatarLarge({
  did,
  size = 96,
  className,
}: DidAvatarProps) {
  const bytes = hashDid(did);
  const color = deriveColor(bytes);
  const pattern = derivePattern(bytes);
  const cellSize = size / 7;
  const padding = cellSize;

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className={className}
      role="img"
      aria-label="Identity avatar"
    >
      <defs>
        <filter id="avatar-glow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur in="SourceGraphic" stdDeviation={size * 0.03} result="blur" />
          <feMerge>
            <feMergeNode in="blur" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      {/* Background */}
      <rect
        width={size}
        height={size}
        rx={size * 0.15}
        fill="#0a0a0a"
      />
      {/* Subtle inner glow */}
      <rect
        x="1"
        y="1"
        width={size - 2}
        height={size - 2}
        rx={size * 0.15}
        fill="none"
        stroke={color}
        strokeOpacity="0.15"
        strokeWidth="1.5"
      />
      {/* Pattern cells */}
      <g filter="url(#avatar-glow)">
        {pattern.map((row, ri) =>
          row.map((filled, ci) =>
            filled ? (
              <rect
                key={`${ri}-${ci}`}
                x={padding + ci * cellSize}
                y={padding + ri * cellSize}
                width={cellSize * 0.88}
                height={cellSize * 0.88}
                rx={cellSize * 0.15}
                fill={color}
                opacity={0.75 + (bytes[(ri + ci) % bytes.length] % 25) / 100}
              />
            ) : null,
          ),
        )}
      </g>
    </svg>
  );
}
