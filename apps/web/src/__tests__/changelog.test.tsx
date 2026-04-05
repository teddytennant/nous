import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import ChangelogPage from "@/app/changelog/page";

// ── Tests ────────────────────────────────────────────────────────────────

describe("Changelog page", () => {
  describe("Page structure", () => {
    it("renders the page heading", () => {
      render(<ChangelogPage />);
      const heading = screen.getByRole("heading", { level: 1 });
      expect(heading).toHaveTextContent("Changelog");
    });

    it("renders the page subtitle", () => {
      render(<ChangelogPage />);
      expect(
        screen.getByText(/Every feature, fix, and improvement shipped to Nous/),
      ).toBeInTheDocument();
    });

    it("renders navigation with back link to home", () => {
      render(<ChangelogPage />);
      const homeLinks = screen
        .getAllByRole("link")
        .filter((el) => el.getAttribute("href") === "/");
      expect(homeLinks.length).toBeGreaterThan(0);
    });

    it("renders Open App button linking to dashboard", () => {
      render(<ChangelogPage />);
      const openApp = screen.getByText("Open App").closest("a");
      expect(openApp).toHaveAttribute("href", "/dashboard");
    });

    it("renders Download link in nav", () => {
      render(<ChangelogPage />);
      // There are multiple download links; the nav one is in the sticky header
      const navDownload = screen
        .getAllByRole("link")
        .filter(
          (el) =>
            el.getAttribute("href") === "/download" &&
            el.textContent === "Download",
        );
      expect(navDownload.length).toBeGreaterThan(0);
    });

    it("shows the latest version badge", () => {
      render(<ChangelogPage />);
      // v0.4.0 appears in the hero badge, quick nav, and timeline
      const badges = screen.getAllByText("v0.4.0");
      expect(badges.length).toBeGreaterThanOrEqual(1);
    });

    it("shows the total changes count", () => {
      render(<ChangelogPage />);
      // The count is in the header as "NN CHANGES SHIPPED"
      expect(screen.getByText(/CHANGES SHIPPED/)).toBeInTheDocument();
    });
  });

  describe("RSS and GitHub links", () => {
    it("renders RSS feed link", () => {
      render(<ChangelogPage />);
      const rssLink = screen.getByText("RSS Feed").closest("a");
      expect(rssLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases.atom",
      );
      expect(rssLink).toHaveAttribute("target", "_blank");
    });

    it("renders GitHub Releases link", () => {
      render(<ChangelogPage />);
      const ghLink = screen.getByText("GitHub Releases").closest("a");
      expect(ghLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases",
      );
      expect(ghLink).toHaveAttribute("target", "_blank");
    });
  });

  describe("Filter tabs", () => {
    it("renders all filter tabs", () => {
      render(<ChangelogPage />);
      const tabs = screen.getAllByRole("tab");
      expect(tabs.length).toBe(6); // All, Features, Fixes, Improvements, Security, Platform
    });

    it("defaults to All filter selected", () => {
      render(<ChangelogPage />);
      const allTab = screen.getByRole("tab", { name: /All/ });
      expect(allTab).toHaveAttribute("aria-selected", "true");
    });

    it("filters to show only features when Features tab clicked", () => {
      render(<ChangelogPage />);
      const featuresTab = screen.getByRole("tab", { name: /Features/ });
      fireEvent.click(featuresTab);
      expect(featuresTab).toHaveAttribute("aria-selected", "true");
      // After filtering, all visible category badges should be "Feature"
      const badges = screen.getAllByText("Feature");
      expect(badges.length).toBeGreaterThan(0);
    });

    it("filters to show only fixes when Fixes tab clicked", () => {
      render(<ChangelogPage />);
      const fixesTab = screen.getByRole("tab", { name: /Fixes/ });
      fireEvent.click(fixesTab);
      // No fix entries in our test data, so should show empty state
      expect(
        screen.getByText("No changes match this filter."),
      ).toBeInTheDocument();
    });

    it("shows clear filter button on empty filter results", () => {
      render(<ChangelogPage />);
      const fixesTab = screen.getByRole("tab", { name: /Fixes/ });
      fireEvent.click(fixesTab);
      const clearBtn = screen.getByText("Clear filter");
      expect(clearBtn).toBeInTheDocument();
    });

    it("clears filter when clear button clicked", () => {
      render(<ChangelogPage />);
      const fixesTab = screen.getByRole("tab", { name: /Fixes/ });
      fireEvent.click(fixesTab);
      const clearBtn = screen.getByText("Clear filter");
      fireEvent.click(clearBtn);
      // Should be back on All and show releases
      const allTab = screen.getByRole("tab", { name: /All/ });
      expect(allTab).toHaveAttribute("aria-selected", "true");
    });

    it("shows count next to each filter tab", () => {
      render(<ChangelogPage />);
      const tabs = screen.getAllByRole("tab");
      // Each tab should contain a number
      for (const tab of tabs) {
        const countEl = tab.querySelector(".font-mono");
        expect(countEl).toBeTruthy();
        expect(Number(countEl!.textContent)).toBeGreaterThanOrEqual(0);
      }
    });
  });

  describe("Version quick nav", () => {
    it("renders jump-to links for all versions", () => {
      render(<ChangelogPage />);
      expect(screen.getByText("Jump to:")).toBeInTheDocument();
      // Version strings appear in multiple places; just verify they exist
      expect(screen.getAllByText("v0.4.0").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("v0.3.0").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("v0.2.0").length).toBeGreaterThanOrEqual(1);
      expect(screen.getAllByText("v0.1.0").length).toBeGreaterThanOrEqual(1);
    });

    it("links to anchor IDs for each version", () => {
      render(<ChangelogPage />);
      const v040Link = screen
        .getAllByRole("link")
        .find((el) => el.getAttribute("href") === "#v0.4.0");
      expect(v040Link).toBeTruthy();
    });
  });

  describe("Release entries", () => {
    it("renders all release versions as headings", () => {
      render(<ChangelogPage />);
      const headings = screen.getAllByRole("heading", { level: 2 });
      const versions = headings.map((h) => h.textContent);
      expect(versions).toContain("0.4.0");
      expect(versions).toContain("0.3.0");
      expect(versions).toContain("0.2.0");
      expect(versions).toContain("0.1.0");
    });

    it("renders release titles as h3 headings", () => {
      render(<ChangelogPage />);
      const h3s = screen.getAllByRole("heading", { level: 3 });
      const titles = h3s.map((h) => h.textContent);
      expect(titles).toContain("UI/UX Overhaul");
      expect(titles).toContain("Full-Stack Polish");
      expect(titles).toContain("Communication & Commerce");
      expect(titles).toContain("Genesis");
    });

    it("renders release summaries", () => {
      render(<ChangelogPage />);
      expect(
        screen.getByText(/Massive frontend polish pass/),
      ).toBeInTheDocument();
      expect(
        screen.getByText(/Initial release. 20-crate Rust workspace/),
      ).toBeInTheDocument();
    });

    it("shows Latest badge on most recent release", () => {
      render(<ChangelogPage />);
      expect(screen.getByText("Latest")).toBeInTheDocument();
    });

    it("renders date for each release", () => {
      render(<ChangelogPage />);
      // formatDate produces "April 5, 2026" etc.
      expect(screen.getByText(/April 5, 2026/)).toBeInTheDocument();
    });

    it("shows change count for each release", () => {
      render(<ChangelogPage />);
      const changeCounts = screen.getAllByText(/\d+ changes/);
      expect(changeCounts.length).toBe(4); // one per release
    });

    it("renders individual changes with descriptions", () => {
      render(<ChangelogPage />);
      expect(
        screen.getByText(
          "FAQ accordion with category tabs and keyboard navigation",
        ),
      ).toBeInTheDocument();
      expect(
        screen.getByText("20-crate Rust workspace architecture"),
      ).toBeInTheDocument();
    });

    it("shows category badges on changes", () => {
      render(<ChangelogPage />);
      // Multiple Feature badges across releases
      const featureBadges = screen.getAllByText("Feature");
      expect(featureBadges.length).toBeGreaterThan(0);
    });

    it("shows scope badges on scoped changes", () => {
      render(<ChangelogPage />);
      const webBadges = screen.getAllByText("web");
      expect(webBadges.length).toBeGreaterThan(0);
      const apiBadges = screen.getAllByText("api");
      expect(apiBadges.length).toBeGreaterThan(0);
    });
  });

  describe("Release links", () => {
    it("renders release notes link for each version", () => {
      render(<ChangelogPage />);
      const releaseLinks = screen.getAllByText("Release notes");
      expect(releaseLinks.length).toBe(4);
      const firstLink = releaseLinks[0].closest("a");
      expect(firstLink).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous/releases/tag/v0.4.0",
      );
    });

    it("renders full diff link for each version", () => {
      render(<ChangelogPage />);
      const diffLinks = screen.getAllByText("Full diff");
      expect(diffLinks.length).toBe(4);
    });

    it("renders download link for each version", () => {
      render(<ChangelogPage />);
      const downloadLinks = screen
        .getAllByRole("link")
        .filter(
          (el) =>
            el.getAttribute("href") === "/download" &&
            el.textContent?.includes("Download"),
        );
      // At least one download link per release plus footer/nav
      expect(downloadLinks.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe("Bottom CTA section", () => {
    it("renders the bottom CTA text", () => {
      render(<ChangelogPage />);
      expect(
        screen.getByText("Building in the open since March 2026"),
      ).toBeInTheDocument();
    });

    it("renders Star on GitHub button", () => {
      render(<ChangelogPage />);
      const starBtn = screen.getByText("Star on GitHub").closest("a");
      expect(starBtn).toHaveAttribute(
        "href",
        "https://github.com/teddytennant/nous",
      );
      expect(starBtn).toHaveAttribute("target", "_blank");
    });

    it("renders Download Nous button", () => {
      render(<ChangelogPage />);
      const dlBtn = screen.getByText("Download Nous").closest("a");
      expect(dlBtn).toHaveAttribute("href", "/download");
    });
  });

  describe("Footer", () => {
    it("renders footer with version", () => {
      render(<ChangelogPage />);
      expect(screen.getByText(/NOUS v0\.4\.0/)).toBeInTheDocument();
    });

    it("renders footer links", () => {
      render(<ChangelogPage />);
      // Footer has Home, Download, GitHub links
      const footerGithub = screen
        .getAllByRole("link")
        .filter(
          (el) =>
            el.getAttribute("href") === "https://github.com/teddytennant/nous" &&
            el.textContent === "GitHub",
        );
      expect(footerGithub.length).toBeGreaterThan(0);
    });

    it("renders tagline", () => {
      render(<ChangelogPage />);
      expect(
        screen.getByText("YOUR INFRASTRUCTURE. YOUR RULES."),
      ).toBeInTheDocument();
    });
  });

  describe("Scroll anchors", () => {
    it("creates scroll anchor IDs for each version", () => {
      render(<ChangelogPage />);
      expect(document.getElementById("v0.4.0")).toBeInTheDocument();
      expect(document.getElementById("v0.3.0")).toBeInTheDocument();
      expect(document.getElementById("v0.2.0")).toBeInTheDocument();
      expect(document.getElementById("v0.1.0")).toBeInTheDocument();
    });
  });

  describe("Security filter", () => {
    it("shows security changes when Security filter clicked", () => {
      render(<ChangelogPage />);
      const securityTab = screen.getByRole("tab", { name: /Security/ });
      fireEvent.click(securityTab);
      expect(securityTab).toHaveAttribute("aria-selected", "true");
      // Should show the security change from v0.1.0
      expect(
        screen.getByText(/Ed25519 \+ X25519 cryptographic key management/),
      ).toBeInTheDocument();
    });
  });

  describe("Platform filter", () => {
    it("shows platform changes when Platform filter clicked", () => {
      render(<ChangelogPage />);
      const platformTab = screen.getByRole("tab", { name: /Platform/ });
      fireEvent.click(platformTab);
      expect(platformTab).toHaveAttribute("aria-selected", "true");
      expect(
        screen.getByText(/Desktop release workflow/),
      ).toBeInTheDocument();
    });
  });
});
