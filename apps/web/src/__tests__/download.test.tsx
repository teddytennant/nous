import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import DownloadPage from "@/app/download/page";

// ── Helpers ──────────────────────────────────────────────────────────────

function setUserAgent(ua: string) {
  Object.defineProperty(navigator, "userAgent", {
    value: ua,
    writable: true,
    configurable: true,
  });
}

const LINUX_UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const MAC_UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const WINDOWS_UA =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const ANDROID_UA =
  "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

// ── Tests ────────────────────────────────────────────────────────────────

describe("Download page", () => {
  beforeEach(() => {
    setUserAgent(LINUX_UA);
  });

  describe("Page structure", () => {
    it("renders the page heading with Download and Nous", () => {
      render(<DownloadPage />);
      const heading = screen.getByRole("heading", { level: 1 });
      expect(heading).toHaveTextContent("Download");
      expect(heading).toHaveTextContent("Nous");
    });

    it("renders the subtitle", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText(/Available on every major platform/),
      ).toBeInTheDocument();
    });

    it("renders navigation with back link to home", () => {
      render(<DownloadPage />);
      // The nav link to home has href="/"
      const homeLinks = screen.getAllByRole("link").filter(
        (el) => el.getAttribute("href") === "/",
      );
      expect(homeLinks.length).toBeGreaterThan(0);
    });

    it("renders Open App button linking to dashboard", () => {
      render(<DownloadPage />);
      const openApp = screen.getByText("Open App").closest("a");
      expect(openApp).toHaveAttribute("href", "/dashboard");
    });

    it("renders All Releases link to GitHub", () => {
      render(<DownloadPage />);
      const link = screen.getByText("All Releases").closest("a");
      expect(link).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases",
      );
      expect(link).toHaveAttribute("target", "_blank");
    });
  });

  describe("CLI install command", () => {
    it("renders the curl install command", () => {
      render(<DownloadPage />);
      const codeElements = screen.getAllByText(
        "curl -fsSL https://nous.sh/install | sh",
      );
      expect(codeElements.length).toBeGreaterThanOrEqual(1);
    });

    it("copies command to clipboard on button click", async () => {
      const writeText = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, {
        clipboard: { writeText },
      });

      render(<DownloadPage />);
      const copyBtn = screen.getByRole("button", {
        name: "Copy install command",
      });

      await act(async () => {
        fireEvent.click(copyBtn);
      });

      expect(writeText).toHaveBeenCalledWith(
        "curl -fsSL https://nous.sh/install | sh",
      );
    });
  });

  describe("Platform cards", () => {
    it("renders all 6 platform sections", () => {
      render(<DownloadPage />);
      // Each platform has an id like platform-macos, platform-linux, etc.
      expect(document.getElementById("platform-macos")).toBeInTheDocument();
      expect(document.getElementById("platform-linux")).toBeInTheDocument();
      expect(document.getElementById("platform-windows")).toBeInTheDocument();
      expect(document.getElementById("platform-android")).toBeInTheDocument();
      expect(document.getElementById("platform-ios")).toBeInTheDocument();
      expect(document.getElementById("platform-cli")).toBeInTheDocument();
    });

    it("renders macOS download options", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Apple Silicon (.dmg)")).toBeInTheDocument();
      expect(screen.getByText("Intel (.dmg)")).toBeInTheDocument();
    });

    it("renders Linux download options", () => {
      render(<DownloadPage />);
      expect(screen.getByText("AppImage (universal)")).toBeInTheDocument();
      expect(screen.getByText("Debian/Ubuntu (.deb)")).toBeInTheDocument();
    });

    it("renders Windows download option", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Installer (.msi)")).toBeInTheDocument();
    });

    it("renders Android download option", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Android APK")).toBeInTheDocument();
    });

    it("renders iOS as coming soon", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Coming soon")).toBeInTheDocument();
    });

    it("renders CLI download options for all platforms", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Linux (x86_64)")).toBeInTheDocument();
      expect(screen.getByText("Linux (aarch64)")).toBeInTheDocument();
      expect(screen.getByText("macOS (Apple Silicon)")).toBeInTheDocument();
      expect(screen.getByText("macOS (Intel)")).toBeInTheDocument();
    });

    it("download links point to correct GitHub release URLs", () => {
      render(<DownloadPage />);
      const appleLink = screen.getByText("Apple Silicon (.dmg)").closest("a");
      expect(appleLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases/latest/download/nous-latest-macos-aarch64.dmg",
      );
    });

    it("Android APK link points to correct URL", () => {
      render(<DownloadPage />);
      const apkLink = screen.getByText("Android APK").closest("a");
      expect(apkLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases/latest/download/nous-latest-android.apk",
      );
    });

    it("Windows MSI link points to correct URL", () => {
      render(<DownloadPage />);
      const msiLink = screen.getByText("Installer (.msi)").closest("a");
      expect(msiLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases/latest/download/nous-latest-windows-x86_64.msi",
      );
    });
  });

  describe("Platform detection", () => {
    it("highlights Linux card when on Linux", () => {
      setUserAgent(LINUX_UA);
      render(<DownloadPage />);
      const detected = screen.getAllByText("Detected");
      expect(detected.length).toBe(1);
      const card = detected[0].closest("[id^='platform-']");
      expect(card?.id).toBe("platform-linux");
    });

    it("highlights macOS card when on Mac", () => {
      setUserAgent(MAC_UA);
      render(<DownloadPage />);
      const detected = screen.getAllByText("Detected");
      expect(detected.length).toBe(1);
      const card = detected[0].closest("[id^='platform-']");
      expect(card?.id).toBe("platform-macos");
    });

    it("highlights Windows card when on Windows", () => {
      setUserAgent(WINDOWS_UA);
      render(<DownloadPage />);
      const detected = screen.getAllByText("Detected");
      expect(detected.length).toBe(1);
      const card = detected[0].closest("[id^='platform-']");
      expect(card?.id).toBe("platform-windows");
    });

    it("highlights Android card when on Android", () => {
      setUserAgent(ANDROID_UA);
      render(<DownloadPage />);
      const detected = screen.getAllByText("Detected");
      expect(detected.length).toBe(1);
      const card = detected[0].closest("[id^='platform-']");
      expect(card?.id).toBe("platform-android");
    });

    it("puts detected platform first in card order", () => {
      setUserAgent(MAC_UA);
      render(<DownloadPage />);
      const allCards = document.querySelectorAll("[id^='platform-']");
      expect(allCards[0]?.id).toBe("platform-macos");
    });
  });

  describe("Platform navigation tabs", () => {
    it("renders nav tabs linking to platform anchors", () => {
      render(<DownloadPage />);
      const navLinks = screen
        .getAllByRole("link")
        .filter((el) => el.getAttribute("href")?.startsWith("#platform-"));
      expect(navLinks.length).toBe(6);
    });

    it("highlights detected platform tab with gold accent", () => {
      setUserAgent(LINUX_UA);
      render(<DownloadPage />);
      const navLinks = screen
        .getAllByRole("link")
        .filter((el) => el.getAttribute("href")?.startsWith("#platform-"));
      const linuxNav = navLinks.find(
        (el) => el.getAttribute("href") === "#platform-linux",
      );
      expect(linuxNav?.className).toContain("text-[#d4af37]");
    });

    it("non-detected platform tabs have neutral styling", () => {
      setUserAgent(LINUX_UA);
      render(<DownloadPage />);
      const navLinks = screen
        .getAllByRole("link")
        .filter((el) => el.getAttribute("href")?.startsWith("#platform-"));
      const macNav = navLinks.find(
        (el) => el.getAttribute("href") === "#platform-macos",
      );
      expect(macNav?.className).toContain("text-neutral-500");
    });
  });

  describe("Requirements and install steps", () => {
    it("renders requirements for macOS", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText("macOS 11 (Big Sur) or later"),
      ).toBeInTheDocument();
    });

    it("renders requirements for Linux", () => {
      render(<DownloadPage />);
      expect(screen.getByText("64-bit Linux (x86_64)")).toBeInTheDocument();
    });

    it("renders requirements for Windows", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText("Windows 10 (version 1803) or later"),
      ).toBeInTheDocument();
    });

    it("renders requirements for Android", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText("Android 8.0 (Oreo) or later"),
      ).toBeInTheDocument();
    });

    it("renders install steps", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText(/Open the .dmg and drag Nous to Applications/),
      ).toBeInTheDocument();
    });
  });

  describe("Verify download widget", () => {
    it("renders the verify download section", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Verify Your Download")).toBeInTheDocument();
    });

    it("renders the file drop zone", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText("Drop file here or click to browse"),
      ).toBeInTheDocument();
    });

    it("has a hidden file input", () => {
      render(<DownloadPage />);
      const input = document.querySelector('input[type="file"]');
      expect(input).toBeInTheDocument();
      expect(input?.className).toContain("hidden");
    });
  });

  describe("Trust indicators", () => {
    it("renders verified builds section", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Verified Builds")).toBeInTheDocument();
    });

    it("renders open source section", () => {
      render(<DownloadPage />);
      // "Open Source" as heading text in the trust section
      const heading = screen.getByText("Open Source");
      expect(heading.tagName).toBe("H4");
    });

    it("renders local-first section", () => {
      render(<DownloadPage />);
      expect(screen.getByText("Local-First")).toBeInTheDocument();
    });

    it("links to SHA256SUMS.txt", () => {
      render(<DownloadPage />);
      const link = screen.getByText("SHA256SUMS.txt").closest("a");
      expect(link).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases/latest/download/SHA256SUMS.txt",
      );
    });
  });

  describe("Footer", () => {
    it("renders version in footer", () => {
      render(<DownloadPage />);
      expect(screen.getByText("nous v0.1.0")).toBeInTheDocument();
    });

    it("renders GitHub link in footer", () => {
      render(<DownloadPage />);
      const link = screen.getByText("github").closest("a");
      expect(link).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous",
      );
    });

    it("renders tagline in footer", () => {
      render(<DownloadPage />);
      expect(
        screen.getByText("Built for sovereignty. Not for sale."),
      ).toBeInTheDocument();
    });
  });
});
