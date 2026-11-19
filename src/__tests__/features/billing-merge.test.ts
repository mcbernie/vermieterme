import { describe, expect, it } from "vitest";
import {
  DEFAULT_COST_VALUE,
  costKey,
  mergeCosts,
  mergePrepayments,
  prepaymentKey,
  type CostValues,
} from "@/features/billing/billing-calc";

const value = (totalAmount: number | null, patch = {}) => ({ ...DEFAULT_COST_VALUE, totalAmount, ...patch });
const none: ReadonlySet<string> = new Set();

describe("mergeCosts", () => {
  it("keeps a position that exists on the server only", () => {
    expect(mergeCosts({ c1: value(100) }, {}, none)).toEqual({ c1: value(100) });
  });

  it("takes saved amounts from the server, so changes from another tab appear", () => {
    expect(mergeCosts({ c1: value(200) }, { c1: value(100) }, none)).toEqual({ c1: value(200) });
  });

  it("keeps an unsaved amount but takes the flags from the server", () => {
    const server: CostValues = { c1: value(100, { enabled: false, distributionKeyOverride: "MEA" }) };
    const local: CostValues = { c1: value(150, { unitAmount: 7 }) };
    expect(mergeCosts(server, local, new Set([costKey("c1")]))).toEqual({
      c1: { totalAmount: 150, unitAmount: 7, enabled: false, distributionKeyOverride: "MEA" },
    });
  });

  it("keeps an unsaved new position with default flags", () => {
    const local: CostValues = { c3: value(50, { enabled: false }) };
    expect(mergeCosts({}, local, new Set([costKey("c3")]))).toEqual({ c3: value(50) });
  });

  it("drops a failed hide of a position without server entry, so it is shown again (#61)", () => {
    const local: CostValues = { c3: { ...DEFAULT_COST_VALUE, enabled: false } };
    // No entry on the page means the table shows the default (shown, category key).
    expect(mergeCosts({}, local, none)).toEqual({});
  });

  it("does not take a prepayment key for a cost position", () => {
    expect(mergeCosts({ c1: value(100) }, { c1: value(1) }, new Set([prepaymentKey("c1")]))).toEqual({
      c1: value(100),
    });
  });
});

describe("mergePrepayments", () => {
  it("keeps prepayments that exist on the server only", () => {
    expect(mergePrepayments({ u1: 100 }, {}, none)).toEqual({ u1: 100 });
  });

  it("takes saved amounts from the server", () => {
    expect(mergePrepayments({ u1: 100 }, { u1: 80, u2: 5 }, none)).toEqual({ u1: 100 });
  });

  it("keeps unsaved amounts, also cleared ones and ones without server entry", () => {
    const dirty = new Set([prepaymentKey("u1"), prepaymentKey("u2")]);
    expect(mergePrepayments({ u1: 100 }, { u1: null, u2: 5 }, dirty)).toEqual({ u1: null, u2: 5 });
  });
});
