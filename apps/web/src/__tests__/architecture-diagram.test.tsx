import { describe, it, expect } from "vitest";
import { render, fireEvent } from "@testing-library/react";
import { ArchitectureDiagram } from "@/components/architecture-diagram";

// ── Constants (mirrored from component for assertions) ──────────────────
const CX = 360;
const CY = 280;
const RADIUS = 190;
const NODE_R = 38;

const SUBSYSTEM_NAMES = [
  "Identity",
  "Messaging",
  "Governance",
  "Payments",
  "Social",
  "Storage",
  "AI",
  "Browser",
];

const TECH_TAGS = [
  "DID:key",
  "E2EE",
  "Quadratic",
  "Multi-chain",
  "Nostr",
  "CRDTs",
  "Local LLM",
  "IPFS",
];

// ── SVG Structure ───────────────────────────────────────────────────────

describe("ArchitectureDiagram", () => {
  describe("SVG structure", () => {
    it("renders an SVG element", () => {
      const { container } = render(<ArchitectureDiagram />);
      expect(container.querySelector("svg")).toBeInTheDocument();
    });

    it("SVG has role='img'", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("role")).toBe("img");
    });

    it("SVG has aria-label containing '8 subsystems'", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("aria-label")).toContain("8 subsystems");
    });

    it("SVG has viewBox '0 0 720 560'", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("viewBox")).toBe("0 0 720 560");
    });

    it("SVG has the arch-diagram class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      expect(svg.className.baseVal).toContain("arch-diagram");
    });
  });

  // ── Center Node ─────────────────────────────────────────────────────────

  describe("Center node", () => {
    it("renders 'Nous' text", () => {
      const { container } = render(<ArchitectureDiagram />);
      const texts = container.querySelectorAll("text");
      const nousText = Array.from(texts).find(
        (t) => t.textContent === "Nous",
      );
      expect(nousText).toBeDefined();
    });

    it("renders 'CORE' text", () => {
      const { container } = render(<ArchitectureDiagram />);
      const texts = container.querySelectorAll("text");
      const coreText = Array.from(texts).find(
        (t) => t.textContent === "CORE",
      );
      expect(coreText).toBeDefined();
    });

    it("center circle exists at cx=360, cy=280", () => {
      const { container } = render(<ArchitectureDiagram />);
      const circles = container.querySelectorAll("circle");
      const center = Array.from(circles).find(
        (c) =>
          c.getAttribute("cx") === String(CX) &&
          c.getAttribute("cy") === String(CY) &&
          c.getAttribute("r") === "44",
      );
      expect(center).toBeDefined();
    });

    it("inner accent ring exists", () => {
      const { container } = render(<ArchitectureDiagram />);
      const circles = container.querySelectorAll("circle");
      const ring = Array.from(circles).find(
        (c) =>
          c.getAttribute("cx") === String(CX) &&
          c.getAttribute("cy") === String(CY) &&
          c.getAttribute("r") === String(NODE_R) &&
          c.getAttribute("fill") === "none",
      );
      expect(ring).toBeDefined();
    });
  });

  // ── Subsystem Nodes ─────────────────────────────────────────────────────

  describe("Subsystem nodes", () => {
    it("renders all 8 subsystem names", () => {
      const { container } = render(<ArchitectureDiagram />);
      const texts = Array.from(container.querySelectorAll("text")).map(
        (t) => t.textContent,
      );
      for (const name of SUBSYSTEM_NAMES) {
        expect(texts).toContain(name);
      }
    });

    it("renders all 8 tech tags", () => {
      const { container } = render(<ArchitectureDiagram />);
      const texts = Array.from(container.querySelectorAll("text")).map(
        (t) => t.textContent,
      );
      for (const tag of TECH_TAGS) {
        expect(texts).toContain(tag);
      }
    });

    it("each node group has arch-node class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");
      expect(nodes.length).toBe(8);
    });

    it("each node has data-delay attribute (0 through 7)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");
      const delays = Array.from(nodes).map((n) => n.getAttribute("data-delay"));
      for (let i = 0; i < 8; i++) {
        expect(delays).toContain(String(i));
      }
    });
  });

  // ── Connections ─────────────────────────────────────────────────────────

  describe("Connections", () => {
    it("renders 8 primary connection lines (center to each node)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const lines = container.querySelectorAll("line.arch-line");
      expect(lines.length).toBe(8);
    });

    it("primary lines have arch-line class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const lines = container.querySelectorAll("line.arch-line");
      for (const line of lines) {
        expect(line.className.baseVal).toContain("arch-line");
      }
    });

    it("primary lines originate from center (CX, CY)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const lines = container.querySelectorAll("line.arch-line");
      for (const line of lines) {
        expect(line.getAttribute("x1")).toBe(String(CX));
        expect(line.getAttribute("y1")).toBe(String(CY));
      }
    });

    it("renders 6 secondary connection lines (dashed)", () => {
      const { container } = render(<ArchitectureDiagram />);
      // All lines in the SVG
      const allLines = container.querySelectorAll("svg line");
      // Secondary lines have strokeDasharray="3 6" and are not .arch-line
      const secondaryLines = Array.from(allLines).filter(
        (l) =>
          l.getAttribute("stroke-dasharray") === "3 6" &&
          !l.className.baseVal.includes("arch-line"),
      );
      expect(secondaryLines.length).toBe(6);
    });

    it("secondary lines have strokeDasharray", () => {
      const { container } = render(<ArchitectureDiagram />);
      const allLines = container.querySelectorAll("svg line");
      const secondaryLines = Array.from(allLines).filter(
        (l) =>
          l.getAttribute("stroke-dasharray") === "3 6" &&
          !l.className.baseVal.includes("arch-line"),
      );
      for (const line of secondaryLines) {
        expect(line.getAttribute("stroke-dasharray")).toBe("3 6");
      }
    });
  });

  // ── Animations ──────────────────────────────────────────────────────────

  describe("Animations", () => {
    it("orbit ring exists with arch-orbit class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const orbit = container.querySelector(".arch-orbit");
      expect(orbit).toBeInTheDocument();
      expect(orbit!.tagName.toLowerCase()).toBe("circle");
      expect(orbit!.getAttribute("r")).toBe(String(RADIUS));
    });

    it("3 pulse ring circles exist (with animate elements)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const animates = container.querySelectorAll("svg > circle > animate");
      // Each pulse ring has 2 animate elements (r and opacity), so 3 rings = 6 animate elements
      expect(animates.length).toBe(6);
      // Count unique parent circles
      const parents = new Set(Array.from(animates).map((a) => a.parentElement));
      expect(parents.size).toBe(3);
    });

    it("particles group has arch-particles class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const particles = container.querySelector(".arch-particles");
      expect(particles).toBeInTheDocument();
      expect(particles!.tagName.toLowerCase()).toBe("g");
    });

    it("8 outward particle circles (gold fill, r=2)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const particlesGroup = container.querySelector(".arch-particles")!;
      const circles = particlesGroup.querySelectorAll("circle");
      const goldCircles = Array.from(circles).filter(
        (c) =>
          c.getAttribute("r") === "2" && c.getAttribute("fill") === "#d4af37",
      );
      expect(goldCircles.length).toBe(8);
    });

    it("8 inward particle circles (white fill, r=1.5)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const particlesGroup = container.querySelector(".arch-particles")!;
      const circles = particlesGroup.querySelectorAll("circle");
      const whiteCircles = Array.from(circles).filter(
        (c) =>
          c.getAttribute("r") === "1.5" && c.getAttribute("fill") === "white",
      );
      expect(whiteCircles.length).toBe(8);
    });

    it("6 secondary particle circles", () => {
      const { container } = render(<ArchitectureDiagram />);
      const particlesGroup = container.querySelector(".arch-particles")!;
      const circles = particlesGroup.querySelectorAll("circle");
      // Secondary particles: r=1.5, fill=#d4af37, opacity=0.12
      const secondaryParticles = Array.from(circles).filter(
        (c) =>
          c.getAttribute("r") === "1.5" &&
          c.getAttribute("fill") === "#d4af37" &&
          c.getAttribute("opacity") === "0.12",
      );
      expect(secondaryParticles.length).toBe(6);
    });
  });

  // ── Hover Interaction ───────────────────────────────────────────────────

  describe("Hover interaction", () => {
    it("hovering a subsystem node changes its fill to gold tinted", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");
      const firstNode = nodes[0];

      // Before hover: node circle has default fill
      const nodeCircle = firstNode.querySelectorAll("circle")[0];
      expect(nodeCircle.getAttribute("fill")).toBe("rgba(255,255,255,0.015)");

      fireEvent.mouseEnter(firstNode);

      // After hover: fill changes to gold tinted
      const updatedCircle = container
        .querySelectorAll(".arch-node")[0]
        .querySelectorAll("circle");
      // The first circle in the node (after glow ring) or the node circle itself
      const mainCircle = Array.from(updatedCircle).find(
        (c) => c.getAttribute("r") === String(NODE_R),
      );
      expect(mainCircle!.getAttribute("fill")).toBe("rgba(212,175,55,0.05)");
    });

    it("hovering a subsystem node changes its stroke to gold", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");
      fireEvent.mouseEnter(nodes[0]);

      const updatedCircles = container
        .querySelectorAll(".arch-node")[0]
        .querySelectorAll("circle");
      const mainCircle = Array.from(updatedCircles).find(
        (c) => c.getAttribute("r") === String(NODE_R),
      );
      expect(mainCircle!.getAttribute("stroke")).toBe("rgba(212,175,55,0.4)");
    });

    it("hovering a subsystem changes the primary connection line stroke", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");
      fireEvent.mouseEnter(nodes[0]);

      const lines = container.querySelectorAll("line.arch-line");
      // First line (index 0) should be highlighted
      expect(lines[0].getAttribute("stroke")).toBe("rgba(212,175,55,0.45)");
      // Other lines should remain default
      expect(lines[1].getAttribute("stroke")).toBe("rgba(255,255,255,0.05)");
    });

    it("hovering a subsystem shows hover glow ring (radius NODE_R + 12)", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");

      // Before hover: no glow ring
      const glowRingBefore = Array.from(
        nodes[0].querySelectorAll("circle"),
      ).find((c) => c.getAttribute("r") === String(NODE_R + 12));
      expect(glowRingBefore).toBeUndefined();

      fireEvent.mouseEnter(nodes[0]);

      // After hover: glow ring appears
      const updatedNode = container.querySelectorAll(".arch-node")[0];
      const glowRing = Array.from(updatedNode.querySelectorAll("circle")).find(
        (c) => c.getAttribute("r") === String(NODE_R + 12),
      );
      expect(glowRing).toBeDefined();
      expect(glowRing!.getAttribute("fill")).toBe("url(#arch-node-glow)");
    });

    it("un-hovering restores default appearance", () => {
      const { container } = render(<ArchitectureDiagram />);
      const nodes = container.querySelectorAll(".arch-node");

      fireEvent.mouseEnter(nodes[0]);
      fireEvent.mouseLeave(nodes[0]);

      // Node circle fill should be back to default
      const nodeCircle = container
        .querySelectorAll(".arch-node")[0]
        .querySelectorAll("circle")[0];
      expect(nodeCircle.getAttribute("fill")).toBe("rgba(255,255,255,0.015)");

      // Primary line stroke should be back to default
      const lines = container.querySelectorAll("line.arch-line");
      expect(lines[0].getAttribute("stroke")).toBe("rgba(255,255,255,0.05)");

      // No glow ring
      const glowRing = Array.from(
        container
          .querySelectorAll(".arch-node")[0]
          .querySelectorAll("circle"),
      ).find((c) => c.getAttribute("r") === String(NODE_R + 12));
      expect(glowRing).toBeUndefined();
    });
  });

  // ── Gradients / Defs ────────────────────────────────────────────────────

  describe("Gradients and defs", () => {
    it("has radialGradient with id 'arch-core-glow'", () => {
      const { container } = render(<ArchitectureDiagram />);
      const gradient = container.querySelector("#arch-core-glow");
      expect(gradient).toBeInTheDocument();
      expect(gradient!.tagName.toLowerCase()).toBe("radialgradient");
    });

    it("has radialGradient with id 'arch-node-glow'", () => {
      const { container } = render(<ArchitectureDiagram />);
      const gradient = container.querySelector("#arch-node-glow");
      expect(gradient).toBeInTheDocument();
      expect(gradient!.tagName.toLowerCase()).toBe("radialgradient");
    });
  });

  // ── Mobile View ─────────────────────────────────────────────────────────

  describe("Mobile view", () => {
    it("renders a grid of 8 mobile cards", () => {
      const { container } = render(<ArchitectureDiagram />);
      const grid = container.querySelector(".sm\\:hidden");
      expect(grid).toBeInTheDocument();
      const cards = grid!.children;
      expect(cards.length).toBe(8);
    });

    it("grid has sm:hidden class", () => {
      const { container } = render(<ArchitectureDiagram />);
      const grid = container.querySelector("div.grid");
      expect(grid).toBeInTheDocument();
      expect(grid!.className).toContain("sm:hidden");
    });

    it("each card shows subsystem name and tag", () => {
      const { container } = render(<ArchitectureDiagram />);
      const grid = container.querySelector(".sm\\:hidden")!;
      const cards = Array.from(grid.children);

      for (let i = 0; i < 8; i++) {
        const card = cards[i];
        expect(card.textContent).toContain(SUBSYSTEM_NAMES[i]);
        expect(card.textContent).toContain(TECH_TAGS[i]);
      }
    });

    it("each card has a gold dot indicator", () => {
      const { container } = render(<ArchitectureDiagram />);
      const grid = container.querySelector(".sm\\:hidden")!;
      const cards = Array.from(grid.children);

      for (const card of cards) {
        const dot = card.querySelector(".rounded-full");
        expect(dot).toBeDefined();
        expect(dot!.className).toContain("bg-[#d4af37]/30");
      }
    });
  });

  // ── Accessibility ───────────────────────────────────────────────────────

  describe("Accessibility", () => {
    it("SVG has role='img' for screen readers", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("role")).toBe("img");
    });

    it("aria-label describes the diagram content", () => {
      const { container } = render(<ArchitectureDiagram />);
      const svg = container.querySelector("svg")!;
      const label = svg.getAttribute("aria-label")!;
      expect(label).toContain("Nous");
      expect(label).toContain("architecture");
      expect(label).toContain("8 subsystems");
    });
  });
});
