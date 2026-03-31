"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { Copy, Check } from "lucide-react";

// ── Terminal demo data ─────────────────────────────────────────────────

interface TerminalStep {
  command: string;
  output: string[];
  /** Delay before starting to type this command (ms) */
  delay?: number;
}

const steps: TerminalStep[] = [
  {
    command: "nous status",
    output: [
      "\x1b[90m┌─────────────────────────────────────────┐\x1b[0m",
      "\x1b[90m│\x1b[0m  \x1b[1mNous\x1b[0m v0.1.0         \x1b[32m● running\x1b[0m        \x1b[90m│\x1b[0m",
      "\x1b[90m├─────────────────────────────────────────┤\x1b[0m",
      "\x1b[90m│\x1b[0m  Identity    \x1b[33mdid:key:z6Mk...xR4q\x1b[0m      \x1b[90m│\x1b[0m",
      "\x1b[90m│\x1b[0m  Peers       \x1b[36m12 connected\x1b[0m              \x1b[90m│\x1b[0m",
      "\x1b[90m│\x1b[0m  Storage     \x1b[37m2.4 GB local\x1b[0m              \x1b[90m│\x1b[0m",
      "\x1b[90m│\x1b[0m  Uptime      \x1b[37m4h 23m\x1b[0m                    \x1b[90m│\x1b[0m",
      "\x1b[90m└─────────────────────────────────────────┘\x1b[0m",
    ],
  },
  {
    command: "nous identity create --name \"Teddy\"",
    delay: 800,
    output: [
      "\x1b[32m✓\x1b[0m Identity created",
      "",
      "  DID      \x1b[33mdid:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK\x1b[0m",
      "  Name     Teddy",
      "  Signing  \x1b[90mEd25519\x1b[0m  \x1b[37m6MkhaXg...a2doK\x1b[0m",
      "  Exchange \x1b[90mX25519\x1b[0m   \x1b[37m2DrZ7vN...pQ8Kx\x1b[0m",
      "",
      "  \x1b[90mKeypair stored in ~/.nous/keystore (encrypted)\x1b[0m",
    ],
  },
  {
    command: "nous message send --to did:key:z6Mkr...9Fj2 \"Hello, sovereign future\"",
    delay: 600,
    output: [
      "\x1b[32m✓\x1b[0m Message encrypted and sent",
      "",
      "  To       \x1b[33mdid:key:z6Mkr...9Fj2\x1b[0m",
      "  Cipher   \x1b[90mAES-256-GCM + X25519\x1b[0m",
      "  Size     \x1b[37m142 bytes\x1b[0m",
      "  Route    \x1b[36mpeer → peer (direct)\x1b[0m",
      "",
      "  \x1b[90mNo server saw this message.\x1b[0m",
    ],
  },
  {
    command: "nous social post \"First post on Nous 🔐\"",
    delay: 600,
    output: [
      "\x1b[32m✓\x1b[0m Post published to network",
      "",
      "  ID       \x1b[33mnote1qzy...8m4v\x1b[0m",
      "  Author   \x1b[37mTeddy\x1b[0m (did:key:z6Mkh...doK)",
      "  Relays   \x1b[36m3 confirmed\x1b[0m",
      "",
      "  \x1b[90mYour post. Your keys. Your network.\x1b[0m",
    ],
  },
];

// ── ANSI parser (minimal) ──────────────────────────────────────────────

interface Span {
  text: string;
  className: string;
}

const ANSI_COLOR_MAP: Record<string, string> = {
  "0": "",
  "1": "font-bold",
  "32": "text-emerald-400",
  "33": "text-[#d4af37]",
  "36": "text-cyan-400",
  "37": "text-neutral-300",
  "90": "text-neutral-600",
};

function parseAnsi(line: string): Span[] {
  const spans: Span[] = [];
  const regex = /\x1b\[([0-9;]+)m/g;
  let lastIndex = 0;
  let currentClass = "";

  let match: RegExpExecArray | null;
  while ((match = regex.exec(line)) !== null) {
    // Text before this escape
    if (match.index > lastIndex) {
      spans.push({ text: line.slice(lastIndex, match.index), className: currentClass });
    }
    // Parse codes
    const codes = match[1].split(";");
    const classes: string[] = [];
    for (const code of codes) {
      const mapped = ANSI_COLOR_MAP[code];
      if (mapped === "") {
        // reset
        currentClass = "";
      } else if (mapped) {
        classes.push(mapped);
      }
    }
    if (classes.length > 0) {
      currentClass = classes.join(" ");
    }
    lastIndex = match.index + match[0].length;
  }

  // Remaining text
  if (lastIndex < line.length) {
    spans.push({ text: line.slice(lastIndex), className: currentClass });
  }

  return spans;
}

function AnsiLine({ line }: { line: string }) {
  const spans = parseAnsi(line);
  if (spans.length === 0) return <br />;
  return (
    <div className="leading-[1.6]">
      {spans.map((span, i) =>
        span.className ? (
          <span key={i} className={span.className}>
            {span.text}
          </span>
        ) : (
          <span key={i}>{span.text}</span>
        )
      )}
    </div>
  );
}

// ── Prompt line with copy-on-hover ────────────────────────────────────

function PromptLine({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  function handleCopy(e: React.MouseEvent) {
    e.stopPropagation();
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  return (
    <div className="group/prompt flex items-start gap-0 relative">
      <span className="text-[#d4af37] select-none shrink-0">❯ </span>
      <span className="text-white">{text}</span>
      <button
        type="button"
        onClick={handleCopy}
        className="absolute right-0 top-0 p-1 rounded opacity-0 group-hover/prompt:opacity-100 hover:bg-white/[0.06] transition-all duration-150"
        aria-label="Copy command"
      >
        {copied ? (
          <Check className="w-3 h-3 text-emerald-400" />
        ) : (
          <Copy className="w-3 h-3 text-neutral-600 hover:text-neutral-400" />
        )}
      </button>
    </div>
  );
}

// ── Terminal Demo Component ────────────────────────────────────────────

export function TerminalDemo() {
  const [lines, setLines] = useState<Array<{ type: "prompt" | "output"; text: string }>>([]);
  const [typingText, setTypingText] = useState("");
  const [isTyping, setIsTyping] = useState(false);
  const [currentStep, setCurrentStep] = useState(0);
  const [showCursor, setShowCursor] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);
  const hasStarted = useRef(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Cursor blink
  useEffect(() => {
    const interval = setInterval(() => setShowCursor((v) => !v), 530);
    return () => clearInterval(interval);
  }, []);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [lines, typingText]);

  const typeCommand = useCallback(
    (command: string): Promise<void> => {
      return new Promise((resolve) => {
        setIsTyping(true);
        let i = 0;
        const interval = setInterval(() => {
          i++;
          setTypingText(command.slice(0, i));
          if (i >= command.length) {
            clearInterval(interval);
            setIsTyping(false);
            resolve();
          }
        }, 35 + Math.random() * 25); // Realistic variable typing speed
      });
    },
    []
  );

  const showOutput = useCallback((outputLines: string[]): Promise<void> => {
    return new Promise((resolve) => {
      let i = 0;
      const interval = setInterval(() => {
        if (i < outputLines.length) {
          setLines((prev) => [...prev, { type: "output", text: outputLines[i] }]);
          i++;
        } else {
          clearInterval(interval);
          resolve();
        }
      }, 40);
    });
  }, []);

  const runDemo = useCallback(async () => {
    for (let s = 0; s < steps.length; s++) {
      const step = steps[s];
      setCurrentStep(s);

      // Delay between commands
      if (s > 0) {
        await new Promise((r) => setTimeout(r, step.delay ?? 600));
      }

      // Type the command
      await typeCommand(step.command);

      // Brief pause after typing, then "execute"
      await new Promise((r) => setTimeout(r, 300));

      // Commit the typed command to lines
      setLines((prev) => [...prev, { type: "prompt", text: step.command }]);
      setTypingText("");

      // Brief "processing" pause
      await new Promise((r) => setTimeout(r, 200));

      // Show output line by line
      await showOutput(step.output);

      // Add blank line between commands
      if (s < steps.length - 1) {
        setLines((prev) => [...prev, { type: "output", text: "" }]);
      }
    }

    // After all commands, pause then restart
    await new Promise((r) => setTimeout(r, 4000));
    setLines([]);
    setTypingText("");
    setCurrentStep(0);
    runDemo();
  }, [typeCommand, showOutput]);

  // Start on mount via IntersectionObserver (only when visible)
  useEffect(() => {
    if (hasStarted.current) return;
    const el = containerRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting && !hasStarted.current) {
          hasStarted.current = true;
          runDemo();
          observer.disconnect();
        }
      },
      { threshold: 0.3 }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [runDemo]);

  return (
    <div ref={containerRef} className="w-full max-w-2xl mx-auto">
      {/* Terminal window */}
      <div className="rounded-lg overflow-hidden border border-white/[0.08] bg-[#0a0a0a] shadow-2xl shadow-black/50">
        {/* Title bar */}
        <div className="flex items-center gap-2 px-4 py-3 bg-white/[0.02] border-b border-white/[0.06]">
          <div className="flex items-center gap-1.5">
            <div className="w-3 h-3 rounded-full bg-[#ff5f57]" />
            <div className="w-3 h-3 rounded-full bg-[#febc2e]" />
            <div className="w-3 h-3 rounded-full bg-[#28c840]" />
          </div>
          <div className="flex-1 text-center">
            <span className="text-[11px] font-mono text-neutral-600">
              nous — ~
            </span>
          </div>
          <div className="w-[54px]" /> {/* Balance the dots */}
        </div>

        {/* Terminal body */}
        <div
          ref={scrollRef}
          className="p-4 sm:p-5 font-mono text-[13px] sm:text-sm leading-relaxed h-[340px] sm:h-[380px] overflow-y-auto terminal-scroll"
        >
          {/* Rendered lines */}
          {lines.map((line, i) =>
            line.type === "prompt" ? (
              <PromptLine key={i} text={line.text} />
            ) : (
              <AnsiLine key={i} line={line.text} />
            )
          )}

          {/* Currently typing line */}
          {(typingText || (!isTyping && lines.length === 0)) && (
            <div className="flex items-start gap-0">
              <span className="text-[#d4af37] select-none shrink-0">❯ </span>
              <span className="text-white">{typingText}</span>
              <span
                className={`inline-block w-[8px] h-[18px] ml-px translate-y-[1px] ${
                  showCursor ? "bg-[#d4af37]" : "bg-transparent"
                }`}
              />
            </div>
          )}

          {/* Cursor on empty prompt when idle and has content */}
          {!typingText && !isTyping && lines.length > 0 && (
            <div className="flex items-start gap-0 mt-0">
              <span className="text-[#d4af37] select-none shrink-0">❯ </span>
              <span
                className={`inline-block w-[8px] h-[18px] ml-px translate-y-[1px] ${
                  showCursor ? "bg-[#d4af37]" : "bg-transparent"
                }`}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
