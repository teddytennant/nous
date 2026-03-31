"use client";

import { useCallback, useMemo, useRef, useState } from "react";
import type { BalanceEntry, TransactionResponse } from "@/lib/api";

// ── SVG Path Generation ──────────────────────────────────────────────────

const CHART_W = 600;
const CHART_H = 120;
const PAD = { top: 8, right: 8, bottom: 8, left: 8 };

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
};

function buildPaths(points: { x: number; y: number }[]): ChartMeta {
  if (points.length < 2) return EMPTY_META;

  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const minX = Math.min(...xs);
  const maxX = Math.max(...xs);
  const minY = Math.min(...ys, 0);
  const maxY = Math.max(...ys, 1);
  const rangeX = maxX - minX || 1;
  const rangeY = maxY - minY || 1;

  const cw = CHART_W - PAD.left - PAD.right;
  const ch = CHART_H - PAD.top - PAD.bottom;

  const scaled = points.map((p) => ({
    x: PAD.left + ((p.x - minX) / rangeX) * cw,
    y: PAD.top + ch - ((p.y - minY) / rangeY) * ch,
  }));

  let line = `M ${scaled[0].x} ${scaled[0].y}`;
  for (let i = 1; i < scaled.length; i++) {
    const prev = scaled[i - 1];
    const curr = scaled[i];
    const cpx = (prev.x + curr.x) / 2;
    line += ` C ${cpx} ${prev.y}, ${cpx} ${curr.y}, ${curr.x} ${curr.y}`;
  }

  const baseY = PAD.top + ch;
  const area = `${line} L ${scaled[scaled.length - 1].x} ${baseY} L ${scaled[0].x} ${baseY} Z`;

  return { line, area, scaled, minX, maxX, minY, maxY, rangeX, rangeY };
}

// ── Hover Interpolation ─────────────────────────────────────────────────

function interpolateAtX(
  svgX: number,
  points: { x: number; y: number }[],
  meta: ChartMeta,
): { dataX: number; dataY: number; svgY: number } | null {
  if (points.length < 2) return null;

  const cw = CHART_W - PAD.left - PAD.right;
  const ch = CHART_H - PAD.top - PAD.bottom;

  const dataX = meta.minX + ((svgX - PAD.left) / cw) * meta.rangeX;
  if (dataX < meta.minX || dataX > meta.maxX) return null;

  let i = 0;
  while (i < points.length - 1 && points[i + 1].x < dataX) i++;

  if (i >= points.length - 1) {
    const last = points[points.length - 1];
    const svgY = PAD.top + ch - ((last.y - meta.minY) / meta.rangeY) * ch;
    return { dataX: last.x, dataY: last.y, svgY };
  }

  const p0 = points[i];
  const p1 = points[i + 1];
  const t = p1.x === p0.x ? 0 : (dataX - p0.x) / (p1.x - p0.x);
  const dataY = p0.y + t * (p1.y - p0.y);
  const svgY = PAD.top + ch - ((dataY - meta.minY) / meta.rangeY) * ch;

  return { dataX, dataY, svgY };
}

function formatTooltipDate(ts: number): string {
  return new Date(ts).toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

// ── Token Colors ─────────────────────────────────────────────────────────

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

// ── Component ────────────────────────────────────────────────────────────

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

    let cumulative = 0;
    let inflow = 0;
    let outflow = 0;

    const points: { x: number; y: number }[] = [];

    const firstTime = new Date(sorted[0].timestamp).getTime();
    points.push({ x: firstTime - 86_400_000, y: 0 });

    for (const tx of sorted) {
      const amount = Number(tx.amount);
      const isReceive = tx.to_did === userDid;

      if (isReceive) {
        cumulative += amount;
        inflow += amount;
      } else {
        cumulative -= amount;
        outflow += amount;
      }

      points.push({ x: new Date(tx.timestamp).getTime(), y: cumulative });
    }

    return {
      chartPoints: points,
      totalIn: inflow,
      totalOut: outflow,
      txCount: sorted.length,
    };
  }, [transactions, userDid]);

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

  const meta = useMemo(() => buildPaths(chartPoints), [chartPoints]);

  const { line, area, scaled } = meta;
  const hasChart = chartPoints.length > 1;

  // ── Hover state ──────────────────────────────────────────────────
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

  return (
    <section className="mb-16">
      {/* ── Stats Row ─────────────────────────────────────── */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4 sm:gap-8 mb-6">
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

      {/* ── Sparkline Chart ───────────────────────────────── */}
      <div className="relative w-full mb-6">
        <div className="border border-white/[0.04] rounded-sm overflow-hidden">
          {hasChart ? (
            <svg
              ref={svgRef}
              viewBox={`0 0 ${CHART_W} ${CHART_H}`}
              preserveAspectRatio="none"
              className="w-full h-[120px] block cursor-crosshair"
              onMouseMove={handleMouseMove}
              onMouseLeave={clearHover}
              onTouchMove={handleTouchMove}
              onTouchEnd={clearHover}
            >
              <defs>
                <linearGradient id="wc-grad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor="#d4af37" stopOpacity="0.12" />
                  <stop offset="100%" stopColor="#d4af37" stopOpacity="0" />
                </linearGradient>
              </defs>

              {/* Subtle grid lines */}
              {[0.25, 0.5, 0.75].map((pct) => (
                <line
                  key={pct}
                  x1={PAD.left}
                  y1={PAD.top + (CHART_H - PAD.top - PAD.bottom) * pct}
                  x2={CHART_W - PAD.right}
                  y2={PAD.top + (CHART_H - PAD.top - PAD.bottom) * pct}
                  stroke="white"
                  strokeOpacity="0.04"
                  strokeWidth="1"
                />
              ))}

              {/* Area fill */}
              <path d={area} fill="url(#wc-grad)" />

              {/* Line */}
              <path
                d={line}
                fill="none"
                stroke="#d4af37"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
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
                        r="4"
                        fill="#d4af37"
                        opacity="0.2"
                      />
                      <circle cx={last.x} cy={last.y} r="2" fill="#d4af37" />
                    </>
                  );
                })()}

              {/* Hover crosshair + dot */}
              {hover && (
                <>
                  <line
                    x1={hover.svgX}
                    y1={PAD.top}
                    x2={hover.svgX}
                    y2={CHART_H - PAD.bottom}
                    stroke="white"
                    strokeOpacity="0.08"
                    strokeWidth="1"
                    strokeDasharray="2 2"
                  />
                  <circle
                    cx={hover.svgX}
                    cy={hover.svgY}
                    r="5"
                    fill="#d4af37"
                    opacity="0.15"
                  />
                  <circle
                    cx={hover.svgX}
                    cy={hover.svgY}
                    r="2.5"
                    fill="#d4af37"
                  />
                </>
              )}
            </svg>
          ) : (
            <div className="h-[120px] flex items-center justify-center">
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
              transform: "translate(-50%, calc(-100% - 12px))",
            }}
          >
            <div className="bg-neutral-900 border border-white/[0.08] rounded-sm px-2.5 py-1.5 shadow-xl whitespace-nowrap">
              <p className="text-xs font-light tabular-nums text-white">
                {hover.dataY.toFixed(2)}
              </p>
              <p className="text-[10px] font-mono text-neutral-600 mt-0.5">
                {formatTooltipDate(hover.dataX)}
              </p>
            </div>
          </div>
        )}
      </div>

      {/* ── Token Allocation Bar ──────────────────────────── */}
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
