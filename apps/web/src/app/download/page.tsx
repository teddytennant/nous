"use client";

import { useState, useCallback, useRef, useSyncExternalStore } from "react";
import Link from "next/link";
import {
  Monitor,
  Terminal,
  Smartphone,
  Download,
  ArrowLeft,
  ExternalLink,
  Copy,
  Check,
  Shield,
  Cpu,
  HardDrive,
  Upload,
  Loader2,
} from "lucide-react";

// ── Platform detection ───────────────────────────────────────────────────

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

const noop = () => () => {};
function usePlatform(): Platform {
  return useSyncExternalStore(noop, detectPlatform, () => "unknown" as Platform);
}

// ── Constants ────────────────────────────────────────────────────────────

const GITHUB_REPO = "teddytennant/nous";
const RELEASE_BASE = `https://github.com/${GITHUB_REPO}/releases/latest/download`;

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
    </svg>
  );
}

function WindowsIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801" />
    </svg>
  );
}

function LinuxIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12.504 0c-.155 0-.315.008-.48.021-4.226.333-3.105 4.807-3.17 6.298-.076 1.092-.3 1.953-1.05 3.02-.885 1.051-2.127 2.75-2.716 4.521-.278.832-.41 1.684-.287 2.489a.424.424 0 00-.11.135c-.26.268-.45.6-.663.839-.199.199-.485.267-.797.4-.313.136-.658.269-.864.68-.09.189-.136.394-.132.602 0 .199.027.4.055.536.058.399.116.728.04.97-.249.68-.28 1.145-.106 1.484.174.334.535.47.94.601.81.2 1.91.135 2.774.6.926.466 1.866.67 2.616.47.526-.116.97-.464 1.208-.946.587-.003 1.23-.269 2.26-.334.699-.058 1.574.267 2.577.2.025.134.063.198.114.333l.003.003c.391.778 1.113 1.368 1.884 1.43.585.047 1.042-.245 1.243-.645.2-.392.141-.875-.224-1.466-.536-.863-1.31-2.037-1.17-3.132.065-.467.168-.774.258-1.043.088-.264.176-.498.214-.852.063-.565-.019-1.216-.36-1.966-.36-.79-.889-1.394-1.371-1.865-.26-.254-.514-.472-.717-.607l-.026-.022c-.46-.513-.51-1.241-.56-1.963-.05-.706-.06-1.407-.36-1.989-.43-.853-1.215-1.178-1.93-1.243z" />
    </svg>
  );
}

function AndroidIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M17.523 15.341a.96.96 0 00-.707-.293h-.002a.956.956 0 00-.68.283l-.002.002a.956.956 0 00-.283.68v.002c0 .271.104.517.283.697l.002.002a.96.96 0 00.68.282h.002a.96.96 0 00.697-.282l.002-.002a.96.96 0 00.282-.697v-.002a.96.96 0 00-.274-.672zm-10.334 0a.96.96 0 00-.707-.293h-.002a.956.956 0 00-.68.283l-.002.002a.956.956 0 00-.283.68v.002c0 .271.104.517.283.697l.002.002a.96.96 0 00.68.282h.002a.96.96 0 00.697-.282l.002-.002a.96.96 0 00.282-.697v-.002a.96.96 0 00-.274-.672zM17.592 8.7l1.675-2.903a.348.348 0 00-.127-.476.348.348 0 00-.476.127l-1.697 2.94a10.374 10.374 0 00-4.32-.917h-.002a10.37 10.37 0 00-4.323.92L6.625 5.448a.348.348 0 00-.476-.127.348.348 0 00-.127.476L7.697 8.7C4.29 10.482 2.003 13.744 2.003 17.5h20.644c0-3.756-2.286-7.018-5.693-8.8h.638z" />
    </svg>
  );
}

// ── Platform configs ─────────────────────────────────────────────────────

interface PlatformConfig {
  name: string;
  icon: typeof Monitor;
  customIcon?: typeof WindowsIcon;
  description: string;
  downloads: { label: string; file: string; arch?: string; note: string }[];
  requirements: string[];
  installSteps: string[];
}

const platforms: Record<string, PlatformConfig> = {
  macos: {
    name: "macOS",
    icon: Monitor,
    description: "Native desktop app built with Tauri. Fast, lightweight, and secure.",
    downloads: [
      {
        label: "Apple Silicon (.dmg)",
        file: "nous-latest-macos-aarch64.dmg",
        arch: "aarch64",
        note: "M1, M2, M3, M4",
      },
      {
        label: "Intel (.dmg)",
        file: "nous-latest-macos-x86_64.dmg",
        arch: "x86_64",
        note: "Intel Macs",
      },
    ],
    requirements: ["macOS 11 (Big Sur) or later", "~50 MB disk space"],
    installSteps: [
      "Download the .dmg file for your Mac",
      "Open the .dmg and drag Nous to Applications",
      "Launch Nous from Applications or Spotlight",
      'If blocked by Gatekeeper: System Settings → Privacy & Security → "Open Anyway"',
    ],
  },
  linux: {
    name: "Linux",
    icon: Terminal,
    customIcon: LinuxIcon,
    description: "Available as AppImage (universal) or .deb package. Integrates with system tray.",
    downloads: [
      {
        label: "AppImage (universal)",
        file: "nous-latest-linux-x86_64.AppImage",
        arch: "x86_64",
        note: "Runs on any distro",
      },
      {
        label: "Debian/Ubuntu (.deb)",
        file: "nous-latest-linux-amd64.deb",
        arch: "amd64",
        note: "apt install compatible",
      },
    ],
    requirements: [
      "64-bit Linux (x86_64)",
      "WebKit2GTK 4.1+ (most distros ship this)",
      "~50 MB disk space",
    ],
    installSteps: [
      "Download the AppImage or .deb file",
      "AppImage: chmod +x nous-*.AppImage && ./nous-*.AppImage",
      "Deb: sudo dpkg -i nous-*.deb",
      "Or install via CLI: curl -fsSL https://nous.sh/install | sh",
    ],
  },
  windows: {
    name: "Windows",
    icon: Monitor,
    customIcon: WindowsIcon,
    description: "Native installer for Windows 10 and later. Uses system WebView2.",
    downloads: [
      {
        label: "Installer (.msi)",
        file: "nous-latest-windows-x86_64.msi",
        arch: "x86_64",
        note: "64-bit Windows",
      },
    ],
    requirements: [
      "Windows 10 (version 1803) or later",
      "WebView2 Runtime (ships with Windows 11, auto-installed on Windows 10)",
      "~50 MB disk space",
    ],
    installSteps: [
      "Download the .msi installer",
      "Double-click to run the installer",
      "Follow the installation wizard",
      "Launch Nous from the Start menu or desktop shortcut",
    ],
  },
  android: {
    name: "Android",
    icon: Smartphone,
    customIcon: AndroidIcon,
    description: "Native Android app with Material 3 design. Sideload the APK directly.",
    downloads: [
      {
        label: "Android APK",
        file: "nous-latest-android.apk",
        note: "Universal APK",
      },
    ],
    requirements: ["Android 8.0 (Oreo) or later", "~30 MB storage"],
    installSteps: [
      "Download the .apk file",
      "Open the file — if prompted, allow installs from this source",
      "Tap Install",
      "Open Nous from your app drawer",
    ],
  },
  ios: {
    name: "iOS",
    icon: Smartphone,
    description: "Coming soon via TestFlight. Native SwiftUI app.",
    downloads: [],
    requirements: ["iOS 16 or later", "iPhone or iPad"],
    installSteps: [
      "Join the TestFlight beta (link coming soon)",
      "Install via TestFlight",
    ],
  },
  cli: {
    name: "CLI",
    icon: Terminal,
    description: "Command-line interface for power users. Manage your node, identity, and data from the terminal.",
    downloads: [
      {
        label: "Linux (x86_64)",
        file: "nous-latest-x86_64-unknown-linux-gnu.tar.gz",
        note: "Static binary",
      },
      {
        label: "Linux (aarch64)",
        file: "nous-latest-aarch64-unknown-linux-gnu.tar.gz",
        note: "ARM64",
      },
      {
        label: "macOS (Apple Silicon)",
        file: "nous-latest-aarch64-apple-darwin.tar.gz",
        note: "M1/M2/M3/M4",
      },
      {
        label: "macOS (Intel)",
        file: "nous-latest-x86_64-apple-darwin.tar.gz",
        note: "Intel Macs",
      },
      {
        label: "Windows",
        file: "nous-latest-x86_64-pc-windows-msvc.zip",
        note: "64-bit",
      },
    ],
    requirements: ["Any 64-bit OS", "Terminal emulator"],
    installSteps: [
      "curl -fsSL https://nous.sh/install | sh",
      "Or download the binary, extract, and add to PATH",
      "Run: nous status",
    ],
  },
};

// Map detected platform to highlighted platform key
function getPrimaryPlatform(platform: Platform): string {
  if (platform === "macos") return "macos";
  if (platform === "linux") return "linux";
  if (platform === "windows") return "windows";
  if (platform === "android") return "android";
  if (platform === "ios") return "ios";
  return "linux";
}

// ── Components ───────────────────────────────────────────────────────────

function PlatformCard({
  id,
  config,
  isDetected,
}: {
  id: string;
  config: PlatformConfig;
  isDetected: boolean;
}) {
  const IconComponent = config.customIcon || config.icon;
  const isComingSoon = config.downloads.length === 0;

  return (
    <div
      id={`platform-${id}`}
      className={`relative border rounded-md transition-all duration-200 ${
        isDetected
          ? "border-[#d4af37]/40 bg-[#d4af37]/[0.02]"
          : "border-white/[0.06] bg-black hover:border-white/10"
      }`}
    >
      {isDetected && (
        <div className="absolute -top-3 left-6">
          <span className="text-[10px] font-mono uppercase tracking-[0.15em] bg-[#d4af37] text-black px-3 py-1 rounded-sm">
            Detected
          </span>
        </div>
      )}

      <div className="p-8">
        {/* Header */}
        <div className="flex items-start gap-4 mb-6">
          <div
            className={`w-12 h-12 rounded-md flex items-center justify-center transition-colors duration-200 ${
              isDetected
                ? "bg-[#d4af37]/10 border border-[#d4af37]/20"
                : "bg-white/[0.04] border border-white/[0.06]"
            }`}
          >
            <IconComponent
              className={`w-5 h-5 ${isDetected ? "text-[#d4af37]" : "text-neutral-400"}`}
            />
          </div>
          <div>
            <h3 className="text-lg font-medium tracking-wide mb-1">{config.name}</h3>
            <p className="text-sm text-neutral-500 font-light leading-relaxed">
              {config.description}
            </p>
          </div>
        </div>

        {/* Downloads */}
        {config.downloads.length > 0 ? (
          <div className="space-y-2 mb-6">
            {config.downloads.map((dl) => (
              <a
                key={dl.file}
                href={`${RELEASE_BASE}/${dl.file}`}
                className="group flex items-center justify-between p-3 rounded-sm border border-white/[0.04] hover:border-white/10 hover:bg-white/[0.02] transition-all duration-200"
              >
                <div className="flex items-center gap-3">
                  <Download className="w-4 h-4 text-neutral-600 group-hover:text-[#d4af37] transition-colors duration-200" />
                  <div>
                    <span className="text-sm font-light text-neutral-300 group-hover:text-white transition-colors duration-200">
                      {dl.label}
                    </span>
                    <span className="text-[10px] font-mono text-neutral-700 ml-3">
                      {dl.note}
                    </span>
                  </div>
                </div>
                <ExternalLink className="w-3 h-3 text-neutral-700 group-hover:text-neutral-400 transition-colors duration-200" />
              </a>
            ))}
          </div>
        ) : (
          <div className="flex items-center gap-2 mb-6 p-3 rounded-sm bg-white/[0.02] border border-white/[0.04]">
            <span className="text-sm text-neutral-600 font-light">Coming soon</span>
          </div>
        )}

        {/* Requirements */}
        <div className="mb-6">
          <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
            Requirements
          </p>
          <ul className="space-y-1.5">
            {config.requirements.map((req) => (
              <li key={req} className="flex items-start gap-2">
                <Cpu className="w-3 h-3 text-neutral-700 mt-0.5 shrink-0" />
                <span className="text-xs text-neutral-500 font-light">{req}</span>
              </li>
            ))}
          </ul>
        </div>

        {/* Install steps */}
        {!isComingSoon && (
          <div>
            <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-3">
              Install
            </p>
            <ol className="space-y-2">
              {config.installSteps.map((step, i) => (
                <li key={i} className="flex items-start gap-3">
                  <span className="text-[10px] font-mono text-neutral-700 mt-0.5 shrink-0 w-4 text-right">
                    {i + 1}.
                  </span>
                  <span className="text-xs text-neutral-400 font-light leading-relaxed">
                    {step}
                  </span>
                </li>
              ))}
            </ol>
          </div>
        )}
      </div>
    </div>
  );
}

// ── File Verification Widget ──────────────────────────────────────────────

function VerifyDownload() {
  const [hash, setHash] = useState<string | null>(null);
  const [fileName, setFileName] = useState<string | null>(null);
  const [hashing, setHashing] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [matchResult, setMatchResult] = useState<"match" | "mismatch" | "unknown" | null>(null);
  const [checksums, setChecksums] = useState<Record<string, string>>({});
  const inputRef = useRef<HTMLInputElement>(null);

  // Fetch checksums once
  const fetchChecksums = useCallback(async () => {
    if (Object.keys(checksums).length > 0) return checksums;
    try {
      const urls = [
        `${RELEASE_BASE}/SHA256SUMS.txt`,
        `${RELEASE_BASE}/DESKTOP-SHA256SUMS.txt`,
      ];
      const results = await Promise.allSettled(urls.map((u) => fetch(u).then((r) => r.ok ? r.text() : "")));
      const map: Record<string, string> = {};
      for (const r of results) {
        if (r.status !== "fulfilled" || !r.value) continue;
        for (const line of r.value.trim().split("\n")) {
          const parts = line.trim().split(/\s+/);
          if (parts.length >= 2) {
            const h = parts[0];
            const f = parts[parts.length - 1].replace(/^\*/, "");
            map[f] = h;
          }
        }
      }
      setChecksums(map);
      return map;
    } catch {
      return {};
    }
  }, [checksums]);

  const hashFile = useCallback(async (file: File) => {
    setHashing(true);
    setHash(null);
    setMatchResult(null);
    setFileName(file.name);

    const buffer = await file.arrayBuffer();
    const digest = await crypto.subtle.digest("SHA-256", buffer);
    const arr = Array.from(new Uint8Array(digest));
    const hex = arr.map((b) => b.toString(16).padStart(2, "0")).join("");
    setHash(hex);

    const sums = await fetchChecksums();
    const match = Object.values(sums).some((h) => h === hex);
    setMatchResult(match ? "match" : Object.keys(sums).length === 0 ? "unknown" : "mismatch");
    setHashing(false);
  }, [fetchChecksums]);

  const onDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file) hashFile(file);
  }, [hashFile]);

  const onFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) hashFile(file);
  }, [hashFile]);

  return (
    <div
      className={`relative border rounded-md transition-all duration-200 ${
        dragOver
          ? "border-[#d4af37]/40 bg-[#d4af37]/[0.02]"
          : "border-white/[0.06] hover:border-white/10"
      }`}
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={onDrop}
    >
      <input
        ref={inputRef}
        type="file"
        onChange={onFileSelect}
        className="hidden"
      />

      <div className="p-8">
        <div className="flex items-start gap-4 mb-6">
          <div className="w-12 h-12 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center">
            <Shield className="w-5 h-5 text-neutral-400" />
          </div>
          <div>
            <h3 className="text-lg font-medium tracking-wide mb-1">
              Verify Your Download
            </h3>
            <p className="text-sm text-neutral-500 font-light leading-relaxed">
              Drop a downloaded file here to compute its SHA-256 hash and verify it against the official release checksums.
            </p>
          </div>
        </div>

        {!hash && !hashing && (
          <button
            onClick={() => inputRef.current?.click()}
            className="w-full py-8 border border-dashed border-white/[0.08] rounded-sm hover:border-white/[0.15] hover:bg-white/[0.01] transition-all duration-200 flex flex-col items-center gap-3 cursor-pointer"
          >
            <Upload className="w-6 h-6 text-neutral-600" />
            <span className="text-xs text-neutral-500 font-light">
              Drop file here or click to browse
            </span>
          </button>
        )}

        {hashing && (
          <div className="flex items-center justify-center gap-3 py-8">
            <Loader2 className="w-5 h-5 text-[#d4af37] animate-spin" />
            <span className="text-sm text-neutral-400 font-light">
              Computing SHA-256...
            </span>
          </div>
        )}

        {hash && !hashing && (
          <div className="space-y-4">
            <div>
              <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-2">
                File
              </p>
              <p className="text-sm font-light text-neutral-300">
                {fileName}
              </p>
            </div>

            <div>
              <p className="text-[10px] font-mono uppercase tracking-[0.15em] text-neutral-600 mb-2">
                SHA-256
              </p>
              <code className="block text-xs font-mono text-neutral-400 break-all bg-white/[0.02] px-3 py-2 rounded-sm border border-white/[0.04]">
                {hash}
              </code>
            </div>

            <div className={`flex items-center gap-2 px-4 py-3 rounded-sm border ${
              matchResult === "match"
                ? "border-emerald-500/20 bg-emerald-500/[0.04]"
                : matchResult === "mismatch"
                  ? "border-red-500/20 bg-red-500/[0.04]"
                  : "border-white/[0.06] bg-white/[0.02]"
            }`}>
              {matchResult === "match" && (
                <>
                  <Check className="w-4 h-4 text-emerald-500 shrink-0" />
                  <span className="text-sm text-emerald-400 font-light">
                    Verified — hash matches official release
                  </span>
                </>
              )}
              {matchResult === "mismatch" && (
                <>
                  <Shield className="w-4 h-4 text-red-400 shrink-0" />
                  <span className="text-sm text-red-400 font-light">
                    Hash does not match any official release — verify you downloaded from the correct source
                  </span>
                </>
              )}
              {matchResult === "unknown" && (
                <>
                  <Shield className="w-4 h-4 text-neutral-500 shrink-0" />
                  <span className="text-sm text-neutral-400 font-light">
                    Could not fetch checksums to compare — verify manually against SHA256SUMS.txt
                  </span>
                </>
              )}
            </div>

            <button
              onClick={() => { setHash(null); setFileName(null); setMatchResult(null); }}
              className="text-[10px] font-mono uppercase tracking-wider text-neutral-600 hover:text-white transition-colors duration-150"
            >
              Verify another file
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────

export default function DownloadPage() {
  const platform = usePlatform();
  const [copied, setCopied] = useState(false);

  const primaryPlatform = getPrimaryPlatform(platform);
  const installCmd = "curl -fsSL https://nous.sh/install | sh";

  function handleCopy() {
    navigator.clipboard.writeText(installCmd);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  // Order: detected platform first, then the rest
  const platformOrder = [
    primaryPlatform,
    ...Object.keys(platforms).filter((k) => k !== primaryPlatform),
  ];

  return (
    <div className="flex flex-col min-h-screen">
      {/* Nav */}
      <nav className="fixed top-0 left-0 right-0 z-50 bg-black/80 backdrop-blur-xl border-b border-white/[0.06]">
        <div className="max-w-6xl mx-auto px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href="/"
              className="flex items-center gap-2 text-neutral-500 hover:text-white transition-colors duration-200"
            >
              <ArrowLeft className="w-4 h-4" />
              <span className="text-base font-extralight tracking-[-0.04em] text-white">
                Nous
              </span>
            </Link>
          </div>
          <div className="flex items-center gap-6">
            <a
              href={`https://github.com/${GITHUB_REPO}/releases`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-neutral-500 hover:text-white transition-colors duration-200 flex items-center gap-1.5"
            >
              <GithubIcon className="w-3.5 h-3.5" />
              <span className="hidden sm:inline">All Releases</span>
            </a>
            <Link
              href="/dashboard"
              className="text-xs font-medium bg-white text-black px-4 py-1.5 rounded-md hover:bg-neutral-200 transition-colors duration-200"
            >
              Open App
            </Link>
          </div>
        </div>
      </nav>

      {/* Hero */}
      <section className="px-6 pt-32 pb-16 max-w-6xl mx-auto w-full">
        <div className="max-w-2xl">
          <h1 className="text-4xl sm:text-5xl font-extralight tracking-[-0.04em] mb-4">
            Download <span className="text-[#d4af37]">Nous</span>
          </h1>
          <p className="text-base sm:text-lg text-neutral-400 font-light leading-relaxed mb-8">
            Available on every major platform. One app, one identity, everywhere.
          </p>

          {/* Quick CLI install */}
          <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
            <div className="flex-1 flex items-center gap-2 p-3 bg-white/[0.02] border border-white/[0.06] rounded-md">
              <Terminal className="w-4 h-4 text-neutral-600 shrink-0" />
              <code className="flex-1 text-sm font-mono text-neutral-400 overflow-x-auto whitespace-nowrap">
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
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Platform nav */}
      <section className="px-6 py-4 max-w-6xl mx-auto w-full">
        <div className="flex items-center gap-1 overflow-x-auto pb-1">
          {platformOrder.map((key) => {
            const cfg = platforms[key];
            const IconComp = cfg.customIcon || cfg.icon;
            const isActive = key === primaryPlatform;
            return (
              <a
                key={key}
                href={`#platform-${key}`}
                className={`flex items-center gap-2 px-4 py-2 rounded-md text-xs font-light whitespace-nowrap transition-all duration-200 ${
                  isActive
                    ? "bg-[#d4af37]/10 text-[#d4af37] border border-[#d4af37]/20"
                    : "text-neutral-500 hover:text-white hover:bg-white/[0.03] border border-transparent"
                }`}
              >
                <IconComp className="w-3.5 h-3.5" />
                {cfg.name}
              </a>
            );
          })}
        </div>
      </section>

      {/* Platform cards */}
      <section className="px-6 pb-24 max-w-6xl mx-auto w-full">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          {platformOrder.map((key) => (
            <PlatformCard
              key={key}
              id={key}
              config={platforms[key]}
              isDetected={key === primaryPlatform && platform !== "unknown"}
            />
          ))}
        </div>
      </section>

      {/* Verify download */}
      <section className="px-6 pb-16 max-w-6xl mx-auto w-full">
        <VerifyDownload />
      </section>

      {/* Divider */}
      <div className="w-full h-px bg-gradient-to-r from-transparent via-white/10 to-transparent" />

      {/* Verification section */}
      <section className="px-6 py-20 max-w-6xl mx-auto w-full">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
          <div className="flex items-start gap-4">
            <div className="w-10 h-10 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center shrink-0">
              <Shield className="w-4 h-4 text-neutral-500" />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-1">Verified Builds</h4>
              <p className="text-xs text-neutral-500 font-light leading-relaxed">
                Every release includes SHA-256 checksums. Verify your download
                against{" "}
                <a
                  href={`${RELEASE_BASE}/SHA256SUMS.txt`}
                  className="text-neutral-400 hover:text-white underline underline-offset-2 transition-colors duration-200"
                >
                  SHA256SUMS.txt
                </a>
              </p>
            </div>
          </div>

          <div className="flex items-start gap-4">
            <div className="w-10 h-10 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center shrink-0">
              <GithubIcon className="w-4 h-4 text-neutral-500" />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-1">Open Source</h4>
              <p className="text-xs text-neutral-500 font-light leading-relaxed">
                Every line of code is public. Build from source anytime:{" "}
                <code className="text-[10px] font-mono text-neutral-600">
                  cargo build --release
                </code>
              </p>
            </div>
          </div>

          <div className="flex items-start gap-4">
            <div className="w-10 h-10 rounded-md bg-white/[0.04] border border-white/[0.06] flex items-center justify-center shrink-0">
              <HardDrive className="w-4 h-4 text-neutral-500" />
            </div>
            <div>
              <h4 className="text-sm font-medium mb-1">Local-First</h4>
              <p className="text-xs text-neutral-500 font-light leading-relaxed">
                All data stored locally in encrypted SQLite. No cloud accounts, no
                telemetry, no tracking.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="mt-auto px-6 py-8 border-t border-white/[0.04]">
        <div className="max-w-6xl mx-auto flex flex-col sm:flex-row justify-between items-center gap-4">
          <div className="flex items-center gap-4">
            <Link href="/" className="text-[10px] text-neutral-700 font-mono tracking-wider hover:text-neutral-500 transition-colors duration-200">
              nous v0.1.0
            </Link>
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
