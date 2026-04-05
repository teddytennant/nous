import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { Sparkline, MiniBarChart } from "@/components/sparkline";

// ── Test data ────────────────────────────────────────────────────────────

const SAMPLE_DATA = [10, 15, 12, 18, 22, 20, 25];
const FLAT_DATA = [5, 5, 5, 5, 5];
const RISING_DATA = [1, 2, 3, 4, 5, 6, 7, 8];

// ── Sparkline tests ──────────────────────────────────────────────────────

describe("Sparkline", () => {
  describe("Rendering", () => {
    it("renders as an SVG element", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      expect(container.querySelector("svg")).toBeInTheDocument();
    });

    it("renders with default dimensions", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("width")).toBe("80");
      expect(svg.getAttribute("height")).toBe("28");
    });

    it("supports custom dimensions", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} width={120} height={40} />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("width")).toBe("120");
      expect(svg.getAttribute("height")).toBe("40");
    });

    it("is aria-hidden", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("aria-hidden")).toBe("true");
    });

    it("has sparkline-enter animation class", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const svg = container.querySelector("svg")!;
      expect(svg.className.baseVal).toContain("sparkline-enter");
    });
  });

  describe("Line rendering", () => {
    it("renders a polyline element", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const polyline = container.querySelector("polyline");
      expect(polyline).toBeInTheDocument();
    });

    it("polyline has points attribute", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const polyline = container.querySelector("polyline")!;
      const points = polyline.getAttribute("points")!;
      // Should have one x,y pair per data point
      const pairs = points.trim().split(" ");
      expect(pairs.length).toBe(SAMPLE_DATA.length);
    });

    it("applies correct stroke width", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} strokeWidth={2.5} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke-width")).toBe("2.5");
    });

    it("has rounded line caps and joins", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke-linecap")).toBe("round");
      expect(polyline.getAttribute("stroke-linejoin")).toBe("round");
    });

    it("has sparkline-line-enter animation class", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.className.baseVal).toContain("sparkline-line-enter");
    });
  });

  describe("Area fill", () => {
    it("renders area path by default", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const path = container.querySelector("path");
      expect(path).toBeInTheDocument();
    });

    it("area path has sparkline-area-enter class", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const path = container.querySelector("path")!;
      expect(path.className.baseVal).toContain("sparkline-area-enter");
    });

    it("does not render area when fillColor is 'none'", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} fillColor="none" />);
      const path = container.querySelector("path");
      expect(path).toBeNull();
    });
  });

  describe("Endpoint dot", () => {
    it("renders endpoint dot by default", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const circle = container.querySelector("circle");
      expect(circle).toBeInTheDocument();
    });

    it("hides endpoint dot when showDot is false", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} showDot={false} />);
      const circle = container.querySelector("circle");
      expect(circle).toBeNull();
    });

    it("dot has sparkline-dot-enter animation class", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const circle = container.querySelector("circle")!;
      expect(circle.className.baseVal).toContain("sparkline-dot-enter");
    });

    it("dot radius is 2", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} />);
      const circle = container.querySelector("circle")!;
      expect(circle.getAttribute("r")).toBe("2");
    });
  });

  describe("Trend colors", () => {
    it("uses emerald stroke for positive trend", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} trend={true} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke")).toContain("52, 211, 153");
    });

    it("uses red stroke for negative trend", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} trend={false} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke")).toContain("239, 68, 68");
    });

    it("uses neutral white for null trend", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} trend={null} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke")).toContain("255, 255, 255");
    });

    it("custom strokeColor overrides trend color", () => {
      const { container } = render(
        <Sparkline data={SAMPLE_DATA} trend={true} strokeColor="red" />,
      );
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("stroke")).toBe("red");
    });
  });

  describe("Edge cases", () => {
    it("returns null for fewer than 2 data points", () => {
      const { container } = render(<Sparkline data={[5]} />);
      expect(container.querySelector("svg")).toBeNull();
    });

    it("returns null for empty data", () => {
      const { container } = render(<Sparkline data={[]} />);
      expect(container.querySelector("svg")).toBeNull();
    });

    it("handles flat data (all same values)", () => {
      const { container } = render(<Sparkline data={FLAT_DATA} />);
      const polyline = container.querySelector("polyline");
      expect(polyline).toBeInTheDocument();
      // Points should still be valid
      const points = polyline!.getAttribute("points")!;
      expect(points.trim().split(" ").length).toBe(FLAT_DATA.length);
    });

    it("handles exactly 2 data points", () => {
      const { container } = render(<Sparkline data={[10, 20]} />);
      const polyline = container.querySelector("polyline")!;
      expect(polyline.getAttribute("points")!.trim().split(" ").length).toBe(2);
    });
  });

  describe("Custom className", () => {
    it("appends custom className", () => {
      const { container } = render(<Sparkline data={SAMPLE_DATA} className="my-spark" />);
      const svg = container.querySelector("svg")!;
      expect(svg.className.baseVal).toContain("my-spark");
      expect(svg.className.baseVal).toContain("sparkline-enter");
    });
  });
});

// ── MiniBarChart tests ───────────────────────────────────────────────────

describe("MiniBarChart", () => {
  describe("Rendering", () => {
    it("renders as an SVG element", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      expect(container.querySelector("svg")).toBeInTheDocument();
    });

    it("renders with default dimensions", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("width")).toBe("80");
      expect(svg.getAttribute("height")).toBe("28");
    });

    it("is aria-hidden", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      const svg = container.querySelector("svg")!;
      expect(svg.getAttribute("aria-hidden")).toBe("true");
    });
  });

  describe("Bar rendering", () => {
    it("renders one rect per data point", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      const rects = container.querySelectorAll("rect");
      expect(rects.length).toBe(SAMPLE_DATA.length);
    });

    it("last bar uses active color", () => {
      const { container } = render(
        <MiniBarChart
          data={SAMPLE_DATA}
          barColor="gray"
          activeBarColor="gold"
        />,
      );
      const rects = container.querySelectorAll("rect");
      const lastRect = rects[rects.length - 1];
      expect(lastRect.getAttribute("fill")).toBe("gold");
    });

    it("non-last bars use default color", () => {
      const { container } = render(
        <MiniBarChart
          data={SAMPLE_DATA}
          barColor="gray"
          activeBarColor="gold"
        />,
      );
      const rects = container.querySelectorAll("rect");
      expect(rects[0].getAttribute("fill")).toBe("gray");
      expect(rects[3].getAttribute("fill")).toBe("gray");
    });

    it("bars have staggered animation delays", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      const rects = container.querySelectorAll("rect");
      for (let i = 0; i < rects.length; i++) {
        const style = rects[i].getAttribute("style")!;
        expect(style).toContain(`${i * 30}ms`);
      }
    });

    it("bars have sparkline-bar-enter animation class", () => {
      const { container } = render(<MiniBarChart data={SAMPLE_DATA} />);
      const rects = container.querySelectorAll("rect");
      for (const rect of rects) {
        expect(rect.className.baseVal).toContain("sparkline-bar-enter");
      }
    });

    it("tallest bar reaches full height", () => {
      const { container } = render(<MiniBarChart data={[0, 10, 5]} height={28} />);
      const rects = container.querySelectorAll("rect");
      // The tallest bar (value 10) should have the largest height
      const heights = Array.from(rects).map((r) => parseFloat(r.getAttribute("height")!));
      const maxIdx = heights.indexOf(Math.max(...heights));
      expect(maxIdx).toBe(1); // index 1 has value 10
    });
  });

  describe("Edge cases", () => {
    it("returns null for empty data", () => {
      const { container } = render(<MiniBarChart data={[]} />);
      expect(container.querySelector("svg")).toBeNull();
    });

    it("handles single data point", () => {
      const { container } = render(<MiniBarChart data={[5]} />);
      const rects = container.querySelectorAll("rect");
      expect(rects.length).toBe(1);
    });

    it("handles all-zero data", () => {
      const { container } = render(<MiniBarChart data={[0, 0, 0]} />);
      const rects = container.querySelectorAll("rect");
      expect(rects.length).toBe(3);
      // Heights should be minimal (1px minimum)
      for (const rect of rects) {
        expect(parseFloat(rect.getAttribute("height")!)).toBeGreaterThanOrEqual(1);
      }
    });
  });
});
