"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { ChevronDown } from "lucide-react";

/* ── Types ─────────────────────────────────────────────────────────────── */

interface FaqItem {
  question: string;
  answer: string;
}

interface FaqCategory {
  label: string;
  items: FaqItem[];
}

/* ── Data ──────────────────────────────────────────────────────────────── */

const faqCategories: FaqCategory[] = [
  {
    label: "General",
    items: [
      {
        question: "What is Nous?",
        answer:
          "Nous is a sovereign everything-app that combines identity, messaging, governance, payments, social, storage, AI, and a decentralized browser into a single encrypted, local-first platform. It replaces apps like WhatsApp, Signal, Venmo, Twitter, iCloud, and more — with one unified system you actually own.",
      },
      {
        question: "Is Nous free?",
        answer:
          "Yes. Nous is open-source under the MIT license. There are no subscriptions, no premium tiers, and no ads. The entire codebase is available on GitHub. You can run it, fork it, or contribute to it.",
      },
      {
        question: "How is Nous different from other decentralized apps?",
        answer:
          "Most decentralized apps solve one problem — messaging, or identity, or payments. Nous unifies all of these into a single composable protocol built on 20 Rust crates. Each subsystem works independently or together, so you get the convenience of a super-app with the sovereignty of a self-hosted stack.",
      },
    ],
  },
  {
    label: "Privacy & Security",
    items: [
      {
        question: "Who can read my messages?",
        answer:
          "Nobody but you and your intended recipients. All messages use end-to-end encryption with X25519 key exchange and AES-256-GCM. Sealed-box anonymous encryption is available for scenarios where even the sender's identity should be protected. No server ever processes plaintext.",
      },
      {
        question: "Where is my data stored?",
        answer:
          "Everything is stored locally on your device in an encrypted SQLite database. Nous is local-first — it works offline and syncs via CRDTs when you connect. There are no cloud servers holding your data, and your encryption keys never leave your machine.",
      },
      {
        question: "Can my account be banned or suspended?",
        answer:
          "No. Your identity is a DID:key generated locally on your device — no platform grants it, and no platform can revoke it. You have full sovereignty over your identity and data. Even if Nous the project ceased to exist, your identity and data would remain yours.",
      },
    ],
  },
  {
    label: "Technical",
    items: [
      {
        question: "What platforms does Nous run on?",
        answer:
          "Nous runs on macOS, Windows, Linux, Android, and the web. Desktop apps are built with Tauri for a native experience with minimal resource usage. The Android app is native Kotlin with Jetpack Compose. iOS is coming soon via TestFlight.",
      },
      {
        question: "What is the tech stack?",
        answer:
          "The backend is a 20-crate Rust workspace providing REST, GraphQL, and gRPC APIs. The web frontend is Next.js with TypeScript. Desktop uses Tauri. Cryptography is built on audited Rust crates: ed25519-dalek, x25519-dalek, aes-gcm, and hkdf. Networking uses libp2p, and data sync uses CRDTs.",
      },
      {
        question: "Can I self-host Nous?",
        answer:
          "Yes. Nous is designed to be self-hostable. You can run the API server on your own hardware, and all data stays on your machine by default. The P2P networking layer means you don't need a central server at all — nodes discover and communicate directly.",
      },
    ],
  },
];

/* ── Accordion Item ───────────────────────────────────────────────────── */

function AccordionItem({
  item,
  isOpen,
  onToggle,
}: {
  item: FaqItem;
  isOpen: boolean;
  onToggle: () => void;
}) {
  const contentRef = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    if (contentRef.current) {
      setHeight(contentRef.current.scrollHeight);
    }
  }, [isOpen]);

  return (
    <div className="border-b border-white/[0.04] last:border-b-0">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={isOpen}
        className="flex items-center justify-between w-full py-5 px-1 text-left group focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[#d4af37]/50 rounded-sm"
      >
        <span
          className={`text-sm font-light pr-8 transition-colors duration-200 ${
            isOpen ? "text-white" : "text-neutral-400 group-hover:text-neutral-200"
          }`}
        >
          {item.question}
        </span>
        <ChevronDown
          className={`w-4 h-4 shrink-0 transition-all duration-200 ${
            isOpen
              ? "text-[#d4af37] rotate-180"
              : "text-neutral-700 group-hover:text-neutral-500"
          }`}
        />
      </button>
      <div
        style={{ maxHeight: isOpen ? `${height}px` : "0px" }}
        className="overflow-hidden transition-[max-height] duration-300 ease-out"
      >
        <div ref={contentRef} className="pb-5 px-1 pr-12">
          <p className="text-sm text-neutral-500 font-light leading-relaxed">
            {item.answer}
          </p>
        </div>
      </div>
    </div>
  );
}

/* ── FAQ Section ──────────────────────────────────────────────────────── */

export function FaqSection() {
  const [openIndex, setOpenIndex] = useState<string | null>(null);
  const [activeCategory, setActiveCategory] = useState(0);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleToggle = useCallback((key: string) => {
    setOpenIndex((prev) => (prev === key ? null : key));
  }, []);

  const selectCategory = useCallback((i: number) => {
    setActiveCategory(i);
    setOpenIndex(null);
    tabRefs.current[i]?.focus();
  }, []);

  const handleTabKeyDown = useCallback(
    (e: React.KeyboardEvent, index: number) => {
      const count = faqCategories.length;
      let next: number | null = null;

      if (e.key === "ArrowDown" || e.key === "ArrowRight") {
        e.preventDefault();
        next = (index + 1) % count;
      } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
        e.preventDefault();
        next = (index - 1 + count) % count;
      } else if (e.key === "Home") {
        e.preventDefault();
        next = 0;
      } else if (e.key === "End") {
        e.preventDefault();
        next = count - 1;
      }

      if (next !== null) {
        selectCategory(next);
      }
    },
    [selectCategory],
  );

  const currentCategory = faqCategories[activeCategory];

  return (
    <section id="faq" className="px-6 py-28 max-w-6xl mx-auto w-full scroll-mt-16">
      <div className="mb-20">
        <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
          FAQ
        </h2>
        <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
          Questions? <span className="text-white">Answered.</span>
        </p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-[200px_1fr] gap-12">
        {/* Category tabs */}
        <div className="flex lg:flex-col gap-1" role="tablist" aria-label="FAQ categories">
          {faqCategories.map((cat, i) => (
            <button
              key={cat.label}
              ref={(el) => { tabRefs.current[i] = el; }}
              type="button"
              role="tab"
              tabIndex={activeCategory === i ? 0 : -1}
              aria-selected={activeCategory === i}
              aria-controls={`faq-panel-${i}`}
              onClick={() => selectCategory(i)}
              onKeyDown={(e) => handleTabKeyDown(e, i)}
              className={`text-left text-xs font-mono uppercase tracking-[0.15em] px-3 py-2 rounded-sm transition-colors duration-200 ${
                activeCategory === i
                  ? "text-[#d4af37] bg-[#d4af37]/[0.06]"
                  : "text-neutral-600 hover:text-neutral-400 hover:bg-white/[0.02]"
              }`}
            >
              {cat.label}
            </button>
          ))}
        </div>

        {/* Questions */}
        <div
          id={`faq-panel-${activeCategory}`}
          role="tabpanel"
          aria-label={`${currentCategory.label} questions`}
        >
          {currentCategory.items.map((item, i) => {
            const key = `${activeCategory}-${i}`;
            return (
              <AccordionItem
                key={key}
                item={item}
                isOpen={openIndex === key}
                onToggle={() => handleToggle(key)}
              />
            );
          })}
        </div>
      </div>
    </section>
  );
}
