import { describe, it, expect } from "vitest";
import { cn } from "@/lib/utils";

describe("cn (class name utility)", () => {
  it("merges simple class names", () => {
    expect(cn("text-sm", "font-bold")).toBe("text-sm font-bold");
  });

  it("handles conditional classes", () => {
    const isActive = true;
    const isDisabled = false;
    expect(cn("base", isActive && "active", isDisabled && "disabled")).toBe(
      "base active",
    );
  });

  it("resolves tailwind conflicts — last wins", () => {
    const result = cn("text-red-500", "text-blue-500");
    expect(result).toBe("text-blue-500");
  });

  it("resolves padding conflicts", () => {
    const result = cn("p-4", "p-6");
    expect(result).toBe("p-6");
  });

  it("handles undefined and null inputs", () => {
    expect(cn("base", undefined, null, "extra")).toBe("base extra");
  });

  it("handles empty string input", () => {
    expect(cn("", "text-sm")).toBe("text-sm");
  });

  it("handles array inputs", () => {
    expect(cn(["text-sm", "font-bold"])).toBe("text-sm font-bold");
  });

  it("handles object inputs", () => {
    expect(cn({ "text-sm": true, "font-bold": false, "mt-4": true })).toBe(
      "text-sm mt-4",
    );
  });

  it("returns empty string for no inputs", () => {
    expect(cn()).toBe("");
  });

  it("deduplicates identical classes", () => {
    expect(cn("text-sm", "text-sm")).toBe("text-sm");
  });

  it("handles complex border + bg conflict resolution", () => {
    const result = cn(
      "bg-white/[0.04]",
      "border-white/[0.06]",
      "bg-[#d4af37]/[0.02]",
    );
    expect(result).toContain("bg-[#d4af37]/[0.02]");
    expect(result).toContain("border-white/[0.06]");
    expect(result).not.toContain("bg-white/[0.04]");
  });
});
