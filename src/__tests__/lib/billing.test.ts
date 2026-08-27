import { describe, expect, it } from "vitest";
import { calculateAllocationAmount, calculateMEAAmount } from "@/lib/billing";

describe("billing calculations", () => {
  it("calculates MEA amounts", () => {
    expect(calculateMEAAmount(5700.92, 143, 10000)).toBeCloseTo(81.52, 2);
  });

  it("calculates custom allocation key amounts", () => {
    expect(calculateAllocationAmount(5700.92, 29.155, 1824.378)).toBeCloseTo(
      91.11,
      2
    );
  });

  it("returns 0 when the allocation denominator is 0", () => {
    expect(calculateAllocationAmount(5700.92, 29.155, 0)).toBe(0);
  });
});
