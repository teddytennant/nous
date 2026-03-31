"use client";

import { useState, useCallback, useEffect } from "react";
import {
  Users,
  MessageSquare,
  Wallet,
  Brain,
  Globe,
  Vote,
  FolderOpen,
  ArrowRight,
  Check,
  Loader2,
  Fingerprint,
} from "lucide-react";
import { identity } from "@/lib/api";
import { cn } from "@/lib/utils";

/* ── Types ─────────────────────────────────────────────────────────────── */

type Step = "welcome" | "identity" | "tour" | "ready";

interface OnboardingProps {
  onComplete: () => void;
}

/* ── Illustrations ─────────────────────────────────────────────────────── */

function NousLogo() {
  return (
    <svg
      width="64"
      height="64"
      viewBox="0 0 64 64"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className="onboarding-logo"
    >
      {/* Outer ring */}
      <circle
        cx="32"
        cy="32"
        r="28"
        stroke="#d4af37"
        strokeWidth="0.5"
        opacity="0.3"
      />
      {/* Inner ring */}
      <circle
        cx="32"
        cy="32"
        r="18"
        stroke="white"
        strokeWidth="0.5"
        opacity="0.15"
      />
      {/* Center node */}
      <circle cx="32" cy="32" r="4" fill="#d4af37" opacity="0.2" />
      <circle cx="32" cy="32" r="2" fill="#d4af37" opacity="0.6" />
      {/* Orbital nodes */}
      <circle cx="32" cy="4" r="1.5" fill="#d4af37" opacity="0.4" />
      <circle cx="56" cy="20" r="1.5" fill="white" opacity="0.2" />
      <circle cx="56" cy="44" r="1.5" fill="white" opacity="0.15" />
      <circle cx="32" cy="60" r="1.5" fill="#d4af37" opacity="0.3" />
      <circle cx="8" cy="44" r="1.5" fill="white" opacity="0.15" />
      <circle cx="8" cy="20" r="1.5" fill="white" opacity="0.2" />
      {/* Connections */}
      <line x1="32" y1="6" x2="32" y2="14" stroke="#d4af37" strokeWidth="0.5" opacity="0.2" />
      <line x1="54" y1="21" x2="46" y2="26" stroke="white" strokeWidth="0.5" opacity="0.1" />
      <line x1="54" y1="43" x2="46" y2="38" stroke="white" strokeWidth="0.5" opacity="0.1" />
      <line x1="32" y1="58" x2="32" y2="50" stroke="#d4af37" strokeWidth="0.5" opacity="0.15" />
      <line x1="10" y1="43" x2="18" y2="38" stroke="white" strokeWidth="0.5" opacity="0.1" />
      <line x1="10" y1="21" x2="18" y2="26" stroke="white" strokeWidth="0.5" opacity="0.1" />
    </svg>
  );
}

function KeyIllustration() {
  return (
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Key body */}
      <circle cx="18" cy="18" r="10" stroke="#d4af37" strokeWidth="1" opacity="0.4" />
      <circle cx="18" cy="18" r="4" stroke="#d4af37" strokeWidth="1" opacity="0.6" />
      {/* Key shaft */}
      <line x1="26" y1="26" x2="42" y2="42" stroke="white" strokeWidth="1" opacity="0.3" />
      {/* Key teeth */}
      <line x1="36" y1="36" x2="40" y2="32" stroke="white" strokeWidth="1" opacity="0.2" />
      <line x1="40" y1="40" x2="44" y2="36" stroke="white" strokeWidth="1" opacity="0.2" />
    </svg>
  );
}

/* ── Feature cards for tour step ──────────────────────────────────────── */

const features = [
  {
    icon: Users,
    name: "Social",
    description: "Decentralized feed on the Nostr protocol",
  },
  {
    icon: MessageSquare,
    name: "Messages",
    description: "End-to-end encrypted, peer-to-peer",
  },
  {
    icon: Wallet,
    name: "Wallet",
    description: "Multi-currency with Lightning support",
  },
  {
    icon: Brain,
    name: "AI",
    description: "Local-first inference, private by default",
  },
  {
    icon: Vote,
    name: "Governance",
    description: "Propose and vote on protocol changes",
  },
  {
    icon: FolderOpen,
    name: "Files",
    description: "Encrypted, distributed storage",
  },
  {
    icon: Globe,
    name: "Network",
    description: "P2P mesh with real-time peer discovery",
  },
  {
    icon: Fingerprint,
    name: "Identity",
    description: "Self-sovereign DIDs and verifiable credentials",
  },
];

/* ── Step components ──────────────────────────────────────────────────── */

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="onboarding-step flex flex-col items-center text-center px-6">
      <div className="mb-10">
        <NousLogo />
      </div>
      <h1 className="text-4xl sm:text-5xl font-extralight tracking-[-0.04em] mb-4 hero-title">
        Welcome to Nous
      </h1>
      <p className="text-sm text-neutral-500 font-light max-w-sm leading-relaxed mb-2">
        Your sovereign digital infrastructure.
      </p>
      <p className="text-xs text-neutral-600 font-light max-w-xs leading-relaxed mb-12">
        Identity, communication, finance, and intelligence — all local-first,
        encrypted, and under your control.
      </p>
      <button
        onClick={onNext}
        className="group flex items-center gap-3 px-8 py-3 bg-[#d4af37] text-black text-sm font-medium rounded-sm hover:bg-[#c4a030] active:bg-[#b39028] transition-colors duration-200"
      >
        Get Started
        <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform duration-200" />
      </button>
      <p className="text-[10px] text-neutral-700 font-mono mt-8 tracking-wider">
        TAKES LESS THAN 30 SECONDS
      </p>
    </div>
  );
}

function IdentityStep({
  onNext,
}: {
  onNext: (did: string) => void;
}) {
  const [displayName, setDisplayName] = useState("");
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleCreate = useCallback(async () => {
    setCreating(true);
    setError(null);
    try {
      const id = await identity.create(displayName || undefined);
      localStorage.setItem("nous_did", id.did);
      onNext(id.did);
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Failed to create identity"
      );
      setCreating(false);
    }
  }, [displayName, onNext]);

  return (
    <div className="onboarding-step flex flex-col items-center text-center px-6">
      <div className="mb-8 text-neutral-600">
        <KeyIllustration />
      </div>
      <h2 className="text-2xl sm:text-3xl font-extralight tracking-[-0.03em] mb-3">
        Create Your Identity
      </h2>
      <p className="text-xs text-neutral-500 font-light max-w-sm leading-relaxed mb-10">
        Generate a cryptographic key pair that becomes your self-sovereign
        identity. No email, no password, no third parties.
      </p>

      <div className="w-full max-w-xs space-y-4">
        <div>
          <input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && !creating && handleCreate()}
            placeholder="Display name (optional)"
            className="w-full bg-white/[0.03] border border-white/[0.08] text-sm font-light px-4 py-3 rounded-sm outline-none placeholder:text-neutral-700 focus:border-[#d4af37]/30 transition-colors duration-200"
            autoFocus
          />
          <p className="text-[10px] text-neutral-700 mt-2 text-left">
            You can change this anytime in Settings
          </p>
        </div>

        {error && (
          <p className="text-xs text-red-500/80 font-light">{error}</p>
        )}

        <button
          onClick={handleCreate}
          disabled={creating}
          className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-[#d4af37] text-black text-sm font-medium rounded-sm hover:bg-[#c4a030] active:bg-[#b39028] transition-colors duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {creating ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Generating keys...
            </>
          ) : (
            <>
              Generate Identity
              <Fingerprint className="w-4 h-4" />
            </>
          )}
        </button>
      </div>

      <div className="mt-10 flex items-center gap-6 text-[10px] font-mono text-neutral-700 tracking-wider">
        <span>ED25519</span>
        <span className="w-px h-3 bg-white/[0.06]" />
        <span>X25519</span>
        <span className="w-px h-3 bg-white/[0.06]" />
        <span>LOCAL ONLY</span>
      </div>
    </div>
  );
}

function TourStep({ onNext }: { onNext: () => void }) {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  return (
    <div className="onboarding-step flex flex-col items-center px-6 w-full max-w-lg">
      <h2 className="text-2xl sm:text-3xl font-extralight tracking-[-0.03em] mb-3 text-center">
        Everything You Need
      </h2>
      <p className="text-xs text-neutral-500 font-light text-center max-w-sm leading-relaxed mb-10">
        Eight modules, one protocol. All running locally on your node.
      </p>

      <div className="grid grid-cols-2 gap-px bg-white/[0.03] w-full mb-10">
        {features.map((feature, i) => {
          const Icon = feature.icon;
          return (
            <div
              key={feature.name}
              onMouseEnter={() => setHoveredIndex(i)}
              onMouseLeave={() => setHoveredIndex(null)}
              className={cn(
                "bg-black p-5 transition-all duration-200 cursor-default",
                hoveredIndex === i && "bg-white/[0.02]",
              )}
              style={{
                animationDelay: `${i * 60}ms`,
              }}
            >
              <Icon
                className={cn(
                  "w-4 h-4 mb-3 transition-colors duration-200",
                  hoveredIndex === i ? "text-[#d4af37]" : "text-neutral-600",
                )}
              />
              <p className="text-sm font-light mb-1">{feature.name}</p>
              <p className="text-[10px] text-neutral-600 font-light leading-relaxed">
                {feature.description}
              </p>
            </div>
          );
        })}
      </div>

      <button
        onClick={onNext}
        className="group flex items-center gap-3 px-8 py-3 bg-[#d4af37] text-black text-sm font-medium rounded-sm hover:bg-[#c4a030] active:bg-[#b39028] transition-colors duration-200"
      >
        Enter Nous
        <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform duration-200" />
      </button>
    </div>
  );
}

function ReadyStep({ did, onEnter }: { did: string; onEnter: () => void }) {
  // Auto-redirect after a brief pause
  useEffect(() => {
    const timer = setTimeout(onEnter, 2500);
    return () => clearTimeout(timer);
  }, [onEnter]);

  return (
    <div className="onboarding-step flex flex-col items-center text-center px-6">
      <div className="w-12 h-12 rounded-full border border-[#d4af37]/30 flex items-center justify-center mb-8">
        <Check className="w-5 h-5 text-[#d4af37]" />
      </div>
      <h2 className="text-2xl sm:text-3xl font-extralight tracking-[-0.03em] mb-3">
        You&apos;re All Set
      </h2>
      <p className="text-xs text-neutral-500 font-light max-w-sm leading-relaxed mb-6">
        Your identity is live. Your node is yours. Welcome to the sovereign web.
      </p>
      <p className="text-[10px] font-mono text-neutral-700 break-all max-w-xs leading-relaxed">
        {did}
      </p>
      <div className="mt-8">
        <Loader2 className="w-4 h-4 text-neutral-600 animate-spin" />
      </div>
    </div>
  );
}

/* ── Progress indicator ───────────────────────────────────────────────── */

const steps: Step[] = ["welcome", "identity", "tour", "ready"];

function StepIndicator({ current }: { current: Step }) {
  const currentIndex = steps.indexOf(current);

  return (
    <div className="flex items-center gap-2">
      {steps.map((step, i) => (
        <div
          key={step}
          className={cn(
            "h-px transition-all duration-300",
            i <= currentIndex
              ? "bg-[#d4af37] w-6"
              : "bg-white/[0.08] w-4",
          )}
        />
      ))}
    </div>
  );
}

/* ── Main onboarding component ────────────────────────────────────────── */

export function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState<Step>("welcome");
  const [did, setDid] = useState("");

  const handleIdentityCreated = useCallback((newDid: string) => {
    setDid(newDid);
    setStep("tour");
  }, []);

  const handleEnter = useCallback(() => {
    onComplete();
  }, [onComplete]);

  return (
    <div className="fixed inset-0 z-[200] bg-black flex flex-col items-center justify-center">
      {/* Background texture */}
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_center,rgba(212,175,55,0.03)_0%,transparent_70%)]" />

      {/* Content */}
      <div className="relative z-10 flex flex-col items-center justify-center flex-1 w-full">
        {step === "welcome" && (
          <WelcomeStep onNext={() => setStep("identity")} />
        )}
        {step === "identity" && (
          <IdentityStep onNext={handleIdentityCreated} />
        )}
        {step === "tour" && (
          <TourStep onNext={() => setStep("ready")} />
        )}
        {step === "ready" && (
          <ReadyStep did={did} onEnter={handleEnter} />
        )}
      </div>

      {/* Step indicator */}
      <div className="relative z-10 pb-8">
        <StepIndicator current={step} />
      </div>
    </div>
  );
}
