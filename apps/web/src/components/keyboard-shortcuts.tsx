"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter, usePathname } from "next/navigation";

// ── Shortcut definitions ─────────────────────────────────────────────────

interface Shortcut {
  keys: string[];
  label: string;
}

interface ShortcutGroup {
  title: string;
  shortcuts: Shortcut[];
}

const shortcutGroups: ShortcutGroup[] = [
  {
    title: "General",
    shortcuts: [
      { keys: ["⌘", "K"], label: "Command palette" },
      { keys: ["?"], label: "Keyboard shortcuts" },
      { keys: ["Esc"], label: "Close modal / dismiss" },
    ],
  },
  {
    title: "Lists",
    shortcuts: [
      { keys: ["J"], label: "Next item" },
      { keys: ["K"], label: "Previous item" },
      { keys: ["↵"], label: "Activate selected item" },
      { keys: ["Esc"], label: "Clear selection" },
    ],
  },
  {
    title: "Navigation",
    shortcuts: [
      { keys: ["G", "D"], label: "Go to Dashboard" },
      { keys: ["G", "S"], label: "Go to Social" },
      { keys: ["G", "M"], label: "Go to Messages" },
      { keys: ["G", "W"], label: "Go to Wallet" },
      { keys: ["G", "K"], label: "Go to Marketplace" },
      { keys: ["G", "G"], label: "Go to Governance" },
      { keys: ["G", "A"], label: "Go to AI" },
      { keys: ["G", "F"], label: "Go to Files" },
      { keys: ["G", "N"], label: "Go to Network" },
      { keys: ["G", "I"], label: "Go to Identity" },
      { keys: ["G", "E"], label: "Go to Settings" },
    ],
  },
];

// ── Page-specific shortcut definitions ──────────────────────────────────

const pageShortcutGroups: Record<string, ShortcutGroup> = {
  "/social": {
    title: "Social",
    shortcuts: [
      { keys: ["N"], label: "New post (focus composer)" },
      { keys: ["R"], label: "Refresh feed" },
    ],
  },
  "/messages": {
    title: "Messages",
    shortcuts: [
      { keys: ["N"], label: "New conversation" },
    ],
  },
  "/wallet": {
    title: "Wallet",
    shortcuts: [
      { keys: ["S"], label: "Send tokens" },
    ],
  },
  "/governance": {
    title: "Governance",
    shortcuts: [
      { keys: ["P"], label: "New proposal" },
      { keys: ["D"], label: "New DAO" },
    ],
  },
  "/ai": {
    title: "AI",
    shortcuts: [
      { keys: ["N"], label: "New agent" },
    ],
  },
  "/files": {
    title: "Files",
    shortcuts: [
      { keys: ["U"], label: "Upload file" },
    ],
  },
  "/marketplace": {
    title: "Marketplace",
    shortcuts: [
      { keys: ["N"], label: "New listing" },
    ],
  },
};

const goToMap: Record<string, string> = {
  d: "/dashboard",
  s: "/social",
  m: "/messages",
  w: "/wallet",
  k: "/marketplace",
  g: "/governance",
  a: "/ai",
  f: "/files",
  n: "/network",
  i: "/identity",
  e: "/settings",
};

// ── Hook: Page-specific keyboard shortcuts ──────────────────────────────

export type PageShortcutMap = Record<string, () => void>;

export function usePageShortcuts(shortcuts: PageShortcutMap) {
  const shortcutsRef = useRef(shortcuts);
  useEffect(() => {
    shortcutsRef.current = shortcuts;
  });

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const isInput =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable;

      if (isInput) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;

      const handler = shortcutsRef.current[e.key.toLowerCase()];
      if (handler) {
        e.preventDefault();
        handler();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}

// ── Hook: List navigation (J/K/Enter) ─────────────────────────────────────

export function useListNavigation({
  itemCount,
  onActivate,
  enabled = true,
}: {
  itemCount: number;
  onActivate?: (index: number) => void;
  enabled?: boolean;
}) {
  const [rawSelected, setRawSelected] = useState(-1);
  const selectedIndex =
    itemCount === 0 ? -1 : Math.min(rawSelected, itemCount - 1);
  const containerRef = useRef<HTMLDivElement>(null);
  const onActivateRef = useRef(onActivate);
  const selectedRef = useRef(selectedIndex);

  useEffect(() => {
    onActivateRef.current = onActivate;
    selectedRef.current = selectedIndex;
  });

  useEffect(() => {
    if (!enabled) return;

    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const isInput =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable;
      if (isInput) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (itemCount === 0) return;

      const key = e.key.toLowerCase();

      if (key === "j") {
        e.preventDefault();
        setRawSelected((prev) => {
          const next = prev < itemCount - 1 ? prev + 1 : 0;
          requestAnimationFrame(() => {
            const items =
              containerRef.current?.querySelectorAll("[data-list-item]");
            items?.[next]?.scrollIntoView({
              block: "nearest",
              behavior: "smooth",
            });
          });
          return next;
        });
      } else if (key === "k") {
        e.preventDefault();
        setRawSelected((prev) => {
          const next = prev > 0 ? prev - 1 : itemCount - 1;
          requestAnimationFrame(() => {
            const items =
              containerRef.current?.querySelectorAll("[data-list-item]");
            items?.[next]?.scrollIntoView({
              block: "nearest",
              behavior: "smooth",
            });
          });
          return next;
        });
      } else if (
        key === "enter" &&
        selectedRef.current >= 0 &&
        onActivateRef.current
      ) {
        e.preventDefault();
        onActivateRef.current(selectedRef.current);
      } else if (key === "escape" && selectedRef.current >= 0) {
        setRawSelected(-1);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [enabled, itemCount]);

  return { selectedIndex, setSelectedIndex: setRawSelected, containerRef };
}

// ── Hook: Global keyboard shortcuts ──────────────────────────────────────

export function useKeyboardShortcuts(
  onOpenHelp: () => void,
) {
  const router = useRouter();
  const pendingG = useRef(false);
  const gTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement;
      const isInput =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.tagName === "SELECT" ||
        target.isContentEditable;

      if (isInput) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;

      // ? — open help
      if (e.key === "?") {
        e.preventDefault();
        onOpenHelp();
        return;
      }

      // G + <key> navigation
      if (pendingG.current) {
        const dest = goToMap[e.key.toLowerCase()];
        if (dest) {
          e.preventDefault();
          router.push(dest);
        }
        pendingG.current = false;
        if (gTimer.current) clearTimeout(gTimer.current);
        gTimer.current = null;
        return;
      }

      if (e.key === "g") {
        pendingG.current = true;
        gTimer.current = setTimeout(() => {
          pendingG.current = false;
          gTimer.current = null;
        }, 500);
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (gTimer.current) clearTimeout(gTimer.current);
    };
  }, [router, onOpenHelp]);
}

// ── Modal component ──────────────────────────────────────────────────────

function ShortcutRow({ shortcut }: { shortcut: Shortcut }) {
  return (
    <div className="flex items-center justify-between py-2">
      <span className="text-xs text-neutral-400 font-light">
        {shortcut.label}
      </span>
      <div className="flex items-center gap-1">
        {shortcut.keys.map((key, i) => (
          <span key={i}>
            <kbd className="inline-flex items-center justify-center min-w-[1.5rem] px-1.5 py-0.5 text-[10px] font-mono text-neutral-400 bg-white/[0.04] border border-white/[0.06] rounded">
              {key}
            </kbd>
            {i < shortcut.keys.length - 1 && (
              <span className="text-neutral-700 text-[10px] mx-0.5">
                then
              </span>
            )}
          </span>
        ))}
      </div>
    </div>
  );
}

export function KeyboardShortcutsModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const pathname = usePathname();

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", handleKeyDown, { capture: true });
  }, [open, onClose]);

  if (!open) return null;

  const pageGroup = pageShortcutGroups[pathname];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm cmd-backdrop-enter"
        onClick={onClose}
      />

      {/* Dialog */}
      <div className="relative w-full max-w-md mx-4 cmd-dialog-enter">
        <div className="overflow-hidden rounded-md border border-white/[0.08] bg-neutral-950 shadow-2xl shadow-black/50">
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-white/[0.06]">
            <h2 className="text-sm font-medium">Keyboard Shortcuts</h2>
            <kbd className="text-[10px] font-mono text-neutral-600 bg-white/[0.04] border border-white/[0.06] px-1.5 py-0.5 rounded">
              ESC
            </kbd>
          </div>

          {/* Content */}
          <div className="max-h-[60vh] overflow-y-auto px-5 py-4 space-y-6">
            {/* Page-specific shortcuts (shown first when on a page that has them) */}
            {pageGroup && (
              <div>
                <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-[#d4af37] mb-3">
                  {pageGroup.title} — This Page
                </p>
                <div className="space-y-0">
                  {pageGroup.shortcuts.map((shortcut) => (
                    <ShortcutRow key={shortcut.label} shortcut={shortcut} />
                  ))}
                </div>
              </div>
            )}

            {shortcutGroups.map((group) => (
              <div key={group.title}>
                <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-600 mb-3">
                  {group.title}
                </p>
                <div className="space-y-0">
                  {group.shortcuts.map((shortcut) => (
                    <ShortcutRow key={shortcut.label} shortcut={shortcut} />
                  ))}
                </div>
              </div>
            ))}
          </div>

          {/* Footer */}
          <div className="px-5 py-3 border-t border-white/[0.06]">
            <p className="text-[10px] text-neutral-700 font-light">
              Press <kbd className="font-mono text-neutral-500">?</kbd> anywhere to toggle this panel
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Combined provider ────────────────────────────────────────────────────

export function KeyboardShortcutsProvider() {
  const [helpOpen, setHelpOpen] = useState(false);

  const openHelp = useCallback(() => setHelpOpen(true), []);
  const closeHelp = useCallback(() => setHelpOpen(false), []);

  useKeyboardShortcuts(openHelp);

  return (
    <KeyboardShortcutsModal open={helpOpen} onClose={closeHelp} />
  );
}
