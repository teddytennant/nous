"use client";

import { useState, useCallback, type ReactNode } from "react";
import { cn } from "@/lib/utils";
import { ArrowUp, ArrowDown, ArrowUpDown } from "lucide-react";

// ── Types ───────────────────────────────────────────────────────────────

type SortDir = "asc" | "desc";

interface Column<T> {
  /** Unique key for this column */
  id: string;
  /** Column header label */
  header: string;
  /** Render the cell content for a row */
  cell: (row: T, index: number) => ReactNode;
  /** Enable sorting for this column */
  sortable?: boolean;
  /** Sort comparator. Return negative if a < b. Required if sortable. */
  compare?: (a: T, b: T) => number;
  /** Text alignment */
  align?: "left" | "center" | "right";
  /** Hide on smaller screens */
  hideBelow?: "sm" | "md" | "lg" | "xl";
  /** Additional className for header and cells */
  className?: string;
  /** Minimum width */
  minWidth?: string;
}

interface DataTableProps<T> {
  /** Column definitions */
  columns: Column<T>[];
  /** Row data */
  data: T[];
  /** Unique key extractor for each row */
  rowKey: (row: T, index: number) => string;
  /** Optional: initial sort column id */
  defaultSortId?: string;
  /** Optional: initial sort direction */
  defaultSortDir?: SortDir;
  /** Optional: callback when a row is clicked */
  onRowClick?: (row: T, index: number) => void;
  /** Optional: determine if a row is "selected" */
  isRowSelected?: (row: T) => boolean;
  /** Content to show when data is empty */
  emptyState?: ReactNode;
  /** Optional className for the table wrapper */
  className?: string;
  /** Whether to show a subtle stagger animation on mount */
  stagger?: boolean;
}

// ── Sort indicator ──────────────────────────────────────────────────────

function SortIcon({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) {
    return <ArrowUpDown className="w-3 h-3 text-neutral-800 ml-1 inline-block" />;
  }
  if (dir === "asc") {
    return <ArrowUp className="w-3 h-3 text-[#d4af37] ml-1 inline-block" />;
  }
  return <ArrowDown className="w-3 h-3 text-[#d4af37] ml-1 inline-block" />;
}

// ── Responsive hide classes ─────────────────────────────────────────────

function getHideClass(hideBelow?: "sm" | "md" | "lg" | "xl"): string {
  if (!hideBelow) return "";
  const map = {
    sm: "hidden sm:table-cell",
    md: "hidden md:table-cell",
    lg: "hidden lg:table-cell",
    xl: "hidden xl:table-cell",
  };
  return map[hideBelow];
}

function getAlignClass(align?: "left" | "center" | "right"): string {
  if (!align || align === "left") return "text-left";
  if (align === "center") return "text-center";
  return "text-right";
}

// ── DataTable ───────────────────────────────────────────────────────────

function DataTable<T>({
  columns,
  data,
  rowKey,
  defaultSortId,
  defaultSortDir = "asc",
  onRowClick,
  isRowSelected,
  emptyState,
  className,
  stagger = true,
}: DataTableProps<T>) {
  const [sortId, setSortId] = useState<string | undefined>(defaultSortId);
  const [sortDir, setSortDir] = useState<SortDir>(defaultSortDir);

  const handleSort = useCallback(
    (colId: string) => {
      if (sortId === colId) {
        setSortDir((d) => (d === "asc" ? "desc" : "asc"));
      } else {
        setSortId(colId);
        setSortDir("asc");
      }
    },
    [sortId],
  );

  // Apply sorting
  const sortedData = (() => {
    if (!sortId) return data;
    const col = columns.find((c) => c.id === sortId);
    if (!col?.compare) return data;
    const sorted = [...data].sort(col.compare);
    return sortDir === "desc" ? sorted.reverse() : sorted;
  })();

  if (sortedData.length === 0 && emptyState) {
    return <>{emptyState}</>;
  }

  return (
    <div className={cn("overflow-x-auto", className)}>
      <table className="w-full border-collapse">
        <thead>
          <tr className="border-b border-white/[0.06]">
            {columns.map((col) => {
              const isSortable = col.sortable && col.compare;
              return (
                <th
                  key={col.id}
                  className={cn(
                    "pb-3 pr-4 last:pr-0",
                    "text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 font-normal",
                    "whitespace-nowrap select-none",
                    getAlignClass(col.align),
                    getHideClass(col.hideBelow),
                    isSortable && "cursor-pointer hover:text-neutral-400 transition-colors duration-150",
                    col.className,
                  )}
                  style={col.minWidth ? { minWidth: col.minWidth } : undefined}
                  onClick={isSortable ? () => handleSort(col.id) : undefined}
                >
                  {col.header}
                  {isSortable && (
                    <SortIcon active={sortId === col.id} dir={sortDir} />
                  )}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody className={stagger ? "stagger-in" : undefined}>
          {sortedData.map((row, i) => {
            const key = rowKey(row, i);
            const selected = isRowSelected?.(row) ?? false;

            return (
              <tr
                key={key}
                data-list-item
                className={cn(
                  "border-b border-white/[0.03] transition-colors duration-100 group",
                  onRowClick && "cursor-pointer",
                  selected
                    ? "bg-white/[0.03]"
                    : "hover:bg-white/[0.015]",
                )}
                onClick={onRowClick ? () => onRowClick(row, i) : undefined}
              >
                {columns.map((col) => (
                  <td
                    key={col.id}
                    className={cn(
                      "py-3 pr-4 last:pr-0",
                      getAlignClass(col.align),
                      getHideClass(col.hideBelow),
                      col.className,
                    )}
                  >
                    {col.cell(row, i)}
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export { DataTable, type Column, type DataTableProps, type SortDir };
