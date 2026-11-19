import { describe, expect, it } from "vitest";
import { calculateMEAAmount, getMonthsInPeriod, suggestNextPrepayment } from "@/lib/billing";
import {
  costPayload,
  isCostUnreviewed,
  nkRecommendations,
  reviewProgress,
  seedValues,
  totalsOf,
  unitResults,
} from "@/features/billing/billing-calc";
import type { BillingPeriodDetail, CostCategory, UnitWithTenants } from "@/types";

const unit = (id: string, shares: number, tenant = true): UnitWithTenants => ({
  id,
  propertyId: "p",
  name: `W${id}`,
  floor: "EG",
  shares,
  tenants: tenant
    ? [{ id: `t${id}`, firstName: "Max", lastName: id, moveOutDate: null } as UnitWithTenants["tenants"][number]]
    : [],
});

const categories: CostCategory[] = [
  { id: "c1", name: "Grundsteuer", distributionKey: "MEA", sortOrder: 1 },
  { id: "c2", name: "Wasser", distributionKey: "laut Bescheid", sortOrder: 2 },
  { id: "c3", name: "Müll", distributionKey: "siehe Anlage", sortOrder: 3 },
  { id: "c4", name: "Strom", distributionKey: "MEA", sortOrder: 4 },
  { id: "c5", name: "Leer", distributionKey: "MEA", sortOrder: 5 },
];

const cost = (id: string, totalAmount: number, unitAmount: number | null, extra: Partial<BillingPeriodDetail["costs"][number]> = {}) => ({
  id: `k${id}`,
  billingPeriodId: "bp",
  costCategoryId: id,
  totalAmount,
  unitAmount,
  reviewed: true,
  enabled: true,
  distributionKeyOverride: null,
  ...extra,
});

const period: BillingPeriodDetail = {
  id: "bp",
  propertyId: "p",
  startDate: "2025-01-01T00:00:00.000Z",
  endDate: "2025-12-31T00:00:00.000Z",
  billingDate: null,
  sentDate: null,
  paidDate: null,
  copiedFromId: null,
  property: { id: "p", street: "Weg 1", zip: "1", city: "C", totalShares: 1000, units: [unit("1", 333), unit("2", 667, false)] },
  costs: [
    cost("c1", 1234.567, null),
    cost("c2", 500, 210.3, { reviewed: false }),
    cost("c3", 0, 0),
    cost("c4", 99.99, 12, { distributionKeyOverride: "laut Bescheid" }),
    cost("c5", 80, null, { enabled: false, reviewed: false }),
  ],
  prepayments: [
    { id: "pp1", billingPeriodId: "bp", unitId: "1", monthlyAmount: 85.5, reviewed: true },
    { id: "pp2", billingPeriodId: "bp", unitId: "2", monthlyAmount: 0, reviewed: false },
  ],
};

/** Verbatim port of the former page calculation on string form values, used as reference. */
function legacyUnitCosts(u: UnitWithTenants, bp: BillingPeriodDetail): number {
  const values: Record<string, { totalAmount: string; unitAmount: string; enabled: boolean; distributionKeyOverride: string | null }> = {};
  bp.costs.forEach((c) => {
    values[c.costCategoryId] = {
      totalAmount: c.totalAmount.toString(),
      unitAmount: c.unitAmount?.toString() || "",
      enabled: c.enabled !== false,
      distributionKeyOverride: c.distributionKeyOverride ?? null,
    };
  });
  let total = 0;
  categories.forEach((cat) => {
    const v = values[cat.id];
    if (!v || !v.totalAmount) return;
    if (v.enabled === false) return;
    const totalAmount = parseFloat(v.totalAmount) || 0;
    const key = v.distributionKeyOverride ?? cat.distributionKey;
    if (key === "MEA") total += calculateMEAAmount(totalAmount, u.shares, bp.property.totalShares);
    else total += v.unitAmount ? parseFloat(v.unitAmount) : 0;
  });
  return total;
}

describe("billing-calc", () => {
  const seeded = seedValues(period);
  const results = unitResults(period, categories, seeded.costs, seeded.prepayments);

  it("computes unit costs exactly like the former page", () => {
    for (const r of results) expect(r.costs).toBe(legacyUnitCosts(r.unit, period));
  });

  it("computes prepayments, results and totals per unit", () => {
    const months = getMonthsInPeriod(period.startDate, period.endDate);
    expect(results[0]?.prepayment).toBe(85.5 * months);
    expect(results[0]?.result).toBe((results[0]?.costs ?? 0) - 85.5 * months);
    const totals = totalsOf(results);
    expect(totals.costs).toBe((results[0]?.costs ?? 0) + (results[1]?.costs ?? 0));
    expect(totals.prepayments).toBe(85.5 * months);
  });

  it("recommends a new prepayment only for units with a back payment", () => {
    const recs = nkRecommendations(results);
    for (const r of results) {
      const expected = suggestNextPrepayment(r.monthly, r.result);
      const rec = recs.find((x) => x.unit.id === r.unit.id);
      expect(rec?.suggested ?? null).toBe(expected);
    }
  });

  it("skips categories without total amount and disabled ones", () => {
    const costs = { ...seeded.costs, c1: { ...seeded.costs.c1!, totalAmount: null } };
    const without = unitResults(period, categories, costs, seeded.prepayments);
    expect(without[0]?.costs).toBeCloseTo((results[0]?.costs ?? 0) - calculateMEAAmount(1234.567, 333, 1000), 10);
  });

  it("counts review progress without disabled positions", () => {
    expect(reviewProgress(period)).toEqual({ total: 6, reviewed: 4, allReviewed: false });
    expect(isCostUnreviewed(period, "c2")).toBe(true);
    expect(isCostUnreviewed(period, "missing")).toBe(false);
  });

  it("keeps a zero unit amount in the save payload and sends null for an empty one", () => {
    expect(costPayload("c3", seeded.costs.c3!)).toEqual({ costCategoryId: "c3", totalAmount: 0, unitAmount: 0 });
    expect(costPayload("c1", seeded.costs.c1!)).toEqual({ costCategoryId: "c1", totalAmount: 1234.567, unitAmount: null });
  });
});
