"use client";

import { useEffect, useState, useCallback, useSyncExternalStore } from "react";
import Link from "next/link";
import {
  Shield,
  Lock,
  Vote,
  Wallet,
  Users,
  HardDrive,
  Brain,
  Globe,
  Download,
  Terminal,
  Smartphone,
  Monitor,
  ArrowRight,
  ExternalLink,
  Copy,
  Check,
  Menu,
  X,
} from "lucide-react";

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}
import { Badge } from "@/components/ui/badge";
import { ArchitectureDiagram } from "@/components/architecture-diagram";
import { TerminalDemo } from "@/components/terminal-demo";

const features = [
  {
    icon: Shield,
    name: "Identity",
    description:
      "Self-sovereign DID:key identifiers with verifiable credentials and zero-knowledge proofs. You own your identity — no platform can revoke it.",
    tag: "did:key + zk-SNARKs",
  },
  {
    icon: Lock,
    name: "Messaging",
    description:
      "End-to-end encrypted with X25519 key exchange and AES-256-GCM. Sealed-box anonymous encryption. No server ever reads your messages.",
    tag: "E2EE + sealed box",
  },
  {
    icon: Vote,
    name: "Governance",
    description:
      "Quadratic voting, delegation, on-chain proposals. Sybil-resistant DAOs where every voice is weighted fairly.",
    tag: "quadratic voting",
  },
  {
    icon: Wallet,
    name: "Payments",
    description:
      "Multi-chain wallet with send, receive, and swap. Escrow-backed transactions for trustless peer-to-peer commerce.",
    tag: "multi-chain",
  },
  {
    icon: Users,
    name: "Social",
    description:
      "Decentralized feeds with posts, follows, and reactions. Your social graph belongs to you — portable across any client.",
    tag: "Nostr + ActivityPub",
  },
  {
    icon: HardDrive,
    name: "Storage",
    description:
      "Local-first with CRDTs for offline editing. IPFS for content distribution. Encrypted vaults for sensitive data.",
    tag: "CRDTs + IPFS",
  },
  {
    icon: Brain,
    name: "AI",
    description:
      "Local inference with an agent framework. Semantic search across all your data. Intelligence without surveillance.",
    tag: "local inference",
  },
  {
    icon: Globe,
    name: "Browser",
    description:
      "Built-in decentralized browser with IPFS gateway, ENS resolution, and per-site identity switching.",
    tag: "IPFS + ENS",
  },
];

const primitives = [
  ["Signing", "Ed25519"],
  ["Exchange", "X25519"],
  ["Encryption", "AES-256-GCM"],
  ["Derivation", "HKDF-SHA256"],
  ["Identity", "DID:key"],
  ["Networking", "libp2p"],
  ["Storage", "SQLite + CRDTs"],
  ["Consensus", "Raft"],
];

type Platform = "macos" | "windows" | "linux" | "android" | "ios" | "unknown";

function detectPlatform(): Platform {
  if (typeof navigator === "undefined") return "unknown";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("android")) return "android";
  if (ua.includes("iphone") || ua.includes("ipad")) return "ios";
  if (ua.includes("mac")) return "macos";
  if (ua.includes("win")) return "windows";
  if (ua.includes("linux")) return "linux";
  return "unknown";
}

const platformDownloads: Record<
  Platform,
  { label: string; file: string; icon: typeof Monitor; note: string }
> = {
  macos: {
    label: "Download for macOS",
    file: "nous-latest-macos-aarch64.dmg",
    icon: Monitor,
    note: "Universal binary — Apple Silicon + Intel",
  },
  windows: {
    label: "Download for Windows",
    file: "nous-latest-windows-x86_64.msi",
    icon: Monitor,
    note: "Windows 10+ (64-bit)",
  },
  linux: {
    label: "Download for Linux",
    file: "nous-latest-linux-x86_64.AppImage",
    icon: Terminal,
    note: "AppImage — runs on any distro",
  },
  android: {
    label: "Download for Android",
    file: "nous-latest-android.apk",
    icon: Smartphone,
    note: "Android 8.0+ (API 26)",
  },
  ios: {
    label: "Coming Soon — iOS",
    file: "",
    icon: Smartphone,
    note: "TestFlight beta available soon",
  },
  unknown: {
    label: "Download Nous",
    file: "nous-latest-linux-x86_64.AppImage",
    icon: Download,
    note: "See all platforms below",
  },
};

const allPlatforms = [
  { name: "macOS (Apple Silicon)", file: "nous-latest-macos-aarch64.dmg" },
  { name: "macOS (Intel)", file: "nous-latest-macos-x86_64.dmg" },
  { name: "Linux (AppImage)", file: "nous-latest-linux-x86_64.AppImage" },
  { name: "Linux (.deb)", file: "nous-latest-linux-amd64.deb" },
  { name: "Windows (.msi)", file: "nous-latest-windows-x86_64.msi" },
  { name: "Android (.apk)", file: "nous-latest-android.apk" },
];

const GITHUB_REPO = "teddytennant/nous";
const RELEASE_BASE = `https://github.com/${GITHUB_REPO}/releases/latest/download`;

const noop = () => () => {};
const getServerPlatform = (): Platform => "unknown";
function getClientPlatform(): Platform {
  return detectPlatform();
}

function usePlatform(): Platform {
  return useSyncExternalStore(noop, getClientPlatform, getServerPlatform);
}

export default function Home() {
  const platform = usePlatform();
  const [copied, setCopied] = useState(false);
  const [scrolled, setScrolled] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  useEffect(() => {
    function onScroll() {
      setScrolled(window.scrollY > 20);
    }
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  // Close mobile menu on Escape
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape" && mobileMenuOpen) {
        setMobileMenuOpen(false);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [mobileMenuOpen]);

  // Lock body scroll when mobile menu is open
  useEffect(() => {
    if (mobileMenuOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileMenuOpen]);

  const closeMobileMenu = useCallback(() => setMobileMenuOpen(false), []);

  const primaryDownload = platformDownloads[platform];
  const installCmd = "curl -fsSL https://nous.sh/install | sh";

  function handleCopy() {
    navigator.clipboard.writeText(installCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="flex flex-col min-h-screen">
      {/* Floating nav */}
      <nav
        className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
          scrolled || mobileMenuOpen
            ? "bg-black/80 backdrop-blur-xl border-b border-white/[0.06]"
            : "bg-transparent"
        }`}
      >
        <div className="max-w-6xl mx-auto px-6 h-14 flex items-center justify-between">
          <span className="text-base font-extralight tracking-[-0.04em]">
            Nous
          </span>
          <div className="flex items-center gap-6">
            <a
              href="#features"
              className="text-xs text-neutral-500 hover:text-white transition-colors duration-200 hidden sm:block"
            >
              Features
            </a>
            <Link
              href="/download"
              className="text-xs text-neutral-500 hover:text-white transition-colors duration-200 hidden sm:block"
            >
              Download
            </Link>
            <a
              href={`https://github.com/${GITHUB_REPO}`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-neutral-500 hover:text-white transition-colors duration-200 hidden sm:flex items-center gap-1.5"
            >
              <GithubIcon className="w-3.5 h-3.5" />
              GitHub
            </a>
            <Link
              href="/dashboard"
              className="text-xs font-medium bg-white text-black px-4 py-1.5 rounded-md hover:bg-neutral-200 transition-colors duration-200 hidden sm:block"
            >
              Open App
            </Link>
            {/* Mobile hamburger */}
            <button
              type="button"
              onClick={() => setMobileMenuOpen((v) => !v)}
              className="sm:hidden p-2 -mr-2 rounded-sm hover:bg-white/[0.04] transition-colors duration-150"
              aria-label={mobileMenuOpen ? "Close menu" : "Open menu"}
              aria-expanded={mobileMenuOpen}
            >
              {mobileMenuOpen ? (
                <X className="w-5 h-5 text-neutral-400" />
              ) : (
                <Menu className="w-5 h-5 text-neutral-400" />
              )}
            </button>
          </div>
        </div>

        {/* Mobile menu overlay */}
        <div
          className={`sm:hidden overflow-hidden transition-all duration-200 ease-out ${
            mobileMenuOpen ? "max-h-80 opacity-100" : "max-h-0 opacity-0"
          }`}
        >
          <div className="bg-black/95 backdrop-blur-xl border-t border-white/[0.06] px-6 py-6 space-y-1">
            <a
              href="#features"
              onClick={closeMobileMenu}
              className="flex items-center gap-3 px-3 py-3 text-sm font-light text-neutral-400 hover:text-white hover:bg-white/[0.02] rounded-sm transition-colors duration-150"
            >
              <Shield className="w-4 h-4 text-neutral-600" />
              Features
            </a>
            <Link
              href="/download"
              onClick={closeMobileMenu}
              className="flex items-center gap-3 px-3 py-3 text-sm font-light text-neutral-400 hover:text-white hover:bg-white/[0.02] rounded-sm transition-colors duration-150"
            >
              <Download className="w-4 h-4 text-neutral-600" />
              Download
            </Link>
            <a
              href={`https://github.com/${GITHUB_REPO}`}
              target="_blank"
              rel="noopener noreferrer"
              onClick={closeMobileMenu}
              className="flex items-center gap-3 px-3 py-3 text-sm font-light text-neutral-400 hover:text-white hover:bg-white/[0.02] rounded-sm transition-colors duration-150"
            >
              <GithubIcon className="w-4 h-4 text-neutral-600" />
              GitHub
            </a>
            <div className="pt-3 border-t border-white/[0.06]">
              <Link
                href="/dashboard"
                onClick={closeMobileMenu}
                className="flex items-center justify-center gap-2 bg-[#d4af37] text-black px-6 py-2.5 rounded-md text-sm font-medium hover:bg-[#c4a030] transition-colors duration-200"
              >
                Open App
                <ArrowRight className="w-4 h-4" />
              </Link>
            </div>
          </div>
        </div>
      </nav>

      {/* Mobile menu backdrop */}
      {mobileMenuOpen && (
        <div
          className="sm:hidden fixed inset-0 z-40 bg-black/40"
          onClick={closeMobileMenu}
          aria-hidden="true"
        />
      )}

      {/* Hero */}
      <section className="relative flex flex-col items-center justify-center px-6 pt-40 pb-32 overflow-hidden">
        {/* Animated gradient orb */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] opacity-[0.07] pointer-events-none">
          <div className="w-full h-full rounded-full bg-[radial-gradient(circle,#d4af37_0%,transparent_70%)] animate-[pulse_6s_ease-in-out_infinite]" />
        </div>

        <div className="relative max-w-3xl text-center">
          <div className="inline-flex items-center gap-2 mb-8">
            <Badge
              variant="outline"
              className="text-[10px] font-mono tracking-wider uppercase px-3 py-1 border-white/10"
            >
              v0.1.0
            </Badge>
            <Badge
              variant="outline"
              className="text-[10px] font-mono tracking-wider uppercase px-3 py-1 border-[#d4af37]/30 text-[#d4af37]"
            >
              Private Alpha
            </Badge>
          </div>

          <h1 className="text-6xl sm:text-7xl md:text-8xl lg:text-9xl font-extralight tracking-[-0.05em] mb-6 hero-title">
            Nous
          </h1>

          <p className="text-lg sm:text-xl md:text-2xl text-neutral-400 font-extralight leading-relaxed max-w-2xl mx-auto mb-4">
            The sovereign everything-app.
          </p>
          <p className="text-sm sm:text-base text-neutral-600 font-light leading-relaxed max-w-lg mx-auto mb-12">
            Identity, messaging, governance, payments, AI — unified under one
            encrypted, decentralized protocol. Own your digital life.
          </p>

          <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
            <Link
              href="/dashboard"
              className="group flex items-center gap-2 bg-[#d4af37] text-black px-8 py-3 rounded-md text-sm font-medium hover:bg-[#c4a030] transition-all duration-200 glow-pulse"
            >
              Get Started
              <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform duration-200" />
            </Link>
            <Link
              href="/download"
              className="flex items-center gap-2 border border-white/10 px-8 py-3 rounded-md text-sm font-light text-neutral-300 hover:border-white/20 hover:text-white transition-all duration-200"
            >
              <Download className="w-4 h-4" />
              Download
            </Link>
          </div>
        </div>
      </section>

      {/* Divider line */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Features */}
      <section id="features" className="px-6 py-28 max-w-6xl mx-auto w-full scroll-mt-16">
        <div className="mb-20">
          <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
            Architecture
          </h2>
          <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
            Eight subsystems. One protocol.{" "}
            <span className="text-white">Zero compromise.</span>
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-px bg-white/[0.04] rounded-sm overflow-hidden stagger-in">
          {features.map((feature) => {
            const Icon = feature.icon;
            return (
              <div
                key={feature.name}
                className="bg-black p-8 sm:p-10 group hover:bg-white/[0.02] transition-colors duration-200"
              >
                <div className="flex items-start gap-4">
                  <div className="shrink-0 w-10 h-10 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center group-hover:border-[#d4af37]/20 group-hover:bg-[#d4af37]/[0.04] transition-colors duration-300">
                    <Icon className="w-4.5 h-4.5 text-neutral-500 group-hover:text-[#d4af37] transition-colors duration-300" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-3 mb-3">
                      <h3 className="text-base font-medium tracking-wide">
                        {feature.name}
                      </h3>
                      <span className="text-[10px] font-mono text-neutral-700 tracking-wider uppercase">
                        {feature.tag}
                      </span>
                    </div>
                    <p className="text-sm text-neutral-500 font-light leading-relaxed">
                      {feature.description}
                    </p>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* How It Works — Terminal Demo */}
      <section className="px-6 py-28 max-w-6xl mx-auto w-full">
        <div className="mb-16 text-center">
          <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
            How It Works
          </h2>
          <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl mx-auto">
            One CLI. <span className="text-white">Total sovereignty.</span>
          </p>
        </div>

        <TerminalDemo />

        <p className="text-center text-xs text-neutral-700 font-light mt-8 max-w-md mx-auto">
          Every command runs locally. Your keys never leave your machine.
          No accounts, no servers, no permission needed.
        </p>
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Topology */}
      <section className="px-6 py-28 max-w-6xl mx-auto w-full">
        <div className="mb-16 text-center">
          <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
            Topology
          </h2>
          <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl mx-auto">
            Everything connects through{" "}
            <span className="text-white">one core.</span>
          </p>
        </div>

        <ArchitectureDiagram />

        <p className="text-center text-xs text-neutral-700 font-light mt-8 max-w-md mx-auto">
          Each subsystem is independent but interconnected — composable
          primitives that work together or standalone.
        </p>
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Download */}
      <section id="download" className="px-6 py-28 max-w-6xl mx-auto w-full scroll-mt-16">
        <div className="mb-20">
          <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
            Install
          </h2>
          <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
            One click. <span className="text-white">Every platform.</span>
          </p>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12">
          {/* Primary download */}
          <div className="space-y-6">
            <a
              href={
                primaryDownload.file
                  ? `${RELEASE_BASE}/${primaryDownload.file}`
                  : undefined
              }
              className={`group flex items-center gap-4 p-6 border border-white/[0.08] rounded-md transition-all duration-200 ${
                primaryDownload.file
                  ? "hover:border-[#d4af37]/30 hover:bg-[#d4af37]/[0.02] cursor-pointer"
                  : "opacity-50 cursor-not-allowed"
              }`}
            >
              <div className="w-12 h-12 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center group-hover:border-[#d4af37]/20 transition-colors duration-200">
                <primaryDownload.icon className="w-5 h-5 text-neutral-400 group-hover:text-[#d4af37] transition-colors duration-200" />
              </div>
              <div>
                <p className="text-sm font-medium mb-0.5">
                  {primaryDownload.label}
                </p>
                <p className="text-xs text-neutral-600 font-light">
                  {primaryDownload.note}
                </p>
              </div>
              {primaryDownload.file && (
                <ArrowRight className="w-4 h-4 text-neutral-700 ml-auto group-hover:text-[#d4af37] group-hover:translate-x-0.5 transition-all duration-200" />
              )}
            </a>

            {/* CLI install */}
            <div className="p-4 bg-white/[0.02] border border-white/[0.06] rounded-md">
              <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
                Or install via CLI
              </p>
              <div className="flex items-center gap-2">
                <code className="flex-1 text-xs font-mono text-neutral-400 overflow-x-auto">
                  {installCmd}
                </code>
                <button
                  onClick={handleCopy}
                  className="shrink-0 p-1.5 rounded hover:bg-white/[0.06] transition-colors duration-200"
                  aria-label="Copy install command"
                >
                  {copied ? (
                    <Check className="w-3.5 h-3.5 text-emerald-500" />
                  ) : (
                    <Copy className="w-3.5 h-3.5 text-neutral-600" />
                  )}
                </button>
              </div>
            </div>
          </div>

          {/* All platforms */}
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-4">
              All Platforms
            </p>
            <div className="space-y-1">
              {allPlatforms.map((p) => (
                <a
                  key={p.file}
                  href={`${RELEASE_BASE}/${p.file}`}
                  className="flex items-center justify-between py-3 px-4 rounded-sm hover:bg-white/[0.03] transition-colors duration-200 group"
                >
                  <span className="text-sm font-light text-neutral-400 group-hover:text-white transition-colors duration-200">
                    {p.name}
                  </span>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] font-mono text-neutral-700">
                      {p.file.split(".").pop()}
                    </span>
                    <ExternalLink className="w-3 h-3 text-neutral-700 group-hover:text-neutral-400 transition-colors duration-200" />
                  </div>
                </a>
              ))}
            </div>
            <Link
              href="/download"
              className="flex items-center gap-2 mt-4 px-4 py-2 text-xs text-neutral-500 hover:text-[#d4af37] transition-colors duration-200 link-underline"
            >
              View all platforms, install guides & verification
              <ArrowRight className="w-3 h-3" />
            </Link>
          </div>
        </div>
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Primitives */}
      <section className="px-6 py-28 max-w-6xl mx-auto w-full">
        <div className="mb-20">
          <h2 className="text-xs font-mono uppercase tracking-[0.25em] text-neutral-600 mb-4">
            Primitives
          </h2>
          <p className="text-2xl sm:text-3xl font-extralight tracking-[-0.02em] text-neutral-300 max-w-xl">
            Built on <span className="text-white">audited cryptography.</span>
          </p>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-y-10 gap-x-12">
          {primitives.map(([label, value]) => (
            <div key={label} className="group">
              <p className="text-[10px] font-mono uppercase tracking-[0.2em] text-neutral-700 mb-2 group-hover:text-neutral-500 transition-colors duration-200">
                {label}
              </p>
              <p className="text-sm font-light tracking-wide group-hover:text-[#d4af37] transition-colors duration-200">
                {value}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* CTA */}
      <section className="px-6 py-32 text-center">
        <p className="text-3xl sm:text-4xl md:text-5xl font-extralight tracking-[-0.03em] mb-4">
          Sovereign. Encrypted.{" "}
          <span className="text-[#d4af37]">Unstoppable.</span>
        </p>
        <p className="text-sm text-neutral-600 font-light mb-12 max-w-md mx-auto">
          Join the private alpha and help build the future of decentralized
          software.
        </p>
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4">
          <Link
            href="/dashboard"
            className="group flex items-center gap-2 bg-white text-black px-8 py-3 rounded-md text-sm font-medium hover:bg-neutral-200 transition-colors duration-200"
          >
            Open App
            <ArrowRight className="w-4 h-4 group-hover:translate-x-0.5 transition-transform duration-200" />
          </Link>
          <a
            href={`https://github.com/${GITHUB_REPO}`}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 border border-white/10 px-8 py-3 rounded-md text-sm font-light text-neutral-400 hover:border-white/20 hover:text-white transition-all duration-200"
          >
            <GithubIcon className="w-4 h-4" />
            View on GitHub
          </a>
        </div>
      </section>

      {/* Footer */}
      <footer className="mt-auto px-6 py-8 border-t border-white/[0.04]">
        <div className="max-w-6xl mx-auto flex flex-col sm:flex-row justify-between items-center gap-4">
          <div className="flex items-center gap-4">
            <p className="text-[10px] text-neutral-700 font-mono tracking-wider">
              nous v0.1.0
            </p>
            <span className="text-neutral-800">|</span>
            <a
              href={`https://github.com/${GITHUB_REPO}`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-[10px] text-neutral-700 font-mono tracking-wider hover:text-neutral-500 transition-colors duration-200"
            >
              github
            </a>
          </div>
          <p className="text-[10px] text-neutral-700 font-light tracking-wider">
            Built for sovereignty. Not for sale.
          </p>
        </div>
      </footer>
    </div>
  );
}
