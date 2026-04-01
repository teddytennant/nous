"use client";

import { useMemo } from "react";

// ── Types ──────────────────────────────────────────────────────────────────

interface SparklineProps {
  /** Array of numeric values to plot */
  data: number[];
  /** SVG width in px (default 80) */
  width?: number;
  /** SVG height in px (default 28) */
  height?: number;
  /** Stroke color (default white at 40% opacity) */
  strokeColor?: string;
  /** Stroke width (default 1.5) */
  strokeWidth?: number;
  /** Area fill color — set to "none" to disable (default matches stroke at 8% opacity) */
  fillColor?: string;
  /** Whether to show the endpoint dot (default true) */
  showDot?: boolean;
  /** Dot color (default matches stroke) */
  dotColor?: string;
  /** Whether the trend is positive (true), negative (false), or neutral (null) */
  trend?: boolean | null;
  /** Additional className for the svg element */
  className?: string;
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function buildPolyline(
  data: number[],
  width: number,
  height: number,
  pad: number,
): { points: string; lastX: number; lastY: number } {
  if (data.length < 2) {
    return { points: "", lastX: width / 2, lastY: height / 2 };
  }

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const cw = width - pad * 2;
  const ch = height - pad * 2;

  const pts: string[] = [];
  let lastX = 0;
  let lastY = 0;

  for (let i = 0; i < data.length; i++) {
    const x = pad + (i / (data.length - 1)) * cw;
    const y = pad + ch - ((data[i] - min) / range) * ch;
    pts.push(`${x.toFixed(1)},${y.toFixed(1)}`);
    lastX = x;
    lastY = y;
  }

  return { points: pts.join(" "), lastX, lastY };
}

function buildAreaPath(
  data: number[],
  width: number,
  height: number,
  pad: number,
): string {
  if (data.length < 2) return "";

  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min || 1;

  const cw = width - pad * 2;
  const ch = height - pad * 2;

  let d = "";
  for (let i = 0; i < data.length; i++) {
    const x = pad + (i / (data.length - 1)) * cw;
    const y = pad + ch - ((data[i] - min) / range) * ch;
    d += i === 0 ? `M${x.toFixed(1)},${y.toFixed(1)}` : ` L${x.toFixed(1)},${y.toFixed(1)}`;
  }

  // Close the area: go to bottom-right, then bottom-left
  d += ` L${(pad + cw).toFixed(1)},${(height - pad).toFixed(1)}`;
  d += ` L${pad.toFixed(1)},${(height - pad).toFixed(1)} Z`;

  return d;
}

// ── Component ───────────────────────────────────────────────────────────────

export function Sparkline({
  data,
  width = 80,
  height = 28,
  strokeColor,
  strokeWidth = 1.5,
  fillColor,
  showDot = true,
  dotColor,
  trend = null,
  className,
}: SparklineProps) {
  const pad = 3;

  // Auto-derive colors from trend
  const resolvedStroke = strokeColor
    ?? (trend === true
      ? "rgba(52, 211, 153, 0.6)"   // emerald
      : trend === false
        ? "rgba(239, 68, 68, 0.5)"  // red
        : "rgba(255, 255, 255, 0.35)"); // neutral white

  const resolvedFill = fillColor
    ?? (trend === true
      ? "rgba(52, 211, 153, 0.08)"
      : trend === false
        ? "rgba(239, 68, 68, 0.06)"
        : "rgba(255, 255, 255, 0.04)");

  const resolvedDot = dotColor ?? resolvedStroke;

  const { points, lastX, lastY } = useMemo(
    () => buildPolyline(data, width, height, pad),
    [data, width, height],
  );

  const areaPath = useMemo(
    () => (resolvedFill !== "none" ? buildAreaPath(data, width, height, pad) : ""),
    [data, width, height, resolvedFill],
  );

  if (data.length < 2) return null;

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      fill="none"
      className={`sparkline-enter ${className ?? ""}`}
      aria-hidden="true"
    >
      {/* Area fill */}
      {areaPath && (
        <path
          d={areaPath}
          fill={resolvedFill}
          className="sparkline-area-enter"
        />
      )}

      {/* Line */}
      <polyline
        points={points}
        fill="none"
        stroke={resolvedStroke}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeLinejoin="round"
        className="sparkline-line-enter"
      />

      {/* Endpoint dot */}
      {showDot && (
        <circle
          cx={lastX}
          cy={lastY}
          r={2}
          fill={resolvedDot}
          className="sparkline-dot-enter"
        />
      )}
    </svg>
  );
}

// ── Mini Bar Chart Variant ─────────────────────────────────────────────────

interface MiniBarChartProps {
  data: number[];
  width?: number;
  height?: number;
  barColor?: string;
  activeBarColor?: string;
  className?: string;
}

export function MiniBarChart({
  data,
  width = 80,
  height = 28,
  barColor = "rgba(255, 255, 255, 0.08)",
  activeBarColor = "rgba(212, 175, 55, 0.5)",
  className,
}: MiniBarChartProps) {
  if (data.length === 0) return null;

  const max = Math.max(...data, 1);
  const pad = 3;
  const gap = 2;
  const cw = width - pad * 2;
  const ch = height - pad * 2;
  const barWidth = Math.max(1, (cw - gap * (data.length - 1)) / data.length);

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      fill="none"
      className={`sparkline-enter ${className ?? ""}`}
      aria-hidden="true"
    >
      {data.map((value, i) => {
        const barH = Math.max(1, (value / max) * ch);
        const x = pad + i * (barWidth + gap);
        const y = pad + ch - barH;
        const isLast = i === data.length - 1;

        return (
          <rect
            key={i}
            x={x}
            y={y}
            width={barWidth}
            height={barH}
            rx={0.5}
            fill={isLast ? activeBarColor : barColor}
            className="sparkline-bar-enter"
            style={{ animationDelay: `${i * 30}ms` }}
          />
        );
      })}
    </svg>
  );
}
