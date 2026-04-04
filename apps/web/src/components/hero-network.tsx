/**
 * Animated network constellation for the landing page hero.
 * Pure SVG + CSS animations. No JavaScript animation loop.
 * Shows a subtle, slowly-moving network of nodes and connections
 * representing the decentralized nature of Nous.
 */

// Node positions are fixed — animation is CSS-only (orbital drift)
const nodes = [
  { x: 50, y: 50, r: 3, gold: true, delay: 0 },     // center
  { x: 22, y: 28, r: 1.5, gold: false, delay: 0.2 },
  { x: 78, y: 24, r: 2, gold: true, delay: 0.4 },
  { x: 35, y: 72, r: 1.5, gold: false, delay: 0.6 },
  { x: 72, y: 68, r: 1.5, gold: true, delay: 0.8 },
  { x: 15, y: 52, r: 1, gold: false, delay: 1.0 },
  { x: 85, y: 48, r: 1, gold: false, delay: 1.2 },
  { x: 42, y: 18, r: 1, gold: false, delay: 1.4 },
  { x: 58, y: 82, r: 1, gold: false, delay: 1.6 },
  { x: 30, y: 45, r: 1.5, gold: true, delay: 0.3 },
  { x: 68, y: 38, r: 1.5, gold: false, delay: 0.5 },
  { x: 55, y: 30, r: 1, gold: false, delay: 0.7 },
  { x: 40, y: 60, r: 1, gold: false, delay: 0.9 },
];

// Connections between nodes (indices)
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
      <defs>
        {/* Soft glow for gold nodes */}
        <filter id="hero-glow" x="-100%" y="-100%" width="300%" height="300%">
          <feGaussianBlur in="SourceGraphic" stdDeviation="1.5" />
        </filter>
      </defs>

      {/* Edges */}
      {edges.map(([a, b], i) => (
        <line
          key={`e${i}`}
          x1={nodes[a].x}
          y1={nodes[a].y}
          x2={nodes[b].x}
          y2={nodes[b].y}
          stroke="white"
          strokeOpacity="0.04"
          strokeWidth="0.3"
          className="hero-edge"
          style={{ animationDelay: `${i * 0.15}s` }}
        />
      ))}

      {/* Node glow layer (behind) */}
      {nodes
        .filter((n) => n.gold)
        .map((n, i) => (
          <circle
            key={`glow${i}`}
            cx={n.x}
            cy={n.y}
            r={n.r * 3}
            fill="#d4af37"
            opacity="0.03"
            filter="url(#hero-glow)"
            className="hero-node-glow"
            style={{ animationDelay: `${n.delay}s` }}
          />
        ))}

      {/* Nodes */}
      {nodes.map((n, i) => (
        <circle
          key={`n${i}`}
          cx={n.x}
          cy={n.y}
          r={n.r}
          fill={n.gold ? "#d4af37" : "white"}
          opacity={n.gold ? 0.2 : 0.06}
          className="hero-node"
          style={{ animationDelay: `${n.delay}s` }}
        />
      ))}

      {/* Animated pulse ring on center node */}
      <circle
        cx={50}
        cy={50}
        r="6"
        fill="none"
        stroke="#d4af37"
        strokeWidth="0.3"
        opacity="0"
        className="hero-pulse-ring"
      />
    </svg>
  );
}
