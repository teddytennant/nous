"use client";

import { useState } from "react";

// ── Layout constants ────────────────────────────────────────────────────

const CX = 360;
const CY = 280;
const RADIUS = 190;
const NODE_R = 38;
const VB_W = 720;
const VB_H = 560;

// ── Subsystem definitions ───────────────────────────────────────────────

const subsystems = [
  { name: "Identity", tag: "DID:key", angle: 0 },
  { name: "Messaging", tag: "E2EE", angle: 45 },
  { name: "Governance", tag: "Quadratic", angle: 90 },
  { name: "Payments", tag: "Multi-chain", angle: 135 },
  { name: "Social", tag: "Nostr", angle: 180 },
  { name: "Storage", tag: "CRDTs", angle: 225 },
  { name: "AI", tag: "Local LLM", angle: 270 },
  { name: "Browser", tag: "IPFS", angle: 315 },
];

// Secondary links between related subsystems (index pairs)
const secondaryLinks: [number, number][] = [
  [0, 1], // Identity ↔ Messaging (keys)
  [0, 4], // Identity ↔ Social (profile)
  [2, 3], // Governance ↔ Payments (treasury)
  [5, 6], // Storage ↔ AI (data)
  [1, 4], // Messaging ↔ Social (DMs)
  [3, 7], // Payments ↔ Browser (web3)
];

function getPos(angleDeg: number): { x: number; y: number } {
  const rad = ((angleDeg - 90) * Math.PI) / 180;
  return {
    x: CX + RADIUS * Math.cos(rad),
    y: CY + RADIUS * Math.sin(rad),
  };
}

// ── Component ───────────────────────────────────────────────────────────

export function ArchitectureDiagram() {
  const [hovered, setHovered] = useState<number | null>(null);
  const positions = subsystems.map((s) => getPos(s.angle));

  return (
    <div className="w-full max-w-2xl mx-auto">
      <svg
        viewBox={`0 0 ${VB_W} ${VB_H}`}
        className="w-full h-auto arch-diagram"
        role="img"
        aria-label="Nous architecture: 8 subsystems connected through a central core"
      >
        <defs>
          {/* Core ambient glow */}
          <radialGradient id="arch-core-glow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#d4af37" stopOpacity="0.12" />
            <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
          </radialGradient>

          {/* Node hover glow */}
          <radialGradient id="arch-node-glow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#d4af37" stopOpacity="0.2" />
            <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
          </radialGradient>
        </defs>

        {/* Core glow */}
        <circle cx={CX} cy={CY} r={90} fill="url(#arch-core-glow)" />

        {/* Orbit ring */}
        <circle
          cx={CX}
          cy={CY}
          r={RADIUS}
          fill="none"
          stroke="rgba(255,255,255,0.025)"
          strokeWidth={1}
          strokeDasharray="3 8"
          className="arch-orbit"
        />

        {/* Pulse rings — sonar waves emanating from core */}
        {[0, 1.5, 3].map((delay, i) => (
          <circle
            key={`pulse-${i}`}
            cx={CX}
            cy={CY}
            fill="none"
            stroke="#d4af37"
            strokeWidth={0.5}
          >
            <animate
              attributeName="r"
              values="10;120"
              dur="4.5s"
              begin={`${delay + 1.5}s`}
              repeatCount="indefinite"
            />
            <animate
              attributeName="opacity"
              values="0.1;0"
              dur="4.5s"
              begin={`${delay + 1.5}s`}
              repeatCount="indefinite"
            />
          </circle>
        ))}

        {/* Secondary connections (faint arcs between related subsystems) */}
        {secondaryLinks.map(([a, b]) => {
          const pa = positions[a];
          const pb = positions[b];
          const isActive = hovered === a || hovered === b;
          return (
            <line
              key={`sec-${a}-${b}`}
              x1={pa.x}
              y1={pa.y}
              x2={pb.x}
              y2={pb.y}
              stroke={
                isActive
                  ? "rgba(212,175,55,0.12)"
                  : "rgba(255,255,255,0.015)"
              }
              strokeWidth={1}
              strokeDasharray="3 6"
              style={{ transition: "stroke 0.3s ease" }}
            />
          );
        })}

        {/* Primary connections (center → each node) */}
        {positions.map((pos, i) => {
          const isActive = hovered === i;
          const dx = pos.x - CX;
          const dy = pos.y - CY;
          const len = Math.sqrt(dx * dx + dy * dy);
          return (
            <line
              key={`line-${i}`}
              x1={CX}
              y1={CY}
              x2={pos.x}
              y2={pos.y}
              stroke={
                isActive
                  ? "rgba(212,175,55,0.45)"
                  : "rgba(255,255,255,0.05)"
              }
              strokeWidth={isActive ? 1.5 : 1}
              strokeDasharray={len}
              strokeDashoffset={0}
              className="arch-line"
              style={{
                transition: "stroke 0.3s ease, stroke-width 0.3s ease",
                animationDelay: `${i * 80}ms`,
              }}
            />
          );
        })}

        {/* Data-flow particles — flowing along connection lines */}
        <g className="arch-particles">
          {/* Primary connection particles (center ↔ nodes) */}
          {positions.map((pos, i) => {
            const outPath = `M${CX},${CY} L${pos.x.toFixed(1)},${pos.y.toFixed(1)}`;
            const inPath = `M${pos.x.toFixed(1)},${pos.y.toFixed(1)} L${CX},${CY}`;
            return (
              <g
                key={`p-${i}`}
                style={{
                  opacity: hovered === null || hovered === i ? 1 : 0.15,
                  transition: "opacity 0.3s ease",
                }}
              >
                {/* Outward — gold */}
                <circle r={2} fill="#d4af37" opacity={0.35}>
                  <animateMotion
                    path={outPath}
                    dur={`${2.5 + i * 0.2}s`}
                    repeatCount="indefinite"
                  />
                </circle>
                {/* Inward — white */}
                <circle r={1.5} fill="white" opacity={0.15}>
                  <animateMotion
                    path={inPath}
                    dur={`${3.5 + i * 0.25}s`}
                    repeatCount="indefinite"
                  />
                </circle>
              </g>
            );
          })}

          {/* Secondary connection particles */}
          {secondaryLinks.map(([a, b], i) => {
            const pa = positions[a];
            const pb = positions[b];
            const path = `M${pa.x.toFixed(1)},${pa.y.toFixed(1)} L${pb.x.toFixed(1)},${pb.y.toFixed(1)}`;
            return (
              <g
                key={`sp-${i}`}
                style={{
                  opacity:
                    hovered === null || hovered === a || hovered === b
                      ? 1
                      : 0.15,
                  transition: "opacity 0.3s ease",
                }}
              >
                <circle r={1.5} fill="#d4af37" opacity={0.12}>
                  <animateMotion
                    path={path}
                    dur={`${5 + i * 0.5}s`}
                    repeatCount="indefinite"
                  />
                </circle>
              </g>
            );
          })}
        </g>

        {/* Center node */}
        <circle
          cx={CX}
          cy={CY}
          r={44}
          fill="rgba(0,0,0,0.9)"
          stroke={
            hovered !== null
              ? "rgba(212,175,55,0.3)"
              : "rgba(255,255,255,0.07)"
          }
          strokeWidth={1}
          style={{ transition: "stroke 0.4s ease" }}
        />
        {/* Inner accent ring */}
        <circle
          cx={CX}
          cy={CY}
          r={38}
          fill="none"
          stroke={
            hovered !== null
              ? "rgba(212,175,55,0.12)"
              : "rgba(255,255,255,0.03)"
          }
          strokeWidth={0.5}
          style={{ transition: "stroke 0.4s ease" }}
        />
        <text
          x={CX}
          y={CY - 3}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="white"
          fontSize={16}
          fontWeight={200}
          letterSpacing="-0.03em"
          style={{ fontFamily: "var(--font-geist-sans), sans-serif" }}
        >
          Nous
        </text>
        <text
          x={CX}
          y={CY + 15}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="#525252"
          fontSize={8}
          letterSpacing="0.15em"
          style={{ fontFamily: "var(--font-geist-mono), monospace" }}
        >
          CORE
        </text>

        {/* Subsystem nodes */}
        {subsystems.map((sys, i) => {
          const pos = positions[i];
          const isActive = hovered === i;

          return (
            <g
              key={sys.name}
              onMouseEnter={() => setHovered(i)}
              onMouseLeave={() => setHovered(null)}
              style={{ cursor: "default" }}
              className="arch-node"
              data-delay={i}
            >
              {/* Hover glow ring */}
              {isActive && (
                <circle
                  cx={pos.x}
                  cy={pos.y}
                  r={NODE_R + 12}
                  fill="url(#arch-node-glow)"
                />
              )}

              {/* Node circle */}
              <circle
                cx={pos.x}
                cy={pos.y}
                r={NODE_R}
                fill={
                  isActive
                    ? "rgba(212,175,55,0.05)"
                    : "rgba(255,255,255,0.015)"
                }
                stroke={
                  isActive
                    ? "rgba(212,175,55,0.4)"
                    : "rgba(255,255,255,0.05)"
                }
                strokeWidth={1}
                style={{ transition: "fill 0.3s ease, stroke 0.3s ease" }}
              />

              {/* Subsystem name */}
              <text
                x={pos.x}
                y={pos.y - 5}
                textAnchor="middle"
                dominantBaseline="middle"
                fill={isActive ? "white" : "#a3a3a3"}
                fontSize={12}
                fontWeight={isActive ? 400 : 300}
                letterSpacing="-0.01em"
                style={{
                  transition: "fill 0.3s ease",
                  fontFamily: "var(--font-geist-sans), sans-serif",
                }}
              >
                {sys.name}
              </text>

              {/* Tech tag */}
              <text
                x={pos.x}
                y={pos.y + 12}
                textAnchor="middle"
                dominantBaseline="middle"
                fill={isActive ? "#d4af37" : "#404040"}
                fontSize={8}
                letterSpacing="0.06em"
                style={{
                  transition: "fill 0.3s ease",
                  fontFamily: "var(--font-geist-mono), monospace",
                }}
              >
                {sys.tag}
              </text>
            </g>
          );
        })}
      </svg>

      {/* Mobile simplified view — visible only on very small screens */}
      <div className="grid grid-cols-2 gap-3 mt-6 sm:hidden">
        {subsystems.map((sys) => (
          <div
            key={sys.name}
            className="flex items-center gap-3 p-3 border border-white/[0.05] rounded-sm"
          >
            <div className="w-2 h-2 rounded-full bg-[#d4af37]/30 shrink-0" />
            <div>
              <p className="text-xs font-light text-neutral-300">
                {sys.name}
              </p>
              <p className="text-[9px] font-mono text-neutral-600">
                {sys.tag}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
