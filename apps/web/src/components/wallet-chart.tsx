"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import type { BalanceEntry, TransactionResponse } from "@/lib/api";
import { cn } from "@/lib/utils";

// ── Chart Dimensions ────────────────────────────────────────────────────

const CHART_W = 700;
const CHART_H = 200;
const PAD = { top: 16, right: 56, bottom: 32, left: 56 };

const INNER_W = CHART_W - PAD.left - PAD.right;
const INNER_H = CHART_H - PAD.top - PAD.bottom;

// ── Time Ranges ─────────────────────────────────────────────────────────

type TimeRange = "7D" | "30D" | "90D" | "1Y" | "All";

const TIME_RANGES: TimeRange[] = ["7D", "30D", "90D", "1Y", "All"];

function rangeToMs(range: TimeRange): number | null {
  switch (range) {
    case "7D":
      return 7 * 86_400_000;
    case "30D":
      return 30 * 86_400_000;
    case "90D":
      return 90 * 86_400_000;
    case "1Y":
      return 365 * 86_400_000;
    case "All":
      return null;
  }
}

// ── SVG Path Generation ─────────────────────────────────────────────────

interface ChartMeta {
  line: string;
  area: string;
  scaled: { x: number; y: number }[];
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  rangeX: number;
  rangeY: number;
  pathLength: number;
}

const EMPTY_META: ChartMeta = {
  line: "",
  area: "",
  scaled: [],
  minX: 0,
  maxX: 0,
  minY: 0,
  maxY: 0,
  rangeX: 1,
  rangeY: 1,
  pathLength: 0,
};

function buildPaths(points: { x: number; y: number }[]): ChartMeta {
  if (points.length < 2) return EMPTY_META;

  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const rawMinY = Math.min(...ys);
  const rawMaxY = Math.max(...ys);

  // Add 10% padding to Y range
  const yPadding = (rawMaxY - rawMinY) * 0.1 || 1;
  const minY = rawMinY - yPadding;
  const maxY = rawMaxY + yPadding;
  const rangeX = maxX - minX || 1;
  const rangeY = maxY - minY || 1;

  const scaled = points.map((p) => ({
    x: PAD.left + ((p.x - minX) / rangeX) * INNER_W,
    y: PAD.top + INNER_H - ((p.y - minY) / rangeY) * INNER_H,
  }));

  // Smooth cubic bezier path
  let line = `M ${scaled[0].x} ${scaled[0].y}`;
  for (let i = 1; i < scaled.length; i++) {
    const prev = scaled[i - 1];
    const curr = scaled[i];
    const cpx = (prev.x + curr.x) / 2;
    line += ` C ${cpx} ${prev.y}, ${cpx} ${curr.y}, ${curr.x} ${curr.y}`;
  }

  const baseY = PAD.top + INNER_H;
  const area = `${line} L ${scaled[scaled.length - 1].x} ${baseY} L ${scaled[0].x} ${baseY} Z`;

  // Estimate path length for stroke animation
  let pathLength = 0;
  for (let i = 1; i < scaled.length; i++) {
    const dx = scaled[i].x - scaled[i - 1].x;
    const dy = scaled[i].y - scaled[i - 1].y;
    pathLength += Math.sqrt(dx * dx + dy * dy);
  }

  return { line, area, scaled, minX, maxX, minY, maxY, rangeX, rangeY, pathLength };
}

// ── Hover Interpolation ─────────────────────────────────────────────────

function interpolateAtX(
  svgX: number,
  points: { x: number; y: number }[],
  meta: ChartMeta,
): { dataX: number; dataY: number; svgY: number } | null {
  if (points.length < 2) return null;

  const dataX = meta.minX + ((svgX - PAD.left) / INNER_W) * meta.rangeX;
  if (dataX < meta.minX || dataX > meta.maxX) return null;

  let i = 0;
  while (i < points.length - 1 && points[i + 1].x < dataX) i++;

  if (i >= points.length - 1) {
    const last = points[points.length - 1];
    const svgY = PAD.top + INNER_H - ((last.y - meta.minY) / meta.rangeY) * INNER_H;
    return { dataX: last.x, dataY: last.y, svgY };
  }

  const p0 = points[i];
  const p1 = points[i + 1];
  const t = p1.x === p0.x ? 0 : (dataX - p0.x) / (p1.x - p0.x);
  const dataY = p0.y + t * (p1.y - p0.y);
  const svgY = PAD.top + INNER_H - ((dataY - meta.minY) / meta.rangeY) * INNER_H;

  return { dataX, dataY, svgY };
}

// ── Formatting ──────────────────────────────────────────────────────────

function formatTooltipDate(ts: number): string {
  return new Date(ts).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formatAxisDate(ts: number, range: TimeRange): string {
  const d = new Date(ts);
  if (range === "7D") {
    return d.toLocaleDateString("en-US", { weekday: "short" });
  }
  if (range === "30D" || range === "90D") {
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }
  return d.toLocaleDateString("en-US", { month: "short", year: "2-digit" });
}

function formatYAxis(value: number): string {
  if (Math.abs(value) >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (Math.abs(value) >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  if (Math.abs(value) >= 100) return value.toFixed(0);
  if (Math.abs(value) >= 1) return value.toFixed(1);
  return value.toFixed(2);
}

// ── Token Colors ────────────────────────────────────────────────────────

const TOKEN_COLORS: Record<string, string> = {
  ETH: "#627eea",
  NOUS: "#d4af37",
  USDC: "#2775ca",
};

function tokenColor(token: string, index: number): string {
  if (TOKEN_COLORS[token]) return TOKEN_COLORS[token];
  const fallback = ["#6b7280", "#8b5cf6", "#10b981", "#f59e0b"];
  return fallback[index % fallback.length];
}

// ── Component ───────────────────────────────────────────────────────────

interface WalletChartProps {
  balances: BalanceEntry[];
  transactions: TransactionResponse[];
  userDid: string;
}

export function WalletChart({
  balances,
  transactions,
  userDid,
}: WalletChartProps) {
  const [timeRange, setTimeRange] = useState<TimeRange>("All");
  const lineRef = useRef<SVGPathElement>(null);

  // Animation key: changes on timeRange to trigger CSS re-mount
  const animKey = `chart-${timeRange}`;

  const { chartPoints, totalIn, totalOut, txCount } = useMemo(() => {
    if (transactions.length === 0) {
      return {
        chartPoints: [] as { x: number; y: number }[],
        totalIn: 0,
        totalOut: 0,
        txCount: 0,
      };
    }

    const sorted = [...transactions].sort(
      (a, b) =>
        new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime(),
    );

    // Filter by time range — use the most recent transaction as the reference
    // point rather than Date.now() to keep this computation pure.
    const rangeMs = rangeToMs(timeRange);
    const refTs = new Date(sorted[sorted.length - 1].timestamp).getTime();
    const filtered = rangeMs
      ? sorted.filter((tx) => refTs - new Date(tx.timestamp).getTime() <= rangeMs)
      : sorted;

    // If no transactions in range, still show the period
    const txsToUse = filtered.length > 0 ? filtered : sorted;

    let cumulative = 0;
    let inflow = 0;
    let outflow = 0;

    // Compute cumulative balance up to the start of filtered period
    if (rangeMs && filtered.length > 0) {
      const cutoff = refTs - rangeMs;
      for (const tx of sorted) {
        if (new Date(tx.timestamp).getTime() >= cutoff) break;
        const amount = Number(tx.amount);
        if (tx.to_did === userDid) {
          cumulative += amount;
        } else {
          cumulative -= amount;
        }
      }
    }

    const startCumulative = cumulative;
    const points: { x: number; y: number }[] = [];

    // Add starting point
    if (txsToUse.length > 0) {
      const firstTime = new Date(txsToUse[0].timestamp).getTime();
      points.push({ x: firstTime - 86_400_000, y: cumulative });
    }

    for (const tx of txsToUse) {
      const amount = Number(tx.amount);
      const isReceive = tx.to_did === userDid;

      if (isReceive) {
        cumulative += amount;
        if (rangeMs ? filtered.includes(tx) : true) inflow += amount;
      } else {
        cumulative -= amount;
        if (rangeMs ? filtered.includes(tx) : true) outflow += amount;
      }

      points.push({ x: new Date(tx.timestamp).getTime(), y: cumulative });
    }

    return {
      chartPoints: points,
      totalIn: inflow,
      totalOut: outflow,
      txCount: rangeMs ? filtered.length : sorted.length,
      startCumulative,
    };
  }, [transactions, userDid, timeRange]);

  const totalValue = useMemo(
    () => balances.reduce((sum, b) => sum + Number(b.amount), 0),
    [balances],
  );

  const allocation = useMemo(() => {
    const total = balances.reduce((sum, b) => sum + Number(b.amount), 0);
    if (total === 0) {
      return balances.map((b) => ({
        token: b.token,
        pct: 100 / (balances.length || 1),
      }));
    }
    return balances.map((b) => ({
      token: b.token,
      pct: (Number(b.amount) / total) * 100,
    }));
  }, [balances]);

  // Percentage change
  const pctChange = useMemo(() => {
    if (chartPoints.length < 2) return null;
    const startVal = chartPoints[0].y;
    const endVal = chartPoints[chartPoints.length - 1].y;
    if (startVal === 0) return endVal > 0 ? 100 : endVal < 0 ? -100 : 0;
    return ((endVal - startVal) / Math.abs(startVal)) * 100;
  }, [chartPoints]);

  const meta = useMemo(() => buildPaths(chartPoints), [chartPoints]);

  const { line, area, scaled } = meta;
  const hasChart = chartPoints.length > 1;

  // ── Y-axis labels ─────────────────────────────────────────────────
  const yLabels = useMemo(() => {
    if (!hasChart) return [];
    const steps = 4;
    const labels: { value: number; y: number }[] = [];
    for (let i = 0; i <= steps; i++) {
      const value = meta.minY + (meta.rangeY * i) / steps;
      const y = PAD.top + INNER_H - (INNER_H * i) / steps;
      labels.push({ value, y });
    }
    return labels;
  }, [hasChart, meta]);

  // ── X-axis labels ─────────────────────────────────────────────────
  const xLabels = useMemo(() => {
    if (!hasChart) return [];
    const count = timeRange === "7D" ? 7 : 5;
    const labels: { ts: number; x: number }[] = [];
    for (let i = 0; i <= count; i++) {
      const ts = meta.minX + (meta.rangeX * i) / count;
      const x = PAD.left + (INNER_W * i) / count;
      labels.push({ ts, x });
    }
    return labels;
  }, [hasChart, meta, timeRange]);

  // ── Hover state ───────────────────────────────────────────────────
  const svgRef = useRef<SVGSVGElement>(null);
  const [hover, setHover] = useState<{
    pctX: number;
    svgX: number;
    svgY: number;
    dataX: number;
    dataY: number;
  } | null>(null);

  const updateHover = useCallback(
    (clientX: number) => {
      const svg = svgRef.current;
      if (!svg || chartPoints.length < 2) return;

      const rect = svg.getBoundingClientRect();
      const pctX = (clientX - rect.left) / rect.width;
      const svgX = pctX * CHART_W;

      if (svgX < PAD.left || svgX > CHART_W - PAD.right) {
        setHover(null);
        return;
      }

      const result = interpolateAtX(svgX, chartPoints, meta);
      if (result) {
        setHover({
          pctX,
          svgX,
          svgY: result.svgY,
          dataX: result.dataX,
          dataY: result.dataY,
        });
      }
    },
    [chartPoints, meta],
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => updateHover(e.clientX),
    [updateHover],
  );

  const handleTouchMove = useCallback(
    (e: React.TouchEvent<SVGSVGElement>) => {
      const touch = e.touches[0];
      if (touch) updateHover(touch.clientX);
    },
    [updateHover],
  );

  const clearHover = useCallback(() => setHover(null), []);

  // Determine line/area color based on performance
  const isPositive = pctChange === null || pctChange >= 0;
  const lineColor = isPositive ? "#d4af37" : "#ef4444";
  const gradientId = isPositive ? "wc-grad-pos" : "wc-grad-neg";

  return (
    <section className="mb-16">
      {/* ── Stats Row + Time Range ────────────────────────── */}
      <div className="flex flex-col sm:flex-row sm:items-end justify-between gap-4 mb-6">
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 sm:gap-8 flex-1">
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
              Portfolio
            </p>
            <p className="text-xl font-extralight mt-1 tabular-nums">
              {totalValue.toFixed(2)}
            </p>
          </div>
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
              Inflows
            </p>
            <p className="text-xl font-extralight mt-1 tabular-nums text-emerald-500/80">
              +{totalIn.toFixed(2)}
            </p>
          </div>
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
              Outflows
            </p>
            <p className="text-xl font-extralight mt-1 tabular-nums text-red-400/80">
              -{totalOut.toFixed(2)}
            </p>
          </div>
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600">
              Transactions
            </p>
            <p className="text-xl font-extralight mt-1 tabular-nums">
              {txCount}
            </p>
          </div>
        </div>

        {/* Time range selector */}
        <div className="flex gap-1 shrink-0">
          {TIME_RANGES.map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={cn(
                "text-[10px] font-mono uppercase tracking-wider px-3 py-1.5 transition-all duration-150",
                timeRange === range
                  ? "text-white bg-white/[0.08]"
                  : "text-neutral-700 hover:text-neutral-400"
              )}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* ── Chart ────────────────────────────────────────── */}
      <div className="relative w-full mb-6">
        <div className="border border-white/[0.04] rounded-sm overflow-hidden">
          {hasChart ? (
            <svg
              key={animKey}
              ref={svgRef}
              viewBox={`0 0 ${CHART_W} ${CHART_H}`}
              preserveAspectRatio="none"
              className="w-full h-[200px] block cursor-crosshair"
              onMouseMove={handleMouseMove}
              onMouseLeave={clearHover}
              onTouchMove={handleTouchMove}
              onTouchEnd={clearHover}
            >
              <defs>
                <linearGradient id="wc-grad-pos" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#d4af37" stopOpacity="0.12" />
                  <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
                </linearGradient>
                <linearGradient id="wc-grad-neg" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#ef4444" stopOpacity="0.12" />
                  <stop offset="100%" stopColor="#ef4444" stopOpacity="0" />
                </linearGradient>
              </defs>

              {/* Horizontal grid lines + Y-axis labels */}
              {yLabels.map((label, i) => (
                <g key={i}>
                  <line
                    x1={PAD.left}
                    y1={label.y}
                    x2={CHART_W - PAD.right}
                    y2={label.y}
                    stroke="white"
                    strokeOpacity="0.04"
                    strokeWidth="1"
                  />
                  <text
                    x={PAD.left - 8}
                    y={label.y + 3}
                    textAnchor="end"
                    className="fill-neutral-700"
                    style={{ fontSize: "9px", fontFamily: "var(--font-mono)" }}
                  >
                    {formatYAxis(label.value)}
                  </text>
                </g>
              ))}

              {/* X-axis labels */}
              {xLabels.map((label, i) => (
                <text
                  key={i}
                  x={label.x}
                  y={CHART_H - 6}
                  textAnchor="middle"
                  className="fill-neutral-700"
                  style={{ fontSize: "9px", fontFamily: "var(--font-mono)" }}
                >
                  {formatAxisDate(label.ts, timeRange)}
                </text>
              ))}

              {/* Area fill — fades in via CSS */}
              <path
                d={area}
                fill={`url(#${gradientId})`}
                className="wallet-chart-area-enter"
              />

              {/* Animated line — stroke draws in via CSS */}
              <path
                ref={lineRef}
                d={line}
                fill="none"
                stroke={lineColor}
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                className="wallet-chart-line-draw"
                style={
                  meta.pathLength > 0
                    ? {
                        strokeDasharray: meta.pathLength,
                        strokeDashoffset: meta.pathLength,
                      }
                    : undefined
                }
              />

              {/* End dot — hidden during hover */}
              {scaled.length > 0 &&
                !hover &&
                (() => {
                  const last = scaled[scaled.length - 1];
                  return (
                    <>
                      <circle
                        cx={last.x}
                        cy={last.y}
                        r="5"
                        fill={lineColor}
                        opacity="0.15"
                        className="wallet-chart-dot-pulse"
                      />
                      <circle cx={last.x} cy={last.y} r="2.5" fill={lineColor} />
                    </>
                  );
                })()}

              {/* Hover crosshair + dot */}
              {hover && (
                <>
                  {/* Vertical line */}
                  <line
                    x1={hover.svgX}
                    y1={PAD.top}
                    x2={hover.svgX}
                    y2={PAD.top + INNER_H}
                    stroke="white"
                    strokeOpacity="0.08"
                    strokeWidth="1"
                    strokeDasharray="2 2"
                  />
                  {/* Horizontal line */}
                  <line
                    x1={PAD.left}
                    y1={hover.svgY}
                    x2={CHART_W - PAD.right}
                    y2={hover.svgY}
                    stroke="white"
                    strokeOpacity="0.04"
                    strokeWidth="1"
                    strokeDasharray="2 2"
                  />
                  <circle
                    cx={hover.svgX}
                    cy={hover.svgY}
                    r="6"
                    fill={lineColor}
                    opacity="0.12"
                  />
                  <circle
                    cx={hover.svgX}
                    cy={hover.svgY}
                    r="3"
                    fill={lineColor}
                  />
                </>
              )}
            </svg>
          ) : (
            <div className="h-[200px] flex items-center justify-center">
              <p className="text-xs text-neutral-700 font-light">
                Transaction history will appear here
              </p>
            </div>
          )}
        </div>

        {/* Hover tooltip */}
        {hover && (
          <div
            className="absolute pointer-events-none z-10"
            style={{
              left: `${hover.pctX * 100}%`,
              top: `${(hover.svgY / CHART_H) * 100}%`,
              transform: "translate(-50%, calc(-100% - 14px))",
            }}
          >
            <div className="bg-neutral-900 border border-white/[0.08] rounded-sm px-3 py-2 shadow-xl whitespace-nowrap">
              <p className="text-xs font-light tabular-nums text-white">
                {hover.dataY.toFixed(2)}
              </p>
              <p className="text-[10px] font-mono text-neutral-600 mt-0.5">
                {formatTooltipDate(hover.dataX)}
              </p>
            </div>
          </div>
        )}

        {/* Change indicator */}
        {pctChange !== null && hasChart && (
          <div className="absolute top-3 right-3">
            <div className={cn(
              "flex items-center gap-1 px-2 py-1 rounded-sm text-[10px] font-mono tabular-nums",
              isPositive
                ? "text-[#d4af37] bg-[#d4af37]/[0.06]"
                : "text-red-400 bg-red-400/[0.06]"
            )}>
              <svg
                viewBox="0 0 10 10"
                className={cn("w-2.5 h-2.5", !isPositive && "rotate-180")}
                fill="currentColor"
              >
                <path d="M5 1L9 7H1L5 1Z" />
              </svg>
              {Math.abs(pctChange).toFixed(1)}%
            </div>
          </div>
        )}
      </div>

      {/* ── Token Allocation Bar ─────────────────────────── */}
      {balances.length > 0 && (
        <div>
          <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
            Allocation
          </p>
          <div className="flex h-1.5 rounded-sm overflow-hidden bg-white/[0.03]">
            {allocation.map((t, i) => (
              <div
                key={t.token}
                className="transition-all duration-500 ease-out"
                style={{
                  width: `${t.pct}%`,
                  backgroundColor: tokenColor(t.token, i),
                  opacity: Number(balances[i]?.amount) > 0 ? 1 : 0.15,
                }}
              />
            ))}
          </div>
          <div className="flex gap-6 mt-2.5">
            {allocation.map((t, i) => (
              <div key={t.token} className="flex items-center gap-2">
                <div
                  className="w-1.5 h-1.5 rounded-full"
                  style={{ backgroundColor: tokenColor(t.token, i) }}
                />
                <span className="text-[10px] font-mono text-neutral-600">
                  {t.token} {t.pct.toFixed(0)}%
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
