"use client";

import { useMemo, useState, useCallback } from "react";
import type { PeerResponse } from "@/lib/api";

// ── Types ──────────────────────────────────────────────────────────────────

interface PeerGraphProps {
  peers: PeerResponse[];
  /** Latency history per peer for sparkline-style trend */
  latencyHistory?: Record<string, number[]>;
  className?: string;
}

interface NodePosition {
  x: number;
  y: number;
  peer: PeerResponse;
  angle: number;
  signalLevel: "excellent" | "good" | "fair" | "poor" | "unknown";
  bandwidth: number;
}

// ── Helpers ──────���──────────────────────────────────────────────────────────

function getSignalLevel(
  latencyMs: number | null | undefined,
): "excellent" | "good" | "fair" | "poor" | "unknown" {
  if (latencyMs == null) return "unknown";
  if (latencyMs < 30) return "excellent";
  if (latencyMs < 80) return "good";
  if (latencyMs < 150) return "fair";
  return "poor";
}

const signalColors: Record<string, string> = {
  excellent: "#34d399", // emerald-400
  good: "#34d399",
  fair: "#eab308", // yellow-500
  poor: "#ef4444", // red-500
  unknown: "#525252", // neutral-600
};

const signalGlowColors: Record<string, string> = {
  excellent: "rgba(52, 211, 153, 0.3)",
  good: "rgba(52, 211, 153, 0.2)",
  fair: "rgba(234, 179, 8, 0.2)",
  poor: "rgba(239, 68, 68, 0.2)",
  unknown: "rgba(82, 82, 82, 0.1)",
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function truncateId(id: string): string {
  if (id.length > 16) return `${id.slice(0, 6)}...${id.slice(-4)}`;
  return id;
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60_000) return "now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h`;
  return `${Math.floor(diff / 86_400_000)}d`;
}

// ── Layout ──────────────────────────────────────────────────────────────────

function computeLayout(
  peers: PeerResponse[],
  width: number,
  height: number,
): NodePosition[] {
  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(cx, cy) * 0.7;

  return peers.map((peer, i) => {
    const angle = (i / peers.length) * Math.PI * 2 - Math.PI / 2;
    // Peers with lower latency are slightly closer to center
    const latencyFactor =
      peer.latency_ms != null
        ? Math.max(0.75, Math.min(1.0, peer.latency_ms / 200))
        : 0.9;
    const r = radius * latencyFactor;

    return {
      x: cx + Math.cos(angle) * r,
      y: cy + Math.sin(angle) * r,
      peer,
      angle,
      signalLevel: getSignalLevel(peer.latency_ms),
      bandwidth: peer.bytes_sent + peer.bytes_recv,
    };
  });
}

// ── Connection Line ─────────────────────────────────────────────────────────

function ConnectionLine({
  x1,
  y1,
  x2,
  y2,
  signal,
  index,
  isHovered,
}: {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  signal: string;
  index: number;
  isHovered: boolean;
}) {
  const color = signalColors[signal];
  const baseOpacity = isHovered ? 0.5 : 0.15;
  const pulseDelay = (index * 0.4) % 3;

  return (
    <g>
      {/* Base connection line */}
      <line
        x1={x1}
        y1={y1}
        x2={x2}
        y2={y2}
        stroke={color}
        strokeOpacity={baseOpacity}
        strokeWidth={isHovered ? 1.5 : 0.8}
        strokeDasharray={isHovered ? "none" : "4 4"}
        className="transition-all duration-200"
      />

      {/* Animated data pulse traveling along the line */}
      <circle r={isHovered ? 2.5 : 1.5} fill={color} opacity={0.6}>
        <animateMotion
          dur={`${2 + index * 0.3}s`}
          repeatCount="indefinite"
          begin={`${pulseDelay}s`}
          path={`M${x1},${y1} L${x2},${y2}`}
        />
      </circle>
    </g>
  );
}

// ── Peer Node ─────────────���───────────────────────���─────────────────────────

function PeerNode({
  node,
  index,
  isHovered,
  onHover,
  onLeave,
}: {
  node: NodePosition;
  index: number;
  isHovered: boolean;
  onHover: (id: string) => void;
  onLeave: () => void;
}) {
  const color = signalColors[node.signalLevel];
  const glowColor = signalGlowColors[node.signalLevel];
  const nodeRadius = isHovered ? 7 : 5;
  const breatheDelay = index * 0.5;

  return (
    <g
      onMouseEnter={() => onHover(node.peer.peer_id)}
      onMouseLeave={onLeave}
      className="cursor-pointer"
      role="button"
      tabIndex={0}
      aria-label={`Peer ${truncateId(node.peer.peer_id)}`}
    >
      {/* Outer glow ring */}
      <circle
        cx={node.x}
        cy={node.y}
        r={nodeRadius + 6}
        fill={glowColor}
        className="peer-graph-breathe"
        style={{ animationDelay: `${breatheDelay}s` }}
      />

      {/* Node circle */}
      <circle
        cx={node.x}
        cy={node.y}
        r={nodeRadius}
        fill="#0a0a0a"
        stroke={color}
        strokeWidth={isHovered ? 2 : 1.2}
        className="transition-all duration-200"
      />

      {/* Inner dot */}
      <circle
        cx={node.x}
        cy={node.y}
        r={2}
        fill={color}
        opacity={0.8}
      />

      {/* Peer ID label */}
      <text
        x={node.x}
        y={node.y + (nodeRadius + 14)}
        textAnchor="middle"
        fill={isHovered ? "#fafafa" : "#525252"}
        fontSize="8"
        fontFamily="var(--font-mono)"
        className="transition-all duration-200 select-none pointer-events-none"
      >
        {truncateId(node.peer.peer_id)}
      </text>
    </g>
  );
}

// ── Tooltip ���────────────────────────────────���───────────────────────────────

function PeerTooltip({
  node,
  svgWidth,
  svgHeight,
}: {
  node: NodePosition;
  svgWidth: number;
  svgHeight: number;
}) {
  const tooltipW = 180;
  const tooltipH = 96;
  const padding = 10;

  // Position tooltip to avoid going off-screen
  let tx = node.x + 16;
  let ty = node.y - tooltipH / 2;

  if (tx + tooltipW > svgWidth - padding) tx = node.x - tooltipW - 16;
  if (ty < padding) ty = padding;
  if (ty + tooltipH > svgHeight - padding) ty = svgHeight - tooltipH - padding;

  const latencyText =
    node.peer.latency_ms != null ? `${node.peer.latency_ms}ms` : "—";

  return (
    <g className="peer-graph-tooltip-enter pointer-events-none">
      {/* Background */}
      <rect
        x={tx}
        y={ty}
        width={tooltipW}
        height={tooltipH}
        rx={4}
        fill="#0a0a0a"
        stroke="rgba(255,255,255,0.08)"
        strokeWidth={1}
      />

      {/* Peer ID */}
      <text
        x={tx + 10}
        y={ty + 18}
        fill="#fafafa"
        fontSize="9"
        fontFamily="var(--font-mono)"
        fontWeight="500"
      >
        {truncateId(node.peer.peer_id)}
      </text>

      {/* Latency */}
      <text
        x={tx + 10}
        y={ty + 34}
        fill="#737373"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        Latency:
      </text>
      <text
        x={tx + 60}
        y={ty + 34}
        fill={signalColors[node.signalLevel]}
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        {latencyText}
      </text>

      {/* Bandwidth */}
      <text
        x={tx + 10}
        y={ty + 48}
        fill="#737373"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        Sent:
      </text>
      <text
        x={tx + 42}
        y={ty + 48}
        fill="#a3a3a3"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        {formatBytes(node.peer.bytes_sent)}
      </text>

      <text
        x={tx + 10}
        y={ty + 62}
        fill="#737373"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        Recv:
      </text>
      <text
        x={tx + 42}
        y={ty + 62}
        fill="#a3a3a3"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        {formatBytes(node.peer.bytes_recv)}
      </text>

      {/* Connected */}
      <text
        x={tx + 10}
        y={ty + 78}
        fill="#737373"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        Connected:
      </text>
      <text
        x={tx + 70}
        y={ty + 78}
        fill="#a3a3a3"
        fontSize="8"
        fontFamily="var(--font-mono)"
      >
        {timeAgo(node.peer.connected_at)} ago
      </text>

      {/* Address */}
      <text
        x={tx + 10}
        y={ty + 90}
        fill="#404040"
        fontSize="7"
        fontFamily="var(--font-mono)"
      >
        {node.peer.multiaddr.length > 28
          ? node.peer.multiaddr.slice(0, 28) + "..."
          : node.peer.multiaddr}
      </text>
    </g>
  );
}

// ── Main Component ──────���───────────────────────────────────────────────────

export function PeerGraph({ peers, className }: PeerGraphProps) {
  const [hoveredPeer, setHoveredPeer] = useState<string | null>(null);

  const handleHover = useCallback((id: string) => setHoveredPeer(id), []);
  const handleLeave = useCallback(() => setHoveredPeer(null), []);

  const width = 600;
  const height = 400;
  const cx = width / 2;
  const cy = height / 2;

  const nodes = useMemo(() => computeLayout(peers, width, height), [peers]);

  const hoveredNode = useMemo(
    () => nodes.find((n) => n.peer.peer_id === hoveredPeer) ?? null,
    [nodes, hoveredPeer],
  );

  if (peers.length === 0) return null;

  return (
    <div className={`relative ${className ?? ""}`}>
      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full h-auto max-h-[400px]"
        aria-label="Peer network topology graph"
      >
        <defs>
          {/* Center node gold glow */}
          <radialGradient id="peer-center-glow">
            <stop offset="0%" stopColor="#d4af37" stopOpacity="0.15" />
            <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
          </radialGradient>

          {/* Grid pattern for subtle background */}
          <pattern
            id="peer-graph-grid"
            width="20"
            height="20"
            patternUnits="userSpaceOnUse"
          >
            <circle cx="10" cy="10" r="0.3" fill="rgba(255,255,255,0.03)" />
          </pattern>
        </defs>

        {/* Background grid */}
        <rect width={width} height={height} fill="url(#peer-graph-grid)" />

        {/* Concentric range rings */}
        {[0.35, 0.55, 0.75].map((r) => (
          <circle
            key={r}
            cx={cx}
            cy={cy}
            r={Math.min(cx, cy) * r}
            fill="none"
            stroke="rgba(255,255,255,0.03)"
            strokeWidth={0.5}
            strokeDasharray="2 6"
          />
        ))}

        {/* Connection lines */}
        {nodes.map((node, i) => (
          <ConnectionLine
            key={node.peer.peer_id}
            x1={cx}
            y1={cy}
            x2={node.x}
            y2={node.y}
            signal={node.signalLevel}
            index={i}
            isHovered={node.peer.peer_id === hoveredPeer}
          />
        ))}

        {/* Center node — "You" */}
        <circle
          cx={cx}
          cy={cy}
          r={40}
          fill="url(#peer-center-glow)"
          className="peer-graph-center-pulse"
        />
        <circle
          cx={cx}
          cy={cy}
          r={10}
          fill="#0a0a0a"
          stroke="#d4af37"
          strokeWidth={1.5}
        />
        <circle cx={cx} cy={cy} r={3.5} fill="#d4af37" opacity={0.8} />
        <text
          x={cx}
          y={cy + 24}
          textAnchor="middle"
          fill="#d4af37"
          fontSize="9"
          fontFamily="var(--font-mono)"
          letterSpacing="0.1em"
          className="select-none"
        >
          YOU
        </text>

        {/* Peer nodes */}
        {nodes.map((node, i) => (
          <PeerNode
            key={node.peer.peer_id}
            node={node}
            index={i}
            isHovered={node.peer.peer_id === hoveredPeer}
            onHover={handleHover}
            onLeave={handleLeave}
          />
        ))}

        {/* Tooltip for hovered peer */}
        {hoveredNode && (
          <PeerTooltip
            node={hoveredNode}
            svgWidth={width}
            svgHeight={height}
          />
        )}

        {/* Legend */}
        <g transform={`translate(${width - 120}, 16)`}>
          <text
            x={0}
            y={0}
            fill="#525252"
            fontSize="7"
            fontFamily="var(--font-mono)"
            letterSpacing="0.15em"
          >
            SIGNAL
          </text>
          {(
            [
              ["excellent", "<30ms"],
              ["good", "<80ms"],
              ["fair", "<150ms"],
              ["poor", ">150ms"],
            ] as const
          ).map(([level, label], i) => (
            <g key={level} transform={`translate(0, ${12 + i * 12})`}>
              <circle cx={4} cy={0} r={2.5} fill={signalColors[level]} />
              <text
                x={12}
                y={3}
                fill="#525252"
                fontSize="7"
                fontFamily="var(--font-mono)"
              >
                {label}
              </text>
            </g>
          ))}
        </g>
      </svg>
    </div>
  );
}
