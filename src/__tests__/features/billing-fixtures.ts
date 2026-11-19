import type { BillingPeriodDetail, CostCategory } from "@/types";

/** Two cost categories: one by MEA, one by assessment notice. */
export const categories: CostCategory[] = [
  { id: "c1", name: "Grundsteuer", distributionKey: "MEA", sortOrder: 1 },
  { id: "c2", name: "Wasser", distributionKey: "laut Bescheid", sortOrder: 2 },
];

/** A billing period with one unit; the water cost is still unreviewed. */
export function detail(): BillingPeriodDetail {
  return {
    id: "bp",
    propertyId: "p",
    startDate: "2025-01-01T00:00:00.000Z",
    endDate: "2025-12-31T00:00:00.000Z",
    billingDate: null,
    sentDate: null,
    paidDate: null,
    copiedFromId: null,
    property: {
      id: "p",
      street: "Weg 1",
      zip: "28195",
      city: "Bremen",
      totalShares: 100,
      units: [{ id: "u1", propertyId: "p", name: "EG", floor: "EG", shares: 100, tenants: [] }],
    },
    costs: [
      { id: "k1", billingPeriodId: "bp", costCategoryId: "c1", totalAmount: 1200, unitAmount: null, reviewed: true, enabled: true, distributionKeyOverride: null },
      { id: "k2", billingPeriodId: "bp", costCategoryId: "c2", totalAmount: 300, unitAmount: 300, reviewed: false, enabled: true, distributionKeyOverride: null },
    ],
    prepayments: [{ id: "pp1", billingPeriodId: "bp", unitId: "u1", monthlyAmount: 100, reviewed: true }],
  };
}

/** Minimal `Response` with a JSON body. */
export const json = (data: unknown, status = 200) =>
  ({ ok: status < 400, status, json: async () => data }) as Response;
