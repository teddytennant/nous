/**
 * Editorial hero composition. A small constellation of nodes connected by
 * hairlines on `--rule`. No glow, no breathing, no pulse ring. Renders
 * statically — present, not animated.
 *
 * Two nodes carry the accent (`--oxblood`) — the center and one peer —
 * because the principle is "stamp, not wash."
 */

const nodes = [
  { x: 50, y: 50, r: 2.4, accent: true },   // center
  { x: 22, y: 28, r: 1.2 },
  { x: 78, y: 24, r: 1.6, accent: true },
  { x: 35, y: 72, r: 1.2 },
  { x: 72, y: 68, r: 1.2 },
  { x: 15, y: 52, r: 0.9 },
  { x: 85, y: 48, r: 0.9 },
  { x: 42, y: 18, r: 0.9 },
  { x: 58, y: 82, r: 0.9 },
  { x: 30, y: 45, r: 1.2 },
  { x: 68, y: 38, r: 1.2 },
  { x: 55, y: 30, r: 0.9 },
  { x: 40, y: 60, r: 0.9 },
];

const edges: [number, number][] = [
  [0, 1], [0, 2], [0, 3], [0, 4], [0, 9], [0, 10],
  [1, 7], [1, 5], [1, 9],
  [2, 6], [2, 7], [2, 11],
  [3, 5], [3, 8], [3, 12],
  [4, 6], [4, 8],
  [9, 12], [10, 11],
];

export function HeroNetwork() {
  return (
    <svg
      viewBox="0 0 100 100"
      className="absolute inset-0 w-full h-full"
      preserveAspectRatio="xMidYMid slice"
      aria-hidden="true"
    >
      {edges.map(([a, b], i) => (
        <line
          key={`e${i}`}
          x1={nodes[a].x}
          y1={nodes[a].y}
          x2={nodes[b].x}
          y2={nodes[b].y}
          stroke="var(--rule)"
          strokeWidth="0.25"
        />
      ))}

      {nodes.map((n, i) => (
        <circle
          key={`n${i}`}
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill={n.accent ? "var(--oxblood)" : "var(--ivory-dim)"}
          opacity={n.accent ? 0.85 : 0.18}
        />
      ))}
    </svg>
  );
}
